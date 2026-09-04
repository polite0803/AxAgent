// SPDX-License-Identifier: AGPL-3.0-only

//! Windows Job Objects 封装 —— 将子进程关联到 Job Object，
//! 确保进程树被整体清理（防止 `kill_on_drop` 残留孤儿子进程）。
//!
//! 仅在 Windows 上编译和使用。非 Windows 平台提供空实现。
//!
//! 创建一个 Job Object，将传入进程关联到该 Job，
//! 设置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` —— 当 Job 的最后一个句柄关闭时，
//! 所有关联进程（包括进程树中后续创建的孙子进程）均被终止。
#[cfg(windows)]
pub mod windows_impl {
    use tokio::process::Child;

    /// 持有 Job Object 句柄，Drop 时自动清理
    pub struct JobObject {
        handle: std::ptr::NonNull<std::ffi::c_void>,
        released: std::sync::atomic::AtomicBool,
    }

    // Job Object 句柄可以跨线程安全使用
    unsafe impl Send for JobObject {}
    unsafe impl Sync for JobObject {}

    impl JobObject {
        /// 创建 Job Object 并关联到给定进程原始句柄。
        /// 设置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 标志。
        ///
        /// # Safety
        /// 调用方必须保证 `process_handle` 是有效的进程句柄（如 CreateProcess
        /// 返回的 hProcess），且在本次调用期间未被关闭。
        pub unsafe fn new_raw(
            process_handle: windows_sys::Win32::Foundation::HANDLE,
        ) -> Result<Self, String> {
            use windows_sys::Win32::Foundation::CloseHandle;
            use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
            use windows_sys::Win32::System::JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, SetInformationJobObject,
            };

            // SAFETY: CreateJobObjectW 是 Win32 API，两个参数都传 null 表示
            // 使用默认安全描述符和匿名 Job Object；null 返回时下方立即检查。
            let handle = unsafe {
                CreateJobObjectW(std::ptr::null::<SECURITY_ATTRIBUTES>(), std::ptr::null())
            };

            if handle.is_null() {
                return Err("CreateJobObjectW 失败".to_string());
            }

            // JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: 关闭 Job 句柄时终止所有进程
            // 使用 zeroed() 初始化整个结构体，然后只设置 LimitFlags
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            let ret = unsafe {
                SetInformationJobObject(
                    handle,
                    windows_sys::Win32::System::JobObjects::JobObjectExtendedLimitInformation,
                    &info as *const _ as *const std::ffi::c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            };

            if ret == 0 {
                // SAFETY: handle 为有效的 Job Object 句柄，CloseHandle 释放后不再使用。
                unsafe {
                    CloseHandle(handle);
                }
                return Err("SetInformationJobObject 失败".to_string());
            }

            // SAFETY: handle 为 Job Object 句柄（已 null-check）；process_handle 由调用方
            // 保证为有效进程句柄；失败时 assign_ret==0 下方处理。
            let assign_ret = unsafe { AssignProcessToJobObject(handle, process_handle) };

            if assign_ret == 0 {
                // SAFETY: handle 有效；此错误路径关闭句柄，进程未关联到 Job，
                // 不会触发 KILL_ON_JOB_CLOSE。
                unsafe {
                    CloseHandle(handle);
                }
                return Err("AssignProcessToJobObject 失败".to_string());
            }

            // 包装为 NonNull 以便安全处理
            Ok(Self {
                handle: std::ptr::NonNull::new(handle)
                    .ok_or_else(|| "Invalid Job Object handle".to_string())?,
                released: std::sync::atomic::AtomicBool::new(false),
            })
        }

        /// 创建一个新的 Job Object，并关联到子进程。
        /// 设置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 标志。
        pub fn new(child: &Child) -> Result<Self, String> {
            let raw_handle = child.raw_handle().ok_or_else(|| "子进程句柄为空".to_string())?;
            // SAFETY: tokio Child 的 raw_handle 是有效的进程句柄（进程存活期间）。
            unsafe { Self::new_raw(raw_handle as _) }
        }
    }

    impl Drop for JobObject {
        fn drop(&mut self) {
            // 检查句柄是否已被释放，防止 double-free
            if self.released.swap(true, std::sync::atomic::Ordering::AcqRel) {
                return;
            }
            // SAFETY: self.handle 为有效 Job Object 句柄（NonNull 保证非 null）；
            // AtomicBool 确保只关闭一次防 double-free；
            // 关闭触发 JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE，终止所有关联进程。
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.handle.as_ptr());
            }
        }
    }
}

/// 将子进程关联到 Job Object 中——确保进程树（包括孙子进程）被整体清理。
///
/// - Windows: 创建 Job Object 并设置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`，
///   返回的 JobHandle 在 Drop 时关闭句柄，自动终止整个进程树。
/// - 非 Windows: 空操作，返回的 JobHandle 不做任何事。
#[cfg_attr(not(windows), allow(unused_variables))]
pub fn assign_job(child: &tokio::process::Child) -> Result<JobHandle, String> {
    #[cfg(windows)]
    {
        let job = windows_impl::JobObject::new(child)?;
        Ok(JobHandle { _inner: Some(std::sync::Arc::new(job)) })
    }
    #[cfg(not(windows))]
    {
        Ok(JobHandle { _inner: None })
    }
}

/// 将进程原始句柄关联到 Job Object（供 CreateProcessAsUserW 等手动创建
/// 的进程使用，见 `win_sandbox`）。非 Windows 平台空操作。
///
/// # Safety
/// 调用方必须保证 `process_handle` 是有效的进程句柄，且在本次调用期间未被关闭。
#[cfg(windows)]
pub unsafe fn assign_job_raw(
    process_handle: windows_sys::Win32::Foundation::HANDLE,
) -> Result<JobHandle, String> {
    // SAFETY: 调用方保证 process_handle 有效（unsafe 契约见函数文档）。
    let job = unsafe { windows_impl::JobObject::new_raw(process_handle)? };
    Ok(JobHandle { _inner: Some(std::sync::Arc::new(job)) })
}

/// 非 Windows 平台空操作——直接返回 `JobHandle { _inner: None }`。
///
/// # Safety
/// 本 stub 不执行任何 unsafe 操作，传入参数直接被忽略。
#[cfg(not(windows))]
pub unsafe fn assign_job_raw(_process_handle: isize) -> Result<JobHandle, String> {
    Ok(JobHandle { _inner: None })
}

/// Job Object 句柄——持有它直到子进程执行完毕，Drop 时自动清理进程树。
pub struct JobHandle {
    /// 仅用于 RAII：保持 Arc 引用直到 JobHandle drop，
    /// 此时 Arc<JobObject> 引用计数归零，JobObject 的 Drop impl 关闭句柄，
    /// 触发 JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE 终止整个进程树。
    #[cfg(windows)]
    _inner: Option<std::sync::Arc<windows_impl::JobObject>>,
    #[cfg(not(windows))]
    _inner: Option<()>,
}
