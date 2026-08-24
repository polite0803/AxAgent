// SPDX-License-Identifier: AGPL-3.0-only

//! 策略包管理命令：列出/加载/校验 YAML 策略包
//!
//! 策略包来源：
//! - **内置预设**：编译期嵌入，随二进制分发，只读
//! - **用户自定义**：`~/Documents/axagent/strategy_packs/`，运行时可编辑
//!
//! 命令设计为无状态：每次调用直接从内存（内置）或文件系统（用户）加载，
//! 不在 AppState 中维护全局注册表，避免状态同步复杂度。

use axagent_agent_macro::agent_command;
use std::path::PathBuf;

use axagent_analysis_engine::strategy_pack::{LoadedStrategyPack, load_pack_from_file};
use axagent_harness::strategy_pack::{StrategyPackManifest, StrategyPackSpec};
use serde::Serialize;
use tauri::State;

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;

/// 内置策略包目录标识（用于前端区分来源）
const BUILTIN_SOURCE: &str = "builtin";

/// 用户策略包目录标识
const USER_SOURCE: &str = "user";

/// 内置策略包清单（编译期嵌入）
///
/// 每个元组：(id, name, description, version, author, yaml_content)
/// 使用 `include_str!` 嵌入，确保随二进制分发。
const BUILTIN_PACKS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "balanced-default",
        "平衡型默认策略包",
        "覆盖趋势/价值/资金/反弹四大风格，每个风格启用一个周期，适合新手入门",
        "1.0.0",
        "AxInvest",
        include_str!("../../agency_experts/stock-analysis/strategy_packs/balanced-default.yaml"),
    ),
    (
        "aggressive-short-term",
        "激进短线策略包",
        "聚焦超短线和短线趋势跟踪，高换手，适合追涨杀跌的活跃交易者",
        "1.0.0",
        "AxInvest",
        include_str!(
            "../../agency_experts/stock-analysis/strategy_packs/aggressive-short-term.yaml"
        ),
    ),
    (
        "conservative-long-term",
        "稳健长线策略包",
        "长线趋势跟踪 + 价值投资，低换手，适合长期持有的价值投资者",
        "1.0.0",
        "AxInvest",
        include_str!(
            "../../agency_experts/stock-analysis/strategy_packs/conservative-long-term.yaml"
        ),
    ),
    (
        "trend-focused",
        "趋势专精策略包",
        "覆盖超短/短/中/长四个周期的趋势跟踪策略，适合纯趋势交易者",
        "1.0.0",
        "AxInvest",
        include_str!("../../agency_experts/stock-analysis/strategy_packs/trend-focused.yaml"),
    ),
    (
        "value-focused",
        "价值专精策略包",
        "覆盖短/中/长三个周期的价值投资策略，聚焦 PE/PB/ROE 基本面",
        "1.0.0",
        "AxInvest",
        include_str!("../../agency_experts/stock-analysis/strategy_packs/value-focused.yaml"),
    ),
    (
        "capital-flow",
        "资金流专精策略包",
        "聚焦主力资金净流入、换手率、北向资金，适合跟踪主力的投资者",
        "1.0.0",
        "AxInvest",
        include_str!("../../agency_experts/stock-analysis/strategy_packs/capital-flow.yaml"),
    ),
    (
        "reversion-focused",
        "超跌反弹专精策略包",
        "聚焦超跌反弹策略，捕捉 RSI 超卖后的反弹机会，适合逆向投资者",
        "1.0.0",
        "AxInvest",
        include_str!("../../agency_experts/stock-analysis/strategy_packs/reversion-focused.yaml"),
    ),
    (
        "serenity-bottleneck",
        "Serenity 瓶颈分析策略包",
        "基于 Serenity 瓶颈分析策略，检测量价背离和突破信号",
        "1.0.0",
        "AxInvest",
        include_str!("../../agency_experts/stock-analysis/strategy_packs/serenity-bottleneck.yaml"),
    ),
    (
        "watchlist-monitor",
        "自选股监控策略包",
        "基于自选股列表的监控策略，对候选池进行兜底扫描",
        "1.0.0",
        "AxInvest",
        include_str!("../../agency_experts/stock-analysis/strategy_packs/watchlist-monitor.yaml"),
    ),
    (
        "ultra-short-scalper",
        "超短线打板策略包",
        "纯超短线策略，聚焦开盘冲量和尾盘异动，适合打板客",
        "1.0.0",
        "AxInvest",
        include_str!("../../agency_experts/stock-analysis/strategy_packs/ultra-short-scalper.yaml"),
    ),
    (
        "mid-term-balanced",
        "中线平衡策略包",
        "聚焦中周期，混合趋势/价值/资金/反弹四风格，适合中线投资者",
        "1.0.0",
        "AxInvest",
        include_str!("../../agency_experts/stock-analysis/strategy_packs/mid-term-balanced.yaml"),
    ),
    (
        "high-confidence-only",
        "高置信度精选策略包",
        "所有策略均使用高置信度阈值，只输出强信号，适合谨慎交易者",
        "1.0.0",
        "AxInvest",
        include_str!(
            "../../agency_experts/stock-analysis/strategy_packs/high-confidence-only.yaml"
        ),
    ),
    (
        "diversified-all-styles",
        "多风格分散策略包",
        "六大风格全覆盖，最大化分散风险，适合不想偏科的全能投资者",
        "1.0.0",
        "AxInvest",
        include_str!(
            "../../agency_experts/stock-analysis/strategy_packs/diversified-all-styles.yaml"
        ),
    ),
    (
        "value-growth-mix",
        "价值成长混合策略包",
        "价值投资 + 趋势跟踪混合，价值发现 + 趋势确认，GARP 风格",
        "1.0.0",
        "AxInvest",
        include_str!("../../agency_experts/stock-analysis/strategy_packs/value-growth-mix.yaml"),
    ),
    (
        "momentum-breakout",
        "动量突破策略包",
        "趋势突破 + 资金流入双确认，捕捉动量启动点，适合动量交易者",
        "1.0.0",
        "AxInvest",
        include_str!("../../agency_experts/stock-analysis/strategy_packs/momentum-breakout.yaml"),
    ),
    (
        "contrarian-deep-value",
        "逆向深度价值策略包",
        "深度价值 + 超跌反弹组合，逆向投资，适合熊市或调整行情",
        "1.0.0",
        "AxInvest",
        include_str!(
            "../../agency_experts/stock-analysis/strategy_packs/contrarian-deep-value.yaml"
        ),
    ),
    (
        "growth-momentum-long",
        "成长动量长线策略包",
        "长线趋势 + 长线价值 + 长线资金三重确认，聚焦成长股长线持有",
        "1.0.0",
        "AxInvest",
        include_str!(
            "../../agency_experts/stock-analysis/strategy_packs/growth-momentum-long.yaml"
        ),
    ),
    (
        "swing-trading",
        "波段交易策略包",
        "短线趋势 + 中线趋势 + 短线反弹结合，捕捉中短期波段，适合波段交易者",
        "1.0.0",
        "AxInvest",
        include_str!("../../agency_experts/stock-analysis/strategy_packs/swing-trading.yaml"),
    ),
];

/// 策略包清单响应（轻量，不含完整 spec）
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StrategyPackInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    /// 来源："builtin" | "user"
    pub source: String,
    /// 策略条目数
    pub strategy_count: usize,
    /// 启用条目数
    pub enabled_count: usize,
    /// 是否启用（用户包默认 true，可被前端切换）
    pub enabled: bool,
    /// 包级最低置信度
    pub min_confidence: u8,
    /// 包级最大推荐数
    pub max_picks: usize,
}

/// 策略包完整详情（含 spec）
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StrategyPackDetail {
    #[serde(flatten)]
    pub info: StrategyPackInfo,
    /// 完整 spec（YAML 解析后的结构化数据）
    pub spec: StrategyPackSpec,
    /// 转换后的 template_vars（供 recommend_stocks 消费）
    pub template_vars: serde_json::Value,
}

/// 用户策略包目录：`~/Documents/axagent/strategy_packs/`
fn user_packs_dir() -> PathBuf {
    axagent_storage::storage_paths::documents_root().join("strategy_packs")
}

/// 从内置 YAML 加载所有策略包
fn load_builtin_packs() -> Vec<(StrategyPackInfo, StrategyPackSpec)> {
    BUILTIN_PACKS
        .iter()
        .filter_map(|(id, name, desc, version, author, yaml)| {
            match StrategyPackSpec::from_yaml(yaml) {
                Ok(spec) => {
                    let info = StrategyPackInfo {
                        id: id.to_string(),
                        name: name.to_string(),
                        description: desc.to_string(),
                        version: version.to_string(),
                        author: author.to_string(),
                        source: BUILTIN_SOURCE.to_string(),
                        strategy_count: spec.strategies.len(),
                        enabled_count: spec.enabled_entries().count(),
                        enabled: true,
                        min_confidence: spec.min_confidence,
                        max_picks: spec.max_picks,
                    };
                    Some((info, spec))
                },
                Err(e) => {
                    tracing::error!("[strategy_pack] 内置策略包 {id} 解析失败: {e}");
                    None
                },
            }
        })
        .collect()
}

/// 从用户目录加载所有策略包
fn load_user_packs() -> Vec<(StrategyPackInfo, StrategyPackSpec)> {
    let dir = user_packs_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(), // 用户目录不存在不是错误
    };
    let mut result = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let is_yaml = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("yaml") || s.eq_ignore_ascii_case("yml"))
            .unwrap_or(false);
        if !is_yaml {
            continue;
        }
        match load_pack_from_file(&path) {
            Ok(pack) => {
                let id = pack.manifest.id.clone();
                let info = StrategyPackInfo {
                    id,
                    name: pack.manifest.name.clone(),
                    description: pack.manifest.description.clone(),
                    version: pack.manifest.version.clone(),
                    author: pack.manifest.author.clone(),
                    source: USER_SOURCE.to_string(),
                    strategy_count: pack.manifest.strategy_count,
                    enabled_count: pack.manifest.enabled_count,
                    enabled: pack.manifest.enabled,
                    min_confidence: pack.spec.min_confidence,
                    max_picks: pack.spec.max_picks,
                };
                result.push((info, pack.spec));
            },
            Err(e) => {
                tracing::warn!("[strategy_pack] 用户策略包 {} 加载失败: {e}", path.display());
            },
        }
    }
    result
}

/// 列出所有策略包（内置 + 用户）
#[agent_command(domain = invest, safety = Safe, call_mode = StateInput, description = "列出所有策略包")]
#[tauri::command]
pub async fn list_strategy_packs(
    _state: State<'_, AppState>,
) -> Result<Vec<StrategyPackInfo>, String> {
    let mut packs = load_builtin_packs();
    packs.extend(load_user_packs());
    // 按名称排序
    packs.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    Ok(packs.into_iter().map(|(info, _)| info).collect())
}

/// 获取策略包详情（含完整 spec 和 template_vars）
///
/// - `source` = "builtin" 时从内置加载
/// - `source` = "user" 时从用户目录加载
#[agent_command(domain = invest, safety = Safe, call_mode = StateInput, description = "获取策略包详情")]
#[tauri::command]
pub async fn get_strategy_pack_detail(
    _state: State<'_, AppState>,
    id: String,
    source: String,
) -> Result<StrategyPackDetail, String> {
    let (info, spec) = if source == BUILTIN_SOURCE {
        load_builtin_packs()
            .into_iter()
            .find(|(i, _)| i.id == id)
            .ok_or_else(|| format!("内置策略包不存在: {id}"))?
    } else if source == USER_SOURCE {
        load_user_packs()
            .into_iter()
            .find(|(i, _)| i.id == id)
            .ok_or_else(|| format!("用户策略包不存在: {id}"))?
    } else {
        return Err(format!("无效的来源: {source}（应为 builtin 或 user）"));
    };

    // 构造 LoadedStrategyPack 以获取 template_vars
    let pack = LoadedStrategyPack::from_spec(spec.clone(), &id).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("构造策略包失败: {e}"))
    })?;
    let template_vars: serde_json::Value = serde_json::to_value(
        pack.template_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<serde_json::Map<String, serde_json::Value>>(),
    )
    .map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("序列化 template_vars 失败: {e}"))
    })?;

    Ok(StrategyPackDetail { info, spec, template_vars })
}

/// 校验 YAML 策略包内容（不保存，仅返回解析结果或错误）
///
/// 前端编辑器实时校验用
#[agent_command(domain = invest, safety = Safe, call_mode = StateInput, description = "校验策略包YAML内容")]
#[tauri::command]
pub async fn validate_strategy_pack_yaml(
    _state: State<'_, AppState>,
    yaml: String,
) -> Result<StrategyPackManifest, String> {
    let spec = StrategyPackSpec::from_yaml(&yaml).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("解析 YAML 失败: {e}"))
    })?;
    spec.validate().map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("校验 spec 失败: {e}"))
    })?;
    let manifest = StrategyPackManifest {
        id: "preview".to_string(),
        name: spec.name.clone(),
        description: spec.description.clone(),
        version: spec.version.clone(),
        author: spec.author.clone(),
        strategy_count: spec.strategies.len(),
        enabled_count: spec.enabled_entries().count(),
        enabled: true,
        source: "preview".to_string(),
    };
    Ok(manifest)
}

/// 保存用户策略包到 `~/Documents/axagent/strategy_packs/<id>.yaml`
#[agent_command(domain = invest, safety = Caution, call_mode = StateInput, description = "保存用户策略包")]
#[tauri::command]
pub async fn save_user_strategy_pack(
    _state: State<'_, AppState>,
    id: String,
    yaml: String,
) -> Result<String, String> {
    // 先校验
    let spec = StrategyPackSpec::from_yaml(&yaml).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("解析 YAML 失败: {e}"))
    })?;
    spec.validate().map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("校验 spec 失败: {e}"))
    })?;

    let dir = user_packs_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {e}"))?;

    // 文件名用 id（清理非法字符）
    let safe_id: String = id
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let path = dir.join(format!("{safe_id}.yaml"));
    std::fs::write(&path, &yaml).map_err(|e| format!("写入文件失败: {e}"))?;
    Ok(path.display().to_string())
}

/// 删除用户策略包
#[agent_command(domain = invest, safety = Dangerous, call_mode = StateInput, description = "删除用户策略包")]
#[tauri::command]
pub async fn delete_user_strategy_pack(
    _state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let dir = user_packs_dir();
    let safe_id: String = id
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let path = dir.join(format!("{safe_id}.yaml"));
    if !path.exists() {
        return Err(format!("用户策略包不存在: {id}"));
    }
    std::fs::remove_file(&path).map_err(|e| format!("删除文件失败: {e}"))?;
    Ok(())
}

/// 获取用户策略包目录路径（前端用于打开文件夹）
#[agent_command(domain = invest, safety = Safe, call_mode = StateInput, description = "获取用户策略包目录路径")]
#[tauri::command]
pub async fn get_user_strategy_packs_dir(_state: State<'_, AppState>) -> Result<String, String> {
    let dir = user_packs_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {e}"))?;
    Ok(dir.display().to_string())
}

/// 内置策略包总数（前端展示用）
#[agent_command(domain = invest, safety = Safe, call_mode = StateInput, description = "统计内置策略包总数")]
#[tauri::command]
pub async fn count_builtin_strategy_packs(_state: State<'_, AppState>) -> Result<usize, String> {
    Ok(BUILTIN_PACKS.len())
}
