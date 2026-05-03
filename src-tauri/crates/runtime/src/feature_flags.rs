//! Feature Flag 系统 — 通过环境变量和配置文件控制功能开关
//!
//! 优先级：环境变量 `AXAGENT_FF_<NAME>=1` > settings.json `features` 段 > 默认值
//!
//! ## 内置 Feature Flag 列表
//!
//! | Flag 名称 | 默认值 | 说明 |
//! |-----------|--------|------|
//! | `FORK_SUBAGENT` | false | 隐式 fork 子 agent，共享 prompt cache |
//! | `COORDINATOR_MODE` | false | 协调器模式（主 agent 只能 spawn worker） |
//! | `PROACTIVE_MODE` | false | 主动模式（空闲时自动 tick） |
//! | `SWARM_MODE` | false | 跨进程 Swarm/Teammate 协作 |
//! | `REMOTE_AGENT` | false | 远程 agent 执行 |
//! | `VERIFICATION_AGENT` | false | 独立的验证 agent |
//! | `TOOL_CONCURRENCY` | true | 工具并发安全批量执行 |
//! | `ACP_PROTOCOL` | false | ACP 协议服务端 |
//! | `DREAM_TASK` | true | 梦境任务（后台整合压缩） |
//! | `SUBSCRIBE_PR` | true | PR 订阅通知 |

use std::collections::{BTreeMap, HashMap};
use std::sync::RwLock;

/// Feature Flag 的注册描述
#[derive(Debug, Clone)]
pub struct FeatureFlagDef {
    /// 唯一标识名（大写蛇形）
    pub name: &'static str,
    /// 默认值
    pub default: bool,
    /// 说明
    pub description: &'static str,
}

/// 内置 Feature Flag 定义表
const BUILTIN_FEATURE_FLAGS: &[FeatureFlagDef] = &[
    FeatureFlagDef {
        name: "FORK_SUBAGENT",
        default: false,
        description: "隐式 fork 子 agent，共享 prompt cache",
    },
    FeatureFlagDef {
        name: "COORDINATOR_MODE",
        default: false,
        description: "协调器模式（主 agent 只能 spawn worker）",
    },
    FeatureFlagDef {
        name: "PROACTIVE_MODE",
        default: false,
        description: "主动模式（空闲时自动 tick）",
    },
    FeatureFlagDef {
        name: "SWARM_MODE",
        default: false,
        description: "跨进程 Swarm/Teammate 协作",
    },
    FeatureFlagDef {
        name: "REMOTE_AGENT",
        default: false,
        description: "远程 agent 执行",
    },
    FeatureFlagDef {
        name: "VERIFICATION_AGENT",
        default: false,
        description: "独立的验证 agent",
    },
    FeatureFlagDef {
        name: "TOOL_CONCURRENCY",
        default: true,
        description: "工具并发安全批量执行",
    },
    FeatureFlagDef {
        name: "ACP_PROTOCOL",
        default: false,
        description: "ACP 协议服务端",
    },
    FeatureFlagDef {
        name: "DREAM_TASK",
        default: true,
        description: "梦境任务（后台整合压缩）",
    },
    FeatureFlagDef {
        name: "SUBSCRIBE_PR",
        default: true,
        description: "PR 订阅通知",
    },
];

/// Feature Flag 全局状态
///
/// 运行时动态查询，支持从环境变量和配置文件覆盖。
#[derive(Debug)]
pub struct FeatureFlags {
    /// 当前生效的标志值
    flags: RwLock<HashMap<String, bool>>,
    /// 从 settings.json `features` 段解析的覆盖值
    config_overrides: HashMap<String, bool>,
}

impl FeatureFlags {
    /// 从环境变量和配置加载所有 feature flag
    pub fn new(config_features: Option<&BTreeMap<String, bool>>) -> Self {
        let mut config_overrides = HashMap::new();
        if let Some(features) = config_features {
            for (key, value) in features {
                config_overrides.insert(key.to_uppercase(), *value);
            }
        }

        let flags = Self::build_flags(&config_overrides);

        Self {
            flags: RwLock::new(flags),
            config_overrides,
        }
    }

    /// 无配置覆盖时的默认构造
    pub fn defaults() -> Self {
        Self::new(None)
    }

    /// 从环境变量 + 配置覆盖重新计算标志值
    fn build_flags(config_overrides: &HashMap<String, bool>) -> HashMap<String, bool> {
        let mut flags = HashMap::new();
        for def in BUILTIN_FEATURE_FLAGS {
            let enabled = Self::resolve_flag(def.name, def.default, config_overrides);
            flags.insert(def.name.to_string(), enabled);
        }
        flags
    }

    /// 解析单个 flag 的最终值：环境变量 > 配置文件 > 默认值
    fn resolve_flag(name: &str, default: bool, config_overrides: &HashMap<String, bool>) -> bool {
        // 1. 环境变量 AXAGENT_FF_<NAME>
        let env_key = format!("AXAGENT_FF_{}", name);
        if let Ok(val) = std::env::var(&env_key) {
            return val != "0" && val != "false";
        }

        // 2. settings.json 中的 features.<name>
        if let Some(&enabled) = config_overrides.get(name) {
            return enabled;
        }

        // 3. 默认值
        default
    }

    /// 检查指定 flag 是否启用
    pub fn is_enabled(&self, name: &str) -> bool {
        let guard = self.flags.read().unwrap_or_else(|e| e.into_inner());
        guard.get(name).copied().unwrap_or(false)
    }

    /// 运行时启用一个 flag（仅当前会话）
    pub fn enable(&self, name: &str) {
        if let Ok(mut guard) = self.flags.write() {
            guard.insert(name.to_uppercase(), true);
        }
    }

    /// 运行时禁用一个 flag（仅当前会话）
    pub fn disable(&self, name: &str) {
        if let Ok(mut guard) = self.flags.write() {
            guard.insert(name.to_uppercase(), false);
        }
    }

    /// 重新从环境变量和配置刷新所有 flag
    pub fn refresh(&self) {
        if let Ok(mut guard) = self.flags.write() {
            *guard = Self::build_flags(&self.config_overrides);
        }
    }

    /// 获取所有已注册 flag 的名称和当前值
    pub fn all_flags(&self) -> Vec<(String, bool)> {
        let guard = self.flags.read().unwrap_or_else(|e| e.into_inner());
        guard.iter().map(|(k, v)| (k.clone(), *v)).collect()
    }

    /// 获取所有 flag 定义的元信息
    pub fn definitions() -> &'static [FeatureFlagDef] {
        BUILTIN_FEATURE_FLAGS
    }

    // ── 便捷方法 ──

    /// Fork SubAgent 是否启用
    pub fn fork_subagent(&self) -> bool {
        self.is_enabled("FORK_SUBAGENT")
    }

    /// 协调器模式是否启用
    pub fn coordinator_mode(&self) -> bool {
        self.is_enabled("COORDINATOR_MODE")
    }

    /// 主动模式是否启用
    pub fn proactive_mode(&self) -> bool {
        self.is_enabled("PROACTIVE_MODE")
    }

    /// Swarm 模式是否启用
    pub fn swarm_mode(&self) -> bool {
        self.is_enabled("SWARM_MODE")
    }

    /// 远程 Agent 是否启用
    pub fn remote_agent(&self) -> bool {
        self.is_enabled("REMOTE_AGENT")
    }

    /// 验证 Agent 是否启用
    pub fn verification_agent(&self) -> bool {
        self.is_enabled("VERIFICATION_AGENT")
    }

    /// 工具并发执行是否启用
    pub fn tool_concurrency(&self) -> bool {
        self.is_enabled("TOOL_CONCURRENCY")
    }

    /// ACP 协议是否启用
    pub fn acp_protocol(&self) -> bool {
        self.is_enabled("ACP_PROTOCOL")
    }

    /// 梦境任务是否启用
    pub fn dream_task(&self) -> bool {
        self.is_enabled("DREAM_TASK")
    }

    /// PR 订阅是否启用
    pub fn subscribe_pr(&self) -> bool {
        self.is_enabled("SUBSCRIBE_PR")
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self::defaults()
    }
}

impl Clone for FeatureFlags {
    fn clone(&self) -> Self {
        let guard = self.flags.read().unwrap_or_else(|e| e.into_inner());
        Self {
            flags: RwLock::new(guard.clone()),
            config_overrides: self.config_overrides.clone(),
        }
    }
}

// == 全局单例 ==

static GLOBAL_FEATURE_FLAGS: std::sync::LazyLock<FeatureFlags> =
    std::sync::LazyLock::new(FeatureFlags::defaults);

/// 获取全局 FeatureFlags 引用
pub fn global_feature_flags() -> &'static FeatureFlags {
    &GLOBAL_FEATURE_FLAGS
}

/// 用指定配置初始化全局 FeatureFlags（应在 startup 时调用一次）
pub fn init_global_feature_flags(config_features: Option<&BTreeMap<String, bool>>) {
    let new_flags = FeatureFlags::new(config_features);
    // 通过 LazyLock 内部可变性刷新（这里使用一个技巧）
    let global = global_feature_flags();
    if let Ok(mut guard) = global.flags.write() {
        *guard = FeatureFlags::build_flags(&new_flags.config_overrides);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_return_builtin_values() {
        let flags = FeatureFlags::defaults();
        assert!(!flags.is_enabled("FORK_SUBAGENT"));
        assert!(!flags.is_enabled("COORDINATOR_MODE"));
        assert!(flags.is_enabled("TOOL_CONCURRENCY"));
        assert!(flags.is_enabled("DREAM_TASK"));
        assert!(!flags.is_enabled("NONEXISTENT"));
    }

    #[test]
    fn config_overrides_default() {
        let mut config = BTreeMap::new();
        config.insert("FORK_SUBAGENT".to_string(), true);
        let flags = FeatureFlags::new(Some(&config));
        assert!(flags.is_enabled("FORK_SUBAGENT"));
        // 未覆盖的保持默认值
        assert!(!flags.is_enabled("COORDINATOR_MODE"));
    }

    #[test]
    fn runtime_enable_disable() {
        let flags = FeatureFlags::defaults();
        assert!(!flags.is_enabled("COORDINATOR_MODE"));
        flags.enable("COORDINATOR_MODE");
        assert!(flags.is_enabled("COORDINATOR_MODE"));
        flags.disable("COORDINATOR_MODE");
        assert!(!flags.is_enabled("COORDINATOR_MODE"));
    }

    #[test]
    fn case_insensitive_names() {
        let flags = FeatureFlags::defaults();
        flags.enable("fork_subagent");
        assert!(flags.is_enabled("FORK_SUBAGENT"));
    }

    #[test]
    fn convenience_methods_match_is_enabled() {
        let flags = FeatureFlags::defaults();
        assert_eq!(flags.fork_subagent(), flags.is_enabled("FORK_SUBAGENT"));
        assert_eq!(
            flags.tool_concurrency(),
            flags.is_enabled("TOOL_CONCURRENCY")
        );
    }

    #[test]
    fn all_flags_returns_all_builtins() {
        let flags = FeatureFlags::defaults();
        let all = flags.all_flags();
        assert_eq!(all.len(), BUILTIN_FEATURE_FLAGS.len());
    }

    #[test]
    fn definitions_metadata_is_complete() {
        let defs = FeatureFlags::definitions();
        assert_eq!(defs.len(), 10);
        for def in defs {
            assert!(!def.name.is_empty());
            assert!(!def.description.is_empty());
        }
    }
}
