//! Bash 安全分析器
//!
//! 命令白名单、危险模式检测、安全 flag 映射。

use super::parser::ParsedCommand;
use std::collections::HashSet;

/// 命令白名单条目
#[allow(dead_code)]
struct CommandSpec {
    /// 可执行文件名
    name: &'static str,
    /// 允许的安全标志
    safe_flags: &'static [(&'static str, FlagArg)],
    /// 是否允许无参运行
    allow_no_args: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum FlagArg {
    /// 无参数 (如 -l, --help)
    None,
    /// 数字参数
    Number,
    /// 字符串参数
    String,
    /// 可选字符串
    OptionalString,
}

/// 安全分析器
pub struct SecurityAnalyzer {
    /// 允许的命令集
    allowed_commands: HashSet<&'static str>,
}

impl SecurityAnalyzer {
    pub fn new() -> Self {
        let mut allowed = HashSet::new();

        // 版本控制
        allowed.extend(["git", "hg", "svn"]);
        // 文件操作
        allowed.extend([
            "ls", "dir", "cat", "head", "tail", "wc", "echo", "pwd", "which", "type",
        ]);
        // 搜索
        allowed.extend(["find", "grep", "rg", "locate", "fd"]);
        // 文本处理
        allowed.extend([
            "sed", "awk", "sort", "uniq", "cut", "tr", "wc", "diff", "patch",
        ]);
        // 构建工具
        allowed.extend([
            "cargo", "npm", "yarn", "pnpm", "npx", "go", "python", "python3", "node", "rustc",
            "make", "cmake",
        ]);
        // 包管理
        allowed.extend(["pip", "pip3", "gem", "composer"]);
        // 系统信息
        allowed.extend([
            "uname", "whoami", "hostname", "date", "env", "printenv", "id", "uptime",
        ]);
        // 进程信息
        allowed.extend(["ps", "top", "htop"]);
        // 网络信息
        allowed.extend(["curl", "wget", "ping", "nslookup", "dig"]);
        // 容器
        allowed.extend(["docker", "docker-compose", "kubectl", "podman"]);
        // 文本编辑
        allowed.extend(["nano", "vim", "vi", "code", "idea"]);
        // Windows 命令
        allowed.extend([
            "dir",
            "type",
            "findstr",
            "copy",
            "move",
            "del",
            "mkdir",
            "rmdir",
            "tasklist",
            "systeminfo",
        ]);
        // 其他常用
        allowed.extend([
            "ssh", "scp", "rsync", "tar", "gzip", "gunzip", "zip", "unzip", "jq", "xargs", "tee",
            "basename", "dirname", "realpath", "readlink",
        ]);

        Self {
            allowed_commands: allowed,
        }
    }

    /// 分析命令安全性
    pub fn analyze(&self, cmd: &ParsedCommand) -> SecurityResult {
        if cmd.argv.is_empty() {
            return SecurityResult::safe("空命令");
        }

        let program = &cmd.argv[0];
        let program_base = std::path::Path::new(program)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(program);

        // 1. 检测绝对路径执行
        if program.starts_with('/') || program.starts_with(".\\") || program.starts_with("../") {
            return SecurityResult::dangerous("禁止使用绝对路径或相对路径执行命令");
        }

        // 2. 检测危险模式
        let full_cmd = cmd.argv.join(" ").to_lowercase();
        let dangerous_patterns = [
            ("rm -rf /", "删除根目录"),
            ("rm -rf /*", "删除根目录内容"),
            ("mkfs.", "格式化文件系统"),
            ("dd if=", "直接写入设备"),
            (":(){ :|:& };:", "Fork 炸弹"),
            ("eval ", "代码注入"),
            ("sudo ", "提权操作"),
            ("chmod 777 /", "开放系统权限"),
            ("> /dev/sda", "写入设备"),
            ("curl | bash", "管道执行远程代码"),
            ("wget | bash", "管道执行远程代码"),
            ("reboot", "重启系统"),
            ("shutdown", "关机"),
        ];

        for (pattern, reason) in &dangerous_patterns {
            if full_cmd.contains(pattern) {
                return SecurityResult::dangerous(*reason);
            }
        }

        // 3. 检查输出重定向
        for redirect in &cmd.redirects {
            let target = redirect.target.to_lowercase();
            let dangerous_targets = [
                "/etc/",
                "/boot/",
                "/sys/",
                "/proc/",
                "~/.ssh/",
                "/root/",
                "c:\\windows\\",
                "c:\\program files\\",
            ];
            for dt in &dangerous_targets {
                if target.contains(dt) {
                    return SecurityResult::Warning(format!("重定向目标包含系统路径: {}", dt));
                }
            }
        }

        // 4. 命令白名单检查
        if !self.allowed_commands.contains(program_base) {
            return SecurityResult::Warning(format!(
                "命令 '{}' 不在白名单中，请检查后执行",
                program_base
            ));
        }

        SecurityResult::safe("命令通过安全审查")
    }
}

impl Default for SecurityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum SecurityResult {
    /// 安全
    Safe(String),
    /// 警告（需要用户确认）
    Warning(String),
    /// 阻止
    Blocked(String),
}

impl SecurityResult {
    pub fn safe(reason: impl Into<String>) -> Self {
        SecurityResult::Safe(reason.into())
    }

    pub fn dangerous(reason: impl Into<String>) -> Self {
        SecurityResult::Blocked(reason.into())
    }

    pub fn is_safe(&self) -> bool {
        matches!(self, SecurityResult::Safe(_))
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, SecurityResult::Blocked(_))
    }
}
