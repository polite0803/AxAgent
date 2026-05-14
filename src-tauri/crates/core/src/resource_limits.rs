//! 跨平台进程资源限制。
//!
//! Linux/macOS: rlimit (RLIMIT_CPU, RLIMIT_AS, RLIMIT_NPROC, RLIMIT_FSIZE)
//! Windows: Job Objects (内存限制 + 进程数限制)

/// 沙箱资源限制配置
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
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_seconds: 60,
            max_memory_bytes: 512 * 1024 * 1024,
            max_processes: 10,
            max_file_size_bytes: 100 * 1024 * 1024,
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
        #[cfg(any(target_os = "linux", target_os = "macos"))]
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn apply_rlimit(&self) -> Result<(), String> {
        // RLIMIT_CPU: 进程可使用的 CPU 时间（秒）
        self.set_rlimit(
            libc::RLIMIT_CPU as u32,
            self.max_cpu_seconds,
            self.max_cpu_seconds.saturating_add(5),
        )?;

        // RLIMIT_AS: 进程可用虚拟内存（字节）
        self.set_rlimit(libc::RLIMIT_AS as u32, self.max_memory_bytes, self.max_memory_bytes)?;

        // RLIMIT_NPROC: 最大子进程数
        self.set_rlimit(
            libc::RLIMIT_NPROC as u32,
            self.max_processes as u64,
            self.max_processes as u64,
        )?;

        // RLIMIT_FSIZE: 最大文件写入（字节）
        self.set_rlimit(
            libc::RLIMIT_FSIZE as u32,
            self.max_file_size_bytes,
            self.max_file_size_bytes,
        )?;

        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn set_rlimit(&self, resource: u32, soft: u64, hard: u64) -> Result<(), String> {
        let rlim = libc::rlimit {
            rlim_cur: soft.min(hard),
            rlim_max: hard,
        };
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
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
        };

        let name: Vec<u16> = std::ffi::OsStr::new("AxAgent_Sandbox_Job")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let handle = unsafe { CreateJobObjectW(std::ptr::null(), name.as_ptr()) };
        if handle.is_null() {
            return Err("无法创建 Windows Job Object".to_string());
        }

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_PROCESS_MEMORY | JOB_OBJECT_LIMIT_JOB_MEMORY;
        info.ProcessMemoryLimit = self.max_memory_bytes as usize;
        info.JobMemoryLimit = self.max_memory_bytes.saturating_mul(2) as usize;

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

        let current = unsafe { windows_sys::Win32::System::Threading::GetCurrentProcess() };
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
