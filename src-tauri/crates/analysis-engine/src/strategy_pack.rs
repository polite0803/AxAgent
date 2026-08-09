// SPDX-License-Identifier: AGPL-3.0-only

//! 策略包加载器与适配器
//!
//! 在 `axagent-harness::strategy_pack` 契约层之上，提供：
//!
//! 1. **文件系统加载**：从目录扫描 `.yaml` 文件并解析为 `StrategyPackSpec`
//! 2. **适配转换**：把 `StrategyPackSpec` 转换为内部 `LoadedStrategyPack`，
//!    将 YAML params 转换为 `recommend_stocks` 可消费的 `template_vars`
//! 3. **内存管理**：已加载策略包的注册表，支持热重载
//!
//! ## 与现有 recommender 的关系
//!
//! 现有 `recommend_stocks` 通过 `template_vars: &[(String, Value)]` 接收参数。
//! 策略包不替换现有策略实现，而是作为"参数预设包"：
//! - 用户在 YAML 中配置 `params`（如 `trend_short_kline_limit: 80`）
//! - 适配器把 params 转换为 `template_vars` 格式
//! - `recommend_stocks` 照常读取 template_vars，无需修改
//!
//! 这避免了重复实现策略逻辑，符合铁律 12「禁止重复定义」。

use std::collections::HashMap;
use std::path::Path;

use axagent_harness::strategy_pack::{StrategyPackError, StrategyPackManifest, StrategyPackSpec};
use serde_json::Value;

/// 已加载的策略包（含运行时状态）
#[derive(Debug, Clone)]
pub struct LoadedStrategyPack {
    /// 清单信息
    pub manifest: StrategyPackManifest,
    /// 完整规格
    pub spec: StrategyPackSpec,
    /// 转换后的 template_vars（供 `recommend_stocks` 消费）
    pub template_vars: Vec<(String, Value)>,
}

impl LoadedStrategyPack {
    /// 从 `StrategyPackSpec` 构造已加载包，并预计算 template_vars
    pub fn from_spec(spec: StrategyPackSpec, source: &str) -> Result<Self, StrategyPackError> {
        spec.validate()?;
        let manifest = StrategyPackManifest {
            id: derive_id_from_source(source),
            name: spec.name.clone(),
            description: spec.description.clone(),
            version: spec.version.clone(),
            author: spec.author.clone(),
            strategy_count: spec.strategies.len(),
            enabled_count: spec.enabled_entries().count(),
            enabled: true,
            source: source.to_string(),
        };
        let template_vars = build_template_vars(&spec);
        Ok(Self { manifest, spec, template_vars })
    }
}

/// 从文件加载单个策略包
///
/// 支持的文件扩展名：`.yaml` / `.yml`
pub fn load_pack_from_file(path: &Path) -> Result<LoadedStrategyPack, StrategyPackError> {
    if !path.exists() {
        return Err(StrategyPackError::FileNotFound(path.display().to_string()));
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| StrategyPackError::FileRead(format!("{}: {e}", path.display())))?;
    let spec = StrategyPackSpec::from_yaml(&content)?;
    let source = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
    LoadedStrategyPack::from_spec(spec, &source)
}

/// 从目录扫描所有 `.yaml`/`.yml` 策略包
///
/// 非递归扫描，仅读取目录顶层的策略包文件。
/// 单个文件解析失败不会中断整体扫描，错误会记录到返回的 `LoadResult.errors`。
pub fn load_packs_from_dir(dir: &Path) -> LoadResult {
    let mut packs = Vec::new();
    let mut errors = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            return LoadResult {
                packs,
                errors: vec![format!("无法读取目录 {}: {e}", dir.display())],
            };
        },
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_yaml_file(&path) {
            continue;
        }
        match load_pack_from_file(&path) {
            Ok(pack) => packs.push(pack),
            Err(e) => {
                errors.push(format!("{}: {e}", path.display()));
            },
        }
    }
    // 按包名排序，保证展示稳定
    packs.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    LoadResult { packs, errors }
}

/// 目录加载结果
#[derive(Debug, Default)]
pub struct LoadResult {
    /// 成功加载的策略包
    pub packs: Vec<LoadedStrategyPack>,
    /// 单个文件加载失败的错误信息
    pub errors: Vec<String>,
}

impl LoadResult {
    pub fn is_empty(&self) -> bool {
        self.packs.is_empty() && self.errors.is_empty()
    }
}

/// 策略包注册表（内存态，支持热重载）
#[derive(Debug, Default)]
pub struct StrategyPackRegistry {
    packs: HashMap<String, LoadedStrategyPack>,
}

impl StrategyPackRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个已加载的策略包（ID 重复则覆盖）
    pub fn register(&mut self, pack: LoadedStrategyPack) {
        self.packs.insert(pack.manifest.id.clone(), pack);
    }

    /// 注销策略包
    pub fn unregister(&mut self, id: &str) -> Option<LoadedStrategyPack> {
        self.packs.remove(id)
    }

    /// 获取策略包
    pub fn get(&self, id: &str) -> Option<&LoadedStrategyPack> {
        self.packs.get(id)
    }

    /// 列出所有策略包清单
    pub fn list_manifests(&self) -> Vec<&StrategyPackManifest> {
        let mut items: Vec<_> = self.packs.values().map(|p| &p.manifest).collect();
        items.sort_by(|a, b| a.name.cmp(&b.name));
        items
    }

    /// 从目录加载并替换所有策略包（热重载）
    ///
    /// 返回加载结果（含成功加载的包和错误信息）
    pub fn reload_from_dir(&mut self, dir: &Path) -> LoadResult {
        let result = load_packs_from_dir(dir);
        self.packs.clear();
        for pack in &result.packs {
            self.packs.insert(pack.manifest.id.clone(), pack.clone());
        }
        result
    }

    /// 获取指定策略包的 template_vars（供 `recommend_stocks` 消费）
    pub fn template_vars_for(&self, id: &str) -> Option<&[(String, Value)]> {
        self.packs.get(id).map(|p| p.template_vars.as_slice())
    }
}

// ── 内部辅助函数 ──

/// 判断是否为 YAML 文件
fn is_yaml_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_lowercase().as_str(), "yaml" | "yml"))
        .unwrap_or(false)
}

/// 从来源路径派生策略包 ID
fn derive_id_from_source(source: &str) -> String {
    // 用文件名（不含扩展名）作为 ID；非文件来源用 source 本身
    Path::new(source).file_stem().and_then(|s| s.to_str()).unwrap_or(source).to_string()
}

/// 把 `StrategyPackSpec` 转换为 `template_vars`
///
/// 转换规则：
/// 1. 包级 `min_confidence` → `reco_min_confidence`
/// 2. 每个策略条目的 params 展开为顶层 key（按 `{style}_{period}_{param_name}` 命名）
/// 3. 每个策略条目的 weight → `reco_strategy_weights["{style}_{period}"]`
/// 4. 每个策略条目的 enabled → `reco_{style}_enabled`
fn build_template_vars(spec: &StrategyPackSpec) -> Vec<(String, Value)> {
    let mut vars: Vec<(String, Value)> = Vec::new();

    // 包级最低置信度
    vars.push(("reco_min_confidence".to_string(), Value::Number(spec.min_confidence.into())));

    // 策略权重表
    let mut weights: HashMap<String, f64> = HashMap::new();
    for entry in &spec.strategies {
        let key = format!("{}_{}", entry.style.as_str(), entry.period.as_str());
        weights.insert(key, entry.weight);
    }
    if !weights.is_empty() {
        let weights_obj: serde_json::Map<String, Value> = weights
            .into_iter()
            .map(|(k, v)| {
                (k, Value::Number(serde_json::Number::from_f64(v).unwrap_or_else(|| 0.into())))
            })
            .collect();
        vars.push(("reco_strategy_weights".to_string(), Value::Object(weights_obj)));
    }

    // 各风格启停状态
    for entry in &spec.strategies {
        let key = format!("reco_{}_enabled", entry.style.as_str());
        // 同一风格可能有多个 period，只要有一个 enabled 就启用
        let current =
            vars.iter().find(|(k, _)| k == &key).and_then(|(_, v)| v.as_bool()).unwrap_or(false);
        if entry.enabled || current {
            vars.retain(|(k, _)| k != &key);
            vars.push((key, Value::Bool(true)));
        }
    }

    // 每个策略条目的 params 展开
    for entry in &spec.strategies {
        let prefix = format!("{}_{}", entry.style.as_str(), entry.period.as_str());
        for (param_name, param_value) in &entry.params {
            let key = format!("{prefix}_{param_name}");
            vars.push((key, param_value.clone()));
        }
    }

    vars
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::strategy_pack::{
        StrategyPackPeriod, StrategyPackStrategyEntry, StrategyPackStyle,
    };

    fn make_spec() -> StrategyPackSpec {
        let mut params = HashMap::new();
        params.insert("kline_limit".to_string(), Value::Number(80.into()));
        StrategyPackSpec {
            name: "测试包".to_string(),
            description: "测试".to_string(),
            version: "1.0.0".to_string(),
            author: "test".to_string(),
            min_confidence: 70,
            max_picks: 5,
            strategies: vec![StrategyPackStrategyEntry {
                id: "trend_short".to_string(),
                strategy_id: "trend".to_string(),
                style: StrategyPackStyle::Trend,
                period: StrategyPackPeriod::Short,
                enabled: true,
                weight: 1.5,
                params,
                min_confidence: None,
            }],
        }
    }

    #[test]
    fn from_spec_builds_template_vars() {
        let spec = make_spec();
        let pack = LoadedStrategyPack::from_spec(spec, "test_pack").unwrap();
        assert_eq!(pack.manifest.id, "test_pack");
        assert_eq!(pack.manifest.strategy_count, 1);
        assert_eq!(pack.manifest.enabled_count, 1);

        // template_vars 应包含 reco_min_confidence
        let min_conf = pack
            .template_vars
            .iter()
            .find(|(k, _)| k == "reco_min_confidence")
            .and_then(|(_, v)| v.as_u64());
        assert_eq!(min_conf, Some(70));

        // 应包含权重表
        let weights = pack
            .template_vars
            .iter()
            .find(|(k, _)| k == "reco_strategy_weights")
            .and_then(|(_, v)| v.as_object());
        assert!(weights.is_some());
        assert_eq!(weights.unwrap().get("trend_short").and_then(|v| v.as_f64()), Some(1.5));

        // 应包含展开的 params
        let kline_limit = pack
            .template_vars
            .iter()
            .find(|(k, _)| k == "trend_short_kline_limit")
            .and_then(|(_, v)| v.as_u64());
        assert_eq!(kline_limit, Some(80));

        // 应包含风格启用状态
        let trend_enabled = pack
            .template_vars
            .iter()
            .find(|(k, _)| k == "reco_trend_enabled")
            .and_then(|(_, v)| v.as_bool());
        assert_eq!(trend_enabled, Some(true));
    }

    #[test]
    fn registry_register_and_get() {
        let mut reg = StrategyPackRegistry::new();
        let pack = LoadedStrategyPack::from_spec(make_spec(), "test").unwrap();
        reg.register(pack);
        assert!(reg.get("test").is_some());
        assert_eq!(reg.list_manifests().len(), 1);
        assert!(reg.template_vars_for("test").is_some());
        reg.unregister("test");
        assert!(reg.get("test").is_none());
    }

    #[test]
    fn load_pack_from_file_not_found() {
        let result = load_pack_from_file(Path::new("/nonexistent/pack.yaml"));
        assert!(matches!(result, Err(StrategyPackError::FileNotFound(_))));
    }

    #[test]
    fn load_packs_from_empty_dir() {
        let dir = std::env::temp_dir();
        let result = load_packs_from_dir(&dir);
        // 临时目录可能没有 yaml 文件，packs 应为空
        assert!(result.packs.is_empty());
    }

    #[test]
    fn is_yaml_file_checks_extension() {
        assert!(is_yaml_file(Path::new("foo.yaml")));
        assert!(is_yaml_file(Path::new("foo.yml")));
        assert!(is_yaml_file(Path::new("foo.YAML")));
        assert!(!is_yaml_file(Path::new("foo.json")));
        assert!(!is_yaml_file(Path::new("foo.txt")));
    }

    #[test]
    fn build_template_vars_handles_disabled() {
        let mut spec = make_spec();
        spec.strategies[0].enabled = false;
        let pack = LoadedStrategyPack::from_spec(spec, "test").unwrap();
        let trend_enabled = pack
            .template_vars
            .iter()
            .find(|(k, _)| k == "reco_trend_enabled")
            .and_then(|(_, v)| v.as_bool());
        // disabled 策略不应设置 enabled=true
        assert_ne!(trend_enabled, Some(true));
    }

    /// 验证内置 YAML 策略包（agency_experts/stock-analysis/strategy_packs/）能被正确解析。
    ///
    /// 使用 `include_str!` 编译期嵌入，确保内置预设随二进制分发且格式始终有效。
    /// 这是 1.1.3 的集成验证：任何内置 YAML 格式错误都会在测试期暴露。
    #[test]
    fn builtin_packs_parse_successfully() {
        // 抽样验证 6 个代表性内置策略包（覆盖全部 6 风格）
        const SAMPLES: &[(&str, &str)] = &[
            (
                "balanced-default",
                include_str!(
                    "../../../agency_experts/stock-analysis/strategy_packs/balanced-default.yaml"
                ),
            ),
            (
                "trend-focused",
                include_str!(
                    "../../../agency_experts/stock-analysis/strategy_packs/trend-focused.yaml"
                ),
            ),
            (
                "value-focused",
                include_str!(
                    "../../../agency_experts/stock-analysis/strategy_packs/value-focused.yaml"
                ),
            ),
            (
                "capital-flow",
                include_str!(
                    "../../../agency_experts/stock-analysis/strategy_packs/capital-flow.yaml"
                ),
            ),
            (
                "reversion-focused",
                include_str!(
                    "../../../agency_experts/stock-analysis/strategy_packs/reversion-focused.yaml"
                ),
            ),
            (
                "serenity-bottleneck",
                include_str!(
                    "../../../agency_experts/stock-analysis/strategy_packs/serenity-bottleneck.yaml"
                ),
            ),
        ];
        for (id, yaml) in SAMPLES {
            let spec = StrategyPackSpec::from_yaml(yaml)
                .unwrap_or_else(|e| panic!("内置策略包 {id} 解析失败: {e}"));
            spec.validate().unwrap_or_else(|e| panic!("内置策略包 {id} 校验失败: {e}"));
            let pack = LoadedStrategyPack::from_spec(spec, id)
                .unwrap_or_else(|e| panic!("内置策略包 {id} 加载失败: {e}"));
            // 每个包至少有 1 个策略条目
            assert!(!pack.spec.strategies.is_empty(), "内置策略包 {id} 策略列表为空");
            // template_vars 必须包含包级最低置信度
            assert!(
                pack.template_vars.iter().any(|(k, _)| k == "reco_min_confidence"),
                "内置策略包 {id} 缺少 reco_min_confidence",
            );
        }
    }
}
