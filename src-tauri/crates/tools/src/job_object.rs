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
        /// 创建一个新的 Job Object，并关联到子进程。
        /// 设置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 标志。
        pub fn new(child: &Child) -> Result<Self, String> {
            use windows_sys::Win32::Foundation::CloseHandle;
            use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
            use windows_sys::Win32::System::JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, SetInformationJobObject,
            };

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
                unsafe {
                    CloseHandle(handle);
                }
                return Err("SetInformationJobObject 失败".to_string());
            }

            // raw_handle() 返回 Option<*mut c_void>
            let raw_handle = child.raw_handle().ok_or_else(|| "子进程句柄为空".to_string())?;
            let assign_ret = unsafe { AssignProcessToJobObject(handle, raw_handle) };

            if assign_ret == 0 {
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
    }

    impl Drop for JobObject {
        fn drop(&mut self) {
            // 检查句柄是否已被释放，防止 double-free
            if self.released.swap(true, std::sync::atomic::Ordering::AcqRel) {
                return;
            }
            // 关闭 Job 句柄触发 JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
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
        Ok(JobHandle { inner: Some(std::sync::Arc::new(job)) })
    }
    #[cfg(not(windows))]
    {
        Ok(JobHandle { inner: None })
    }
}

/// Job Object 句柄——持有它直到子进程执行完毕，Drop 时自动清理进程树。
pub struct JobHandle {
    /// 仅用于 RAII：保持 Arc 引用直到 JobHandle drop，
    /// 此时 Arc<JobObject> 引用计数归零，JobObject 的 Drop impl 关闭句柄，
    /// 触发 JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE 终止整个进程树。
    #[cfg(windows)]
    #[allow(dead_code)]
    inner: Option<std::sync::Arc<windows_impl::JobObject>>,
    #[cfg(not(windows))]
    #[allow(dead_code)]
    inner: Option<()>,
}
