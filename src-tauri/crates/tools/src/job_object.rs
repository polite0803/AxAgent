// SPDX-License-Identifier: AGPL-3.0-only

//! Windows Job Objects 封装 —— 将子进程关联到 Job Object，
//! 确保进程树被整体清理（防止 `kill_on_drop` 残留孤儿子进程）。
//!
//! 仅在 Windows 上编译和使用。非 Windows 平台提供空实现。

/// 创建一个 Job Object，将传入进程关联到该 Job，
/// 设置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` —— 当 Job 的最后一个句柄关闭时，
/// 所有关联进程（包括进程树中后续创建的孙子进程）均被终止。

#[cfg(windows)]
pub mod windows_impl {
    use tokio::process::Child;

    /// 持有 Job Object 句柄，Drop 时自动清理
    pub struct JobObject {
        handle: std::ptr::NonNull<std::ffi::c_void>,
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
                AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            };

            let handle = unsafe {
                CreateJobObjectW(
                    std::ptr::null::<SECURITY_ATTRIBUTES>(),
                    std::ptr::null(),
                )
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
                unsafe { CloseHandle(handle); }
                return Err("SetInformationJobObject 失败".to_string());
            }

            // raw_handle() 返回 Option<*mut c_void>
            let raw_handle = child
                .raw_handle()
                .ok_or_else(|| "子进程句柄为空".to_string())?;
            let assign_ret = unsafe { AssignProcessToJobObject(handle, raw_handle) };

            if assign_ret == 0 {
                unsafe { CloseHandle(handle); }
                return Err("AssignProcessToJobObject 失败".to_string());
            }

            // 包装为 NonNull 以便安全处理
            Ok(Self {
                handle: std::ptr::NonNull::new(handle)
                    .ok_or_else(|| "Invalid Job Object handle".to_string())?,
            })
        }
    }

    impl Drop for JobObject {
        fn drop(&mut self) {
            // 关闭 Job 句柄触发 JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.handle.as_ptr());
            }
        }
    }
}

/// 将子进程关联到 Job Object 中——仅在 Windows 上有效
#[cfg(windows)]
pub fn assign_job(child: &tokio::process::Child) -> Result<std::sync::Arc<windows_impl::JobObject>, String> {
    let job = windows_impl::JobObject::new(child)?;
    Ok(std::sync::Arc::new(job))
}

/// 非 Windows 平台的空实现：直接返回 None
#[cfg(not(windows))]
pub fn assign_job(_child: &tokio::process::Child) -> Result<Option<std::sync::Arc<()>>, String> {
    Ok(None)
}
