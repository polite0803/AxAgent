//! 沙箱执行器
//!
//! 支持 Docker/podman 容器执行和进程级隔离。

use std::process::Command;

/// 沙箱配置
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// 使用容器（Docker/Podman）
    pub use_container: bool,
    /// 容器镜像
    pub image: String,
    /// 允许的网络访问
    pub allow_network: bool,
    /// 挂载的卷列表
    pub volumes: Vec<(String, String)>,
    /// 内存限制 (MB)
    pub memory_limit_mb: Option<u64>,
    /// CPU 限制
    pub cpu_limit: Option<f64>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            use_container: false,
            image: "ubuntu:latest".into(),
            allow_network: false,
            volumes: Vec::new(),
            memory_limit_mb: Some(512),
            cpu_limit: Some(0.5),
        }
    }
}

/// 沙箱执行器
pub struct SandboxRunner {
    config: SandboxConfig,
}

impl SandboxRunner {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    /// 构建沙箱命令
    pub fn build_command(&self, cmd: &str, working_dir: &str) -> Command {
        if self.config.use_container {
            self.build_docker_command(cmd, working_dir)
        } else {
            self.build_native_command(cmd, working_dir)
        }
    }

    fn build_docker_command(&self, cmd: &str, working_dir: &str) -> Command {
        let mut command = Command::new("docker");
        command.arg("run");
        command.arg("--rm");
        command.arg("--network=none");
        command.arg("--read-only");
        command.arg(format!("--workdir={}", working_dir));
        command.arg("-v");
        command.arg(format!("{}:{}", working_dir, working_dir));

        if let Some(mem) = self.config.memory_limit_mb {
            command.arg(format!("--memory={}m", mem));
        }
        if let Some(cpu) = self.config.cpu_limit {
            command.arg(format!("--cpus={}", cpu));
        }

        command.arg(&self.config.image);
        command.arg("bash");
        command.arg("-c");
        command.arg(cmd);

        command
    }

    fn build_native_command(&self, cmd: &str, _working_dir: &str) -> Command {
        #[cfg(target_os = "windows")]
        {
            let mut command = Command::new("cmd");
            command.arg("/C");
            command.arg(cmd);
            command
        }

        #[cfg(not(target_os = "windows"))]
        {
            let mut command = Command::new("bash");
            command.arg("-c");
            command.arg(cmd);
            command
        }
    }

    /// 执行沙箱命令
    pub fn execute(&self, cmd: &str, working_dir: &str) -> std::io::Result<std::process::Output> {
        let mut command = self.build_command(cmd, working_dir);
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        command.output()
    }
}
