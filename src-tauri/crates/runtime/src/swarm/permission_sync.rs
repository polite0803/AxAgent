//! 权限同步 — leader 权限桥接到队友

/// 将 leader 的权限策略同步给队友
#[derive(Debug, Clone)]
pub struct PermissionBridge {
    /// 是否允许写操作
    pub allow_write: bool,
    /// 是否允许执行命令
    pub allow_execute: bool,
    /// 是否允许网络访问
    pub allow_network: bool,
}

impl PermissionBridge {
    /// 创建新的权限桥接
    pub fn new(allow_write: bool, allow_execute: bool, allow_network: bool) -> Self {
        Self {
            allow_write,
            allow_execute,
            allow_network,
        }
    }

    /// 从 leader 环境变量同步（默认全部允许）
    pub fn from_env() -> Self {
        Self {
            allow_write: true,
            allow_execute: true,
            allow_network: true,
        }
    }

    /// 生成传递给子进程的环境变量
    pub fn to_env_vars(&self) -> Vec<(String, String)> {
        vec![
            ("AXAGENT_ALLOW_WRITE".into(), self.allow_write.to_string()),
            (
                "AXAGENT_ALLOW_EXECUTE".into(),
                self.allow_execute.to_string(),
            ),
            (
                "AXAGENT_ALLOW_NETWORK".into(),
                self.allow_network.to_string(),
            ),
        ]
    }
}

impl Default for PermissionBridge {
    fn default() -> Self {
        Self::from_env()
    }
}
