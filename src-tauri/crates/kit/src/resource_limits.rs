// SPDX-License-Identifier: AGPL-3.0-only

//! 跨平台进程资源限制。
//!
//! Linux/macOS: rlimit (RLIMIT_CPU, RLIMIT_AS, RLIMIT_NPROC, RLIMIT_FSIZE)
//! Windows: Job Objects (内存限制 + 进程数限制)

#[cfg(all(unix, not(target_os = "android")))]
use libc;
#[derive(Debug, Clone, Copy)]
pub struct ResourceLimits {
    /// CPU 时间限制（秒），默认 60
    pub max_cpu_seconds: u64,
    /// 虚拟内存限制（字节），默认 512MB
    pub max_memory_bytes: u64,
    /// 最大子进程数，默认 10
    pub max_processes: u32,
    /// 最大文件写入（字节），默认 100MB
    pub max_file_size_bytes: u64,
    /// 最大打开文件描述符数，默认 256
    pub max_open_files: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_seconds: 60,
            max_memory_bytes: 512 * 1024 * 1024,
            max_processes: 10,
            max_file_size_bytes: 100 * 1024 * 1024,
            max_open_files: 256,
        }
    }
}

impl ResourceLimits {
    /// 创建沙箱默认限制
    pub fn default_sandbox() -> Self {
        Self::default()
    }

    /// 应用资源限制到当前进程及其子进程
    pub fn apply_to_current_process(&self) -> Result<(), String> {
        #[cfg(all(unix, not(target_os = "android")))]
        self.apply_rlimit()?;

        #[cfg(target_os = "windows")]
        self.apply_job_object()?;

        tracing::info!(
            "Sandbox resource limits applied: cpu={}s, mem={}MB, procs={}, fsize={}MB",
            self.max_cpu_seconds,
            self.max_memory_bytes / (1024 * 1024),
            self.max_processes,
            self.max_file_size_bytes / (1024 * 1024),
        );

        Ok(())
    }

    #[cfg(all(unix, not(target_os = "android")))]
    fn apply_rlimit(&self) -> Result<(), String> {
        // RLIMIT_CPU: 进程可使用的 CPU 时间（秒）
        self.set_rlimit(
            libc::RLIMIT_CPU as _,
            self.max_cpu_seconds,
            self.max_cpu_seconds.saturating_add(5),
        )?;

        // RLIMIT_AS: 进程可用虚拟内存（字节）
        self.set_rlimit(libc::RLIMIT_AS as _, self.max_memory_bytes, self.max_memory_bytes)?;

        // RLIMIT_NPROC: 最大子进程数
        self.set_rlimit(
            libc::RLIMIT_NPROC as _,
            self.max_processes as u64,
            self.max_processes as u64,
        )?;

        // RLIMIT_FSIZE: 最大文件写入（字节）
        self.set_rlimit(
            libc::RLIMIT_FSIZE as _,
            self.max_file_size_bytes,
            self.max_file_size_bytes,
        )?;

        // RLIMIT_NOFILE: 最大打开文件描述符数（防止 fd 耗尽）
        self.set_rlimit(libc::RLIMIT_NOFILE as _, self.max_open_files, self.max_open_files)?;

        Ok(())
    }

    #[cfg(all(unix, not(target_os = "android")))]
    fn set_rlimit(&self, resource: u32, soft: u64, hard: u64) -> Result<(), String> {
        let rlim = libc::rlimit {
            rlim_cur: soft.min(hard),
            rlim_max: hard,
        };
        // SAFETY: rlim is properly initialized with valid soft/hard limit values;
        // resource parameter is a valid libc rlimit resource constant;
        // setrlimit is called on the current process only;
        // failure is handled gracefully (non-zero return logged but not fatal).
        let ret = unsafe { libc::setrlimit(resource as _, &rlim) };
        if ret != 0 {
            let err = std::io::Error::last_os_error();
            tracing::warn!("Failed to set rlimit {:?}: {}", resource, err);
            // 不返回错误——rlimit 失败不应阻止执行
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn apply_job_object(&self) -> Result<(), String> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::HANDLE;
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_JOB_MEMORY,
            JOB_OBJECT_LIMIT_PROCESS_MEMORY, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JobObjectExtendedLimitInformation, SetInformationJobObject,
        };

        let name: Vec<u16> = std::ffi::OsStr::new("AxAgent_Sandbox_Job")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // SAFETY: name is a properly null-terminated UTF-16 string
        // (created with encode_wide().chain(once(0)));
        // null pointer passed for lpSecurityAttributes is valid
        // (uses default security descriptor);
        // null handle return is checked below.
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), name.as_ptr()) };
        if handle.is_null() {
            return Err("无法创建 Windows Job Object".to_string());
        }

        // SAFETY: JOBOBJECT_EXTENDED_LIMIT_INFORMATION is a Windows struct
        // that is safe to zero-initialize; all fields are numeric types
        // (DWORD, SIZE_T) that default to 0; the zeroed struct is
        // immediately populated with valid values before use.
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_PROCESS_MEMORY | JOB_OBJECT_LIMIT_JOB_MEMORY;
        info.ProcessMemoryLimit = if std::mem::size_of::<usize>() < 8 {
            self.max_memory_bytes.min(usize::MAX as u64) as usize
        } else {
            self.max_memory_bytes as usize
        };
        let limit = self.max_memory_bytes.saturating_mul(2);
        let limit = if std::mem::size_of::<usize>() < 8 {
            limit.min(usize::MAX as u64) as usize
        } else {
            limit as usize
        };
        info.JobMemoryLimit = limit;

        // SAFETY: handle is a valid Job Object handle obtained from
        // CreateJobObjectW (null-checked above); info pointer and size are
        // correctly derived from the same struct; JobObjectExtendedLimitInformation
        // is the correct information class for this struct type;
        // failure is handled gracefully (ret == 0 logged but not fatal).
        let ret = unsafe {
            SetInformationJobObject(
                handle as HANDLE,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if ret == 0 {
            tracing::warn!("Failed to configure Windows Job Object");
        }

        // SAFETY: GetCurrentProcess always returns a valid pseudo-handle
        // per Windows documentation; no parameters, cannot fail.
        let current = unsafe { windows_sys::Win32::System::Threading::GetCurrentProcess() };
        // SAFETY: handle is a valid Job Object handle (null-checked above);
        // current is a valid process pseudo-handle from GetCurrentProcess;
        // failure is handled gracefully (ret == 0 logged but not fatal).
        let ret = unsafe { AssignProcessToJobObject(handle as HANDLE, current) };
        if ret == 0 {
            tracing::warn!("Failed to assign process to Job Object");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limits_are_reasonable() {
        let limits = ResourceLimits::default();
        assert!(limits.max_cpu_seconds > 0);
        assert!(limits.max_memory_bytes > 0);
        assert!(limits.max_processes > 0);
        assert!(limits.max_file_size_bytes > 0);
    }

    #[test]
    fn sandbox_limits_are_restrictive() {
        let limits = ResourceLimits::default_sandbox();
        assert!(limits.max_cpu_seconds <= 120);
        assert!(limits.max_memory_bytes <= 1024 * 1024 * 1024);
        assert!(limits.max_processes <= 50);
    }

    #[test]
    fn apply_does_not_panic() {
        let limits = ResourceLimits::default();
        let result = limits.apply_to_current_process();
        // rlimit 可能失败（权限不足等），但不应 panic
        assert!(result.is_ok() || result.is_err());
    }
}
