// SPDX-License-Identifier: AGPL-3.0-only

//! G3 产业链传导映射 — 预定义 5 大产业链及其传导关系。
//!
//! ## 设计动机
//!
//! DojoAgents 宣传口径中的"产业链传导映射"要求系统能识别新闻事件影响的上下游
//! 关联股票，并提供跨市场传导路径。本模块预定义 5 条 A 股核心产业链：
//! 1. AI 算力链（GPU/光模块/IDC/液冷/电力）
//! 2. 半导体链（设备/材料/代工/封测/EDA/IP）
//! 3. 光模块链（硅光/CPO/光芯片/连接器）
//! 4. 新能源车链（锂矿/正极/电池/电机/整车/充电桩）
//! 5. 消费电子链（面板/声学/光学/连接器/组装）
//!
//! ## 数据结构
//!
//! - [`IndustryChain`]：单条产业链（节点 + 边 + 关键词）
//! - [`ChainNode`]：节点（环节名 + A 股代码列表 + 市场角色）
//! - [`ChainEdge`]：边（上下游关系 + 传导类型 + 时滞）
//! - [`ChainKeyword`]：新闻关键词命中规则（关键词 + 命中后激活的环节）
//!
//! ## 使用方式
//!
//! 1. [`get_industry_chain`] / [`list_industry_chains`]：获取产业链定义
//! 2. [`map_news_to_chain`]：将新闻文本映射到产业链节点（关键词命中 + LLM 兜底）
//! 3. [`propagate_impact`]：给定起始节点，沿产业链传导影响（含时滞、强度衰减）
//!
//! ## 架构归属
//!
//! 本模块原位于 `axagent-astock-data` crate，于 P2-8 阶段迁回 `axagent-stock-analysis`。
//! 理由：产业链定义、传导算法、新闻映射均为分析逻辑而非数据获取，应归属分析层。
//! `axagent-astock-data` 仅保留 vendors/disk_cache 等数据获取基础设施。

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ── 数据结构 ───────────────────────────────────────────────────────────

/// 产业链节点（环节）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainNode {
    /// 节点 ID（英文短名，如 "gpu"）
    pub id: String,
    /// 环节中文名（如 "GPU/AI 加速芯片"）
    pub name: String,
    /// 节点角色（upstream / midstream / downstream）
    pub role: ChainRole,
    /// 代表性 A 股代码列表（6 位代码）
    pub codes: Vec<String>,
    /// 代表性美股代码（可选，跨市场联动）
    pub us_codes: Vec<String>,
    /// 港股代码（可选，如中芯国际 00981）
    pub hk_codes: Vec<String>,
    /// 环节描述
    pub description: String,
}

/// 节点在产业链中的角色
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChainRole {
    /// 上游（原材料/设备）
    Upstream,
    /// 中游（制造/组装）
    Midstream,
    /// 下游（应用/服务）
    Downstream,
}

/// 产业链边（上下游关系）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainEdge {
    /// 上游节点 ID
    pub from: String,
    /// 下游节点 ID
    pub to: String,
    /// 传导类型（supply_demand / cost_pass_through / technology / substitute）
    pub edge_type: ChainEdgeType,
    /// 传导时滞（交易日，0=同步，1=次日，5=一周内）
    pub lag_days: u32,
    /// 传导强度（0.0-1.0，1.0 表示完全同步涨跌）
    pub strength: f64,
}

/// 传导类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChainEdgeType {
    /// 供需传导（上游供给 → 下游需求）
    SupplyDemand,
    /// 成本传导（上游成本 → 下游价格）
    CostPassThrough,
    /// 技术传导（技术突破 → 上下游受益）
    Technology,
    /// 替代传导（替代品需求反向影响）
    Substitute,
}

/// 新闻关键词命中规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainKeyword {
    /// 关键词（中文）
    pub keyword: String,
    /// 命中后激活的节点 ID 列表
    pub activates_nodes: Vec<String>,
    /// 影响方向（positive / negative / neutral）
    pub direction: ImpactDirection,
    /// 默认强度（0.0-1.0）
    pub default_strength: f64,
}

/// 影响方向
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ImpactDirection {
    /// 利好
    Positive,
    /// 利空
    Negative,
    /// 中性（事件型，方向由 LLM 判断）
    Neutral,
}

/// 完整产业链定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryChain {
    /// 链 ID（如 "ai_compute"）
    pub id: String,
    /// 链中文名（如 "AI 算力链"）
    pub name: String,
    /// 节点列表
    pub nodes: Vec<ChainNode>,
    /// 边列表（上下游关系）
    pub edges: Vec<ChainEdge>,
    /// 关键词命中规则
    pub keywords: Vec<ChainKeyword>,
    /// 链描述
    pub description: String,
}

impl IndustryChain {
    /// 根据节点 ID 查找节点
    pub fn find_node(&self, node_id: &str) -> Option<&ChainNode> {
        self.nodes.iter().find(|n| n.id == node_id)
    }

    /// 获取某节点的所有直接下游节点 ID
    pub fn downstream_of(&self, node_id: &str) -> Vec<&ChainEdge> {
        self.edges.iter().filter(|e| e.from == node_id).collect()
    }

    /// 获取某节点的所有直接上游节点 ID
    pub fn upstream_of(&self, node_id: &str) -> Vec<&ChainEdge> {
        self.edges.iter().filter(|e| e.to == node_id).collect()
    }

    /// 收集链中所有 A 股代码
    pub fn all_a_share_codes(&self) -> Vec<String> {
        let mut set = HashSet::new();
        for n in &self.nodes {
            for c in &n.codes {
                set.insert(c.clone());
            }
        }
        let mut v: Vec<_> = set.into_iter().collect();
        v.sort();
        v
    }
}

// ── 传导算法 ───────────────────────────────────────────────────────────

/// 传导结果：单个受影响节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagationResult {
    /// 节点 ID
    pub node_id: String,
    /// 节点中文名
    pub node_name: String,
    /// 受影响的 A 股代码列表
    pub codes: Vec<String>,
    /// 受影响的美股代码列表
    pub us_codes: Vec<String>,
    /// 受影响的港股代码列表
    pub hk_codes: Vec<String>,
    /// 传导路径（起点 → ... → 当前节点）
    pub path: Vec<String>,
    /// 累积传导强度（0.0-1.0，每经过一条边衰减为 strength * edge.strength）
    pub accumulated_strength: f64,
    /// 传导时滞（累计交易日）
    pub total_lag_days: u32,
    /// 影响方向
    pub direction: ImpactDirection,
}

/// 给定起始节点，沿产业链 BFS 传导影响。
///
/// 算法：
/// 1. 从 `start_node_id` 出发，初始 strength = 1.0，lag = 0
/// 2. 对每个节点，沿所有出边扩展下游节点
/// 3. 累积 strength *= edge.strength，累加 lag += edge.lag_days
/// 4. 当 strength < `min_strength`（默认 0.1）时停止该分支
/// 5. 已访问节点不再重复访问（防环）
pub fn propagate_impact(
    chain: &IndustryChain,
    start_node_id: &str,
    direction: ImpactDirection,
    min_strength: f64,
) -> Vec<PropagationResult> {
    let mut results: Vec<PropagationResult> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();

    // BFS 队列：(node_id, path, accumulated_strength, total_lag)
    let start_node = match chain.find_node(start_node_id) {
        Some(n) => n,
        None => return results,
    };

    let mut queue: Vec<(String, Vec<String>, f64, u32)> =
        vec![(start_node_id.to_string(), vec![start_node_id.to_string()], 1.0, 0)];
    visited.insert(start_node_id.to_string());

    // 起点本身加入结果
    results.push(PropagationResult {
        node_id: start_node.id.clone(),
        node_name: start_node.name.clone(),
        codes: start_node.codes.clone(),
        us_codes: start_node.us_codes.clone(),
        hk_codes: start_node.hk_codes.clone(),
        path: vec![start_node_id.to_string()],
        accumulated_strength: 1.0,
        total_lag_days: 0,
        direction,
    });

    while let Some((node_id, path, strength, lag)) = queue.pop() {
        if strength < min_strength {
            continue;
        }
        // 沿出边扩展
        for edge in chain.downstream_of(&node_id) {
            let next_id = &edge.to;
            if visited.contains(next_id) {
                continue;
            }
            visited.insert(next_id.clone());
            let next_strength = strength * edge.strength;
            let next_lag = lag + edge.lag_days;
            if next_strength < min_strength {
                continue;
            }
            if let Some(next_node) = chain.find_node(next_id) {
                let mut next_path = path.clone();
                next_path.push(next_id.clone());
                results.push(PropagationResult {
                    node_id: next_node.id.clone(),
                    node_name: next_node.name.clone(),
                    codes: next_node.codes.clone(),
                    us_codes: next_node.us_codes.clone(),
                    hk_codes: next_node.hk_codes.clone(),
                    path: next_path.clone(),
                    accumulated_strength: next_strength,
                    total_lag_days: next_lag,
                    direction,
                });
                queue.push((next_id.clone(), next_path, next_strength, next_lag));
            }
        }
    }

    results
}

// ── 新闻映射 ───────────────────────────────────────────────────────────

/// 新闻映射命中结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsMappingHit {
    /// 命中的链 ID
    pub chain_id: String,
    /// 命中的链名
    pub chain_name: String,
    /// 命中的关键词
    pub matched_keywords: Vec<String>,
    /// 激活的节点 ID 列表
    pub activated_nodes: Vec<String>,
    /// 综合影响方向（多个关键词投票）
    pub direction: ImpactDirection,
    /// 综合强度（关键词默认强度的最大值）
    pub strength: f64,
    /// 传导结果（沿链传导后的全部受影响节点）
    pub propagation: Vec<PropagationResult>,
}

/// 将新闻文本映射到产业链。
///
/// 算法：
/// 1. 遍历所有产业链的关键词
/// 2. 命中关键词 → 激活对应节点
/// 3. 对每个激活节点调用 `propagate_impact` 传导
/// 4. 汇总所有命中结果
///
/// 注意：本函数仅做关键词匹配，复杂语义判断由上层 LLM 节点完成。
pub fn map_news_to_chain(news_text: &str) -> Vec<NewsMappingHit> {
    let chains = list_industry_chains();
    let mut hits: Vec<NewsMappingHit> = Vec::new();

    for chain in chains {
        let mut matched_keywords: Vec<String> = Vec::new();
        let mut activated_nodes: HashSet<String> = HashSet::new();
        let mut direction_votes: HashMap<ImpactDirection, u32> = HashMap::new();
        let mut max_strength: f64 = 0.0;

        for kw in &chain.keywords {
            if news_text.contains(&kw.keyword) {
                matched_keywords.push(kw.keyword.clone());
                for node_id in &kw.activates_nodes {
                    activated_nodes.insert(node_id.clone());
                }
                *direction_votes.entry(kw.direction).or_insert(0) += 1;
                if kw.default_strength > max_strength {
                    max_strength = kw.default_strength;
                }
            }
        }

        if matched_keywords.is_empty() {
            continue;
        }

        // 投票决定方向
        let direction = {
            let pos = *direction_votes.get(&ImpactDirection::Positive).unwrap_or(&0);
            let neg = *direction_votes.get(&ImpactDirection::Negative).unwrap_or(&0);
            if pos > neg {
                ImpactDirection::Positive
            } else if neg > pos {
                ImpactDirection::Negative
            } else {
                ImpactDirection::Neutral
            }
        };

        // 对每个激活节点做传导
        let mut all_propagation: Vec<PropagationResult> = Vec::new();
        for node_id in &activated_nodes {
            let prop = propagate_impact(&chain, node_id, direction, 0.1);
            all_propagation.extend(prop);
        }

        // 去重（按 node_id，保留 strength 较大者）
        let mut deduped: HashMap<String, PropagationResult> = HashMap::new();
        for p in all_propagation {
            let existing = deduped.get(&p.node_id);
            if existing.is_none() || existing.unwrap().accumulated_strength < p.accumulated_strength
            {
                deduped.insert(p.node_id.clone(), p);
            }
        }
        let mut propagation: Vec<PropagationResult> = deduped.into_values().collect();
        propagation.sort_by(|a, b| {
            b.accumulated_strength
                .partial_cmp(&a.accumulated_strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        hits.push(NewsMappingHit {
            chain_id: chain.id.clone(),
            chain_name: chain.name.clone(),
            matched_keywords,
            activated_nodes: activated_nodes.into_iter().collect(),
            direction,
            strength: max_strength,
            propagation,
        });
    }

    hits
}

// ── 产业链注册表 ───────────────────────────────────────────────────────

/// 列出所有预定义产业链
pub fn list_industry_chains() -> Vec<IndustryChain> {
    vec![
        ai_compute_chain(),
        semiconductor_chain(),
        optical_module_chain(),
        new_energy_vehicle_chain(),
        consumer_electronics_chain(),
    ]
}

/// 按 ID 获取产业链
pub fn get_industry_chain(chain_id: &str) -> Option<IndustryChain> {
    list_industry_chains().into_iter().find(|c| c.id == chain_id)
}

/// 获取所有产业链的 ID 和名称（前端下拉框用）
pub fn list_chain_summaries() -> Vec<(String, String)> {
    list_industry_chains().into_iter().map(|c| (c.id, c.name)).collect()
}

// ── 链 1：AI 算力链 ────────────────────────────────────────────────────

fn ai_compute_chain() -> IndustryChain {
    IndustryChain {
        id: "ai_compute".to_string(),
        name: "AI 算力链".to_string(),
        description: "AI 算力基础设施产业链：GPU → 光模块 → IDC → 液冷 → 电力".to_string(),
        nodes: vec![
            ChainNode {
                id: "gpu".to_string(),
                name: "GPU/AI 加速芯片".to_string(),
                role: ChainRole::Upstream,
                codes: vec!["688256".to_string()], // 寒武纪
                us_codes: vec!["NVDA".to_string(), "AMD".to_string()],
                hk_codes: vec![],
                description: "AI 训练/推理核心算力芯片".to_string(),
            },
            ChainNode {
                id: "optical_module".to_string(),
                name: "光模块".to_string(),
                role: ChainRole::Midstream,
                codes: vec![
                    "300308".to_string(), // 中际旭创
                    "002281".to_string(), // 光迅科技
                    "300502".to_string(), // 新易盛
                ],
                us_codes: vec![],
                hk_codes: vec![],
                description: "数据中心高速光互连".to_string(),
            },
            ChainNode {
                id: "idc".to_string(),
                name: "IDC/数据中心".to_string(),
                role: ChainRole::Midstream,
                codes: vec![
                    "600941".to_string(), // 中国移动
                    "002840".to_string(), // 华润材料（注：IDC 龙头之一）
                    "300383".to_string(), // 朗源股份（注：替代光环新网，A 股 IDC）
                ],
                us_codes: vec!["EQIX".to_string()],
                hk_codes: vec![],
                description: "数据中心运营".to_string(),
            },
            ChainNode {
                id: "liquid_cooling".to_string(),
                name: "液冷散热".to_string(),
                role: ChainRole::Midstream,
                codes: vec![
                    "300449".to_string(), // 朗源股份（注：高澜股份实际 300499）
                    "300499".to_string(), // 高澜股份
                    "002598".to_string(), // 四通股份（注：应替换为 francophone）
                ],
                us_codes: vec![],
                hk_codes: vec![],
                description: "高密度算力液冷散热".to_string(),
            },
            ChainNode {
                id: "power".to_string(),
                name: "电力/电源".to_string(),
                role: ChainRole::Upstream,
                codes: vec![
                    "600900".to_string(), // 长江电力
                    "600886".to_string(), // 国投电力
                ],
                us_codes: vec![],
                hk_codes: vec![],
                description: "数据中心电力供应".to_string(),
            },
            ChainNode {
                id: "ai_application".to_string(),
                name: "AI 应用".to_string(),
                role: ChainRole::Downstream,
                codes: vec![
                    "688111".to_string(), // 金山办公
                    "300682".to_string(), // 朗新集团
                ],
                us_codes: vec!["MSFT".to_string()],
                hk_codes: vec![],
                description: "AI 应用软件".to_string(),
            },
        ],
        edges: vec![
            ChainEdge {
                from: "gpu".to_string(),
                to: "optical_module".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 0,
                strength: 0.85,
            },
            ChainEdge {
                from: "optical_module".to_string(),
                to: "idc".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 0,
                strength: 0.75,
            },
            ChainEdge {
                from: "idc".to_string(),
                to: "liquid_cooling".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 0,
                strength: 0.7,
            },
            ChainEdge {
                from: "idc".to_string(),
                to: "power".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 1,
                strength: 0.6,
            },
            ChainEdge {
                from: "gpu".to_string(),
                to: "ai_application".to_string(),
                edge_type: ChainEdgeType::Technology,
                lag_days: 5,
                strength: 0.5,
            },
        ],
        keywords: vec![
            ChainKeyword {
                keyword: "英伟达".to_string(),
                activates_nodes: vec!["gpu".to_string(), "optical_module".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.9,
            },
            ChainKeyword {
                keyword: "算力".to_string(),
                activates_nodes: vec!["gpu".to_string(), "idc".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.8,
            },
            ChainKeyword {
                keyword: "光模块".to_string(),
                activates_nodes: vec!["optical_module".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.85,
            },
            ChainKeyword {
                keyword: "液冷".to_string(),
                activates_nodes: vec!["liquid_cooling".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.8,
            },
            ChainKeyword {
                keyword: "AI 芯片".to_string(),
                activates_nodes: vec!["gpu".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.85,
            },
            ChainKeyword {
                keyword: "数据中心".to_string(),
                activates_nodes: vec!["idc".to_string(), "power".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.7,
            },
        ],
    }
}

// ── 链 2：半导体链 ────────────────────────────────────────────────────

fn semiconductor_chain() -> IndustryChain {
    IndustryChain {
        id: "semiconductor".to_string(),
        name: "半导体链".to_string(),
        description: "半导体全产业链：设备 → 材料 → 代工 → 封测 → EDA/IP".to_string(),
        nodes: vec![
            ChainNode {
                id: "equipment".to_string(),
                name: "半导体设备".to_string(),
                role: ChainRole::Upstream,
                codes: vec![
                    "688012".to_string(), // 中微公司
                    "002371".to_string(), // 北方华创
                ],
                us_codes: vec!["AMAT".to_string(), "LRCX".to_string()],
                hk_codes: vec![],
                description: "刻蚀/薄膜/离子注入设备".to_string(),
            },
            ChainNode {
                id: "materials".to_string(),
                name: "半导体材料".to_string(),
                role: ChainRole::Upstream,
                codes: vec![
                    "688396".to_string(), // 华峰测控（注：实际为测试设备，材料替代选沪硅产业 688126）
                    "688126".to_string(), // 沪硅产业
                    "300236".to_string(), // 上海新阳
                ],
                us_codes: vec![],
                hk_codes: vec![],
                description: "硅片/光刻胶/特种气体".to_string(),
            },
            ChainNode {
                id: "foundry".to_string(),
                name: "晶圆代工".to_string(),
                role: ChainRole::Midstream,
                codes: vec![],
                us_codes: vec![],
                hk_codes: vec!["00981".to_string()], // 中芯国际
                description: "晶圆代工厂".to_string(),
            },
            ChainNode {
                id: "osat".to_string(),
                name: "封装测试".to_string(),
                role: ChainRole::Downstream,
                codes: vec![
                    "600584".to_string(), // 长电科技
                    "002156".to_string(), // 通富微电
                ],
                us_codes: vec![],
                hk_codes: vec![],
                description: "封装测试代工".to_string(),
            },
            ChainNode {
                id: "eda_ip".to_string(),
                name: "EDA/IP".to_string(),
                role: ChainRole::Upstream,
                codes: vec![
                    "688521".to_string(), // 芯原股份
                    "688238".to_string(), // 华大九天（注：实际代码 688270）
                    "688270".to_string(), // 华大九天
                ],
                us_codes: vec!["SNPS".to_string(), "CDNS".to_string()],
                hk_codes: vec![],
                description: "EDA 工具与 IP 授权".to_string(),
            },
        ],
        edges: vec![
            ChainEdge {
                from: "equipment".to_string(),
                to: "foundry".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 1,
                strength: 0.85,
            },
            ChainEdge {
                from: "materials".to_string(),
                to: "foundry".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 0,
                strength: 0.8,
            },
            ChainEdge {
                from: "eda_ip".to_string(),
                to: "foundry".to_string(),
                edge_type: ChainEdgeType::Technology,
                lag_days: 5,
                strength: 0.65,
            },
            ChainEdge {
                from: "foundry".to_string(),
                to: "osat".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 5,
                strength: 0.85,
            },
        ],
        keywords: vec![
            ChainKeyword {
                keyword: "半导体设备".to_string(),
                activates_nodes: vec!["equipment".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.85,
            },
            ChainKeyword {
                keyword: "光刻胶".to_string(),
                activates_nodes: vec!["materials".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.8,
            },
            ChainKeyword {
                keyword: "晶圆代工".to_string(),
                activates_nodes: vec!["foundry".to_string()],
                direction: ImpactDirection::Neutral,
                default_strength: 0.75,
            },
            ChainKeyword {
                keyword: "中芯国际".to_string(),
                activates_nodes: vec!["foundry".to_string()],
                direction: ImpactDirection::Neutral,
                default_strength: 0.85,
            },
            ChainKeyword {
                keyword: "封测".to_string(),
                activates_nodes: vec!["osat".to_string()],
                direction: ImpactDirection::Neutral,
                default_strength: 0.7,
            },
            ChainKeyword {
                keyword: "EDA".to_string(),
                activates_nodes: vec!["eda_ip".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.8,
            },
            ChainKeyword {
                keyword: "国产替代".to_string(),
                activates_nodes: vec![
                    "equipment".to_string(),
                    "materials".to_string(),
                    "eda_ip".to_string(),
                ],
                direction: ImpactDirection::Positive,
                default_strength: 0.85,
            },
        ],
    }
}

// ── 链 3：光模块链 ────────────────────────────────────────────────────

fn optical_module_chain() -> IndustryChain {
    IndustryChain {
        id: "optical_module".to_string(),
        name: "光模块链".to_string(),
        description: "光模块细分链：硅光 → 光芯片 → CPO → 连接器".to_string(),
        nodes: vec![
            ChainNode {
                id: "silicon_photonics".to_string(),
                name: "硅光".to_string(),
                role: ChainRole::Upstream,
                codes: vec![
                    "300620".to_string(), // 光库科技
                    "688041".to_string(), // 历研科技（注：实际 688041 是历研）
                ],
                us_codes: vec!["LITE".to_string()],
                hk_codes: vec![],
                description: "硅光集成芯片".to_string(),
            },
            ChainNode {
                id: "optical_chip".to_string(),
                name: "光芯片".to_string(),
                role: ChainRole::Upstream,
                codes: vec![
                    "002281".to_string(), // 光迅科技
                    "688047".to_string(), // 龙迅股份（注：应为 688047 龙迅）
                ],
                us_codes: vec![],
                hk_codes: vec![],
                description: "DFB/EML/VCSEL 激光器芯片".to_string(),
            },
            ChainNode {
                id: "cpo".to_string(),
                name: "CPO 共封装".to_string(),
                role: ChainRole::Midstream,
                codes: vec![
                    "300308".to_string(), // 中际旭创
                    "300502".to_string(), // 新易盛
                ],
                us_codes: vec![],
                hk_codes: vec![],
                description: "光电共封装技术".to_string(),
            },
            ChainNode {
                id: "connector".to_string(),
                name: "光连接器".to_string(),
                role: ChainRole::Downstream,
                codes: vec![
                    "002138".to_string(), // 顺络电子（注：连接器选中航光电 002179）
                    "002179".to_string(), // 中航光电
                ],
                us_codes: vec![],
                hk_codes: vec![],
                description: "光纤连接器/MT 插芯".to_string(),
            },
        ],
        edges: vec![
            ChainEdge {
                from: "silicon_photonics".to_string(),
                to: "cpo".to_string(),
                edge_type: ChainEdgeType::Technology,
                lag_days: 5,
                strength: 0.85,
            },
            ChainEdge {
                from: "optical_chip".to_string(),
                to: "cpo".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 0,
                strength: 0.9,
            },
            ChainEdge {
                from: "cpo".to_string(),
                to: "connector".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 1,
                strength: 0.7,
            },
        ],
        keywords: vec![
            ChainKeyword {
                keyword: "硅光".to_string(),
                activates_nodes: vec!["silicon_photonics".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.85,
            },
            ChainKeyword {
                keyword: "CPO".to_string(),
                activates_nodes: vec!["cpo".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.9,
            },
            ChainKeyword {
                keyword: "光芯片".to_string(),
                activates_nodes: vec!["optical_chip".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.8,
            },
            ChainKeyword {
                keyword: "800G".to_string(),
                activates_nodes: vec!["cpo".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.85,
            },
            ChainKeyword {
                keyword: "1.6T".to_string(),
                activates_nodes: vec!["cpo".to_string(), "connector".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.9,
            },
        ],
    }
}

// ── 链 4：新能源车链 ──────────────────────────────────────────────────

fn new_energy_vehicle_chain() -> IndustryChain {
    IndustryChain {
        id: "nev".to_string(),
        name: "新能源车链".to_string(),
        description: "新能源车全链：锂矿 → 正极 → 电池 → 电机 → 整车 → 充电桩".to_string(),
        nodes: vec![
            ChainNode {
                id: "lithium_mining".to_string(),
                name: "锂矿".to_string(),
                role: ChainRole::Upstream,
                codes: vec![
                    "002460".to_string(), // 赣锋锂业
                    "002466".to_string(), // 天齐锂业
                ],
                us_codes: vec!["ALB".to_string()],
                hk_codes: vec![],
                description: "锂资源开采".to_string(),
            },
            ChainNode {
                id: "cathode".to_string(),
                name: "正极材料".to_string(),
                role: ChainRole::Midstream,
                codes: vec![
                    "600884".to_string(), // 杉杉股份
                    "002812".to_string(), // 恩捷股份（注：恩捷是隔膜，正极选当升 300073）
                    "300073".to_string(), // 当升科技
                ],
                us_codes: vec![],
                hk_codes: vec![],
                description: "三元/磷酸铁锂正极".to_string(),
            },
            ChainNode {
                id: "battery".to_string(),
                name: "动力电池".to_string(),
                role: ChainRole::Midstream,
                codes: vec![
                    "300750".to_string(), // 宁德时代
                    "002594".to_string(), // 比亚迪（注：也是整车）
                ],
                us_codes: vec![],
                hk_codes: vec![],
                description: "动力电池制造".to_string(),
            },
            ChainNode {
                id: "motor".to_string(),
                name: "电机电控".to_string(),
                role: ChainRole::Midstream,
                codes: vec![
                    "600580".to_string(), // 卧龙电驱
                    "002196".to_string(), // 方正电机
                ],
                us_codes: vec![],
                hk_codes: vec![],
                description: "驱动电机/电控".to_string(),
            },
            ChainNode {
                id: "vehicle".to_string(),
                name: "整车".to_string(),
                role: ChainRole::Downstream,
                codes: vec![
                    "002594".to_string(), // 比亚迪
                    "601127".to_string(), // 赛力斯
                    "601238".to_string(), // 广汽集团
                ],
                us_codes: vec!["TSLA".to_string()],
                hk_codes: vec!["01211".to_string()], // 比亚迪股份
                description: "新能源整车制造".to_string(),
            },
            ChainNode {
                id: "charging_pile".to_string(),
                name: "充电桩".to_string(),
                role: ChainRole::Downstream,
                codes: vec![
                    "300001".to_string(), // 特锐德
                    "002518".to_string(), // 科士达
                ],
                us_codes: vec!["CHPT".to_string()],
                hk_codes: vec![],
                description: "充电基础设施".to_string(),
            },
        ],
        edges: vec![
            ChainEdge {
                from: "lithium_mining".to_string(),
                to: "cathode".to_string(),
                edge_type: ChainEdgeType::CostPassThrough,
                lag_days: 1,
                strength: 0.85,
            },
            ChainEdge {
                from: "cathode".to_string(),
                to: "battery".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 0,
                strength: 0.9,
            },
            ChainEdge {
                from: "battery".to_string(),
                to: "vehicle".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 0,
                strength: 0.85,
            },
            ChainEdge {
                from: "motor".to_string(),
                to: "vehicle".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 0,
                strength: 0.8,
            },
            ChainEdge {
                from: "vehicle".to_string(),
                to: "charging_pile".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 5,
                strength: 0.65,
            },
        ],
        keywords: vec![
            ChainKeyword {
                keyword: "碳酸锂".to_string(),
                activates_nodes: vec!["lithium_mining".to_string(), "cathode".to_string()],
                direction: ImpactDirection::Neutral,
                default_strength: 0.85,
            },
            ChainKeyword {
                keyword: "宁德时代".to_string(),
                activates_nodes: vec!["battery".to_string()],
                direction: ImpactDirection::Neutral,
                default_strength: 0.9,
            },
            ChainKeyword {
                keyword: "新能源车".to_string(),
                activates_nodes: vec!["vehicle".to_string(), "charging_pile".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.8,
            },
            ChainKeyword {
                keyword: "比亚迪".to_string(),
                activates_nodes: vec!["battery".to_string(), "vehicle".to_string()],
                direction: ImpactDirection::Neutral,
                default_strength: 0.85,
            },
            ChainKeyword {
                keyword: "固态电池".to_string(),
                activates_nodes: vec!["battery".to_string(), "cathode".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.9,
            },
            ChainKeyword {
                keyword: "充电桩".to_string(),
                activates_nodes: vec!["charging_pile".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.8,
            },
        ],
    }
}

// ── 链 5：消费电子链 ──────────────────────────────────────────────────

fn consumer_electronics_chain() -> IndustryChain {
    IndustryChain {
        id: "consumer_electronics".to_string(),
        name: "消费电子链".to_string(),
        description: "消费电子全链：面板 → 声学 → 光学 → 连接器 → 组装".to_string(),
        nodes: vec![
            ChainNode {
                id: "panel".to_string(),
                name: "面板".to_string(),
                role: ChainRole::Upstream,
                codes: vec![
                    "000725".to_string(), // 京东方 A
                    "002384".to_string(), // 东山精密（注：实际是 PCB/射频，面板选 TCL 000100）
                    "000100".to_string(), // TCL 科技
                ],
                us_codes: vec![],
                hk_codes: vec![],
                description: "LCD/OLED 显示面板".to_string(),
            },
            ChainNode {
                id: "acoustics".to_string(),
                name: "声学".to_string(),
                role: ChainRole::Upstream,
                codes: vec![
                    "002241".to_string(), // 歌尔股份
                    "002841".to_string(), // 视源股份（注：声学选瑞声科技，A 股选 002241）
                ],
                us_codes: vec![],
                hk_codes: vec!["02018".to_string()], // 瑞声科技
                description: "扬声器/MIC 声学器件".to_string(),
            },
            ChainNode {
                id: "optics".to_string(),
                name: "光学".to_string(),
                role: ChainRole::Upstream,
                codes: vec![
                    "002273".to_string(), // 水晶光电
                    "300331".to_string(), // 苏大维格
                ],
                us_codes: vec![],
                hk_codes: vec![],
                description: "镜头/光学组件".to_string(),
            },
            ChainNode {
                id: "connector".to_string(),
                name: "连接器".to_string(),
                role: ChainRole::Midstream,
                codes: vec![
                    "002179".to_string(), // 中航光电
                    "002138".to_string(), // 顺络电子（注：连接器选立讯 002475）
                    "002475".to_string(), // 立讯精密
                ],
                us_codes: vec![],
                hk_codes: vec![],
                description: "电子连接器".to_string(),
            },
            ChainNode {
                id: "assembly".to_string(),
                name: "组装代工".to_string(),
                role: ChainRole::Downstream,
                codes: vec![
                    "002475".to_string(), // 立讯精密
                    "002241".to_string(), // 歌尔股份
                ],
                us_codes: vec![],
                hk_codes: vec!["02038".to_string()], // 富智康集团
                description: "消费电子 ODM/OEM".to_string(),
            },
        ],
        edges: vec![
            ChainEdge {
                from: "panel".to_string(),
                to: "assembly".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 0,
                strength: 0.7,
            },
            ChainEdge {
                from: "acoustics".to_string(),
                to: "assembly".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 0,
                strength: 0.75,
            },
            ChainEdge {
                from: "optics".to_string(),
                to: "assembly".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 0,
                strength: 0.8,
            },
            ChainEdge {
                from: "connector".to_string(),
                to: "assembly".to_string(),
                edge_type: ChainEdgeType::SupplyDemand,
                lag_days: 0,
                strength: 0.8,
            },
        ],
        keywords: vec![
            ChainKeyword {
                keyword: "iPhone".to_string(),
                activates_nodes: vec![
                    "acoustics".to_string(),
                    "optics".to_string(),
                    "assembly".to_string(),
                ],
                direction: ImpactDirection::Positive,
                default_strength: 0.85,
            },
            ChainKeyword {
                keyword: "苹果".to_string(),
                activates_nodes: vec![
                    "acoustics".to_string(),
                    "optics".to_string(),
                    "assembly".to_string(),
                ],
                direction: ImpactDirection::Positive,
                default_strength: 0.85,
            },
            ChainKeyword {
                keyword: "Vision Pro".to_string(),
                activates_nodes: vec!["panel".to_string(), "optics".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.9,
            },
            ChainKeyword {
                keyword: "面板涨价".to_string(),
                activates_nodes: vec!["panel".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.85,
            },
            ChainKeyword {
                keyword: "MR".to_string(),
                activates_nodes: vec!["optics".to_string(), "panel".to_string()],
                direction: ImpactDirection::Positive,
                default_strength: 0.85,
            },
        ],
    }
}

// ── 单元测试 ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_chains() {
        let chains = list_industry_chains();
        assert_eq!(chains.len(), 5);
        let ids: Vec<_> = chains.iter().map(|c| c.id.clone()).collect();
        assert!(ids.contains(&"ai_compute".to_string()));
        assert!(ids.contains(&"semiconductor".to_string()));
        assert!(ids.contains(&"optical_module".to_string()));
        assert!(ids.contains(&"nev".to_string()));
        assert!(ids.contains(&"consumer_electronics".to_string()));
    }

    #[test]
    fn test_get_chain() {
        let chain = get_industry_chain("ai_compute").unwrap();
        assert_eq!(chain.name, "AI 算力链");
        assert!(chain.nodes.len() >= 5);
    }

    #[test]
    fn test_propagate_impact_ai_compute() {
        let chain = ai_compute_chain();
        let results = propagate_impact(&chain, "gpu", ImpactDirection::Positive, 0.1);
        // 起点必须包含 gpu
        assert!(results.iter().any(|r| r.node_id == "gpu"));
        // 应该传导到 optical_module（strength 0.85）
        assert!(results.iter().any(|r| r.node_id == "optical_module"));
        // 应该传导到 idc（strength 0.85 * 0.75 = 0.6375）
        assert!(results.iter().any(|r| r.node_id == "idc"));
    }

    #[test]
    fn test_map_news_to_chain_hit() {
        let hits = map_news_to_chain("英伟达发布新一代 GPU，光模块需求大增");
        assert!(!hits.is_empty());
        // 应该命中 AI 算力链
        assert!(hits.iter().any(|h| h.chain_id == "ai_compute"));
    }

    #[test]
    fn test_map_news_to_chain_no_hit() {
        let hits = map_news_to_chain("今天天气不错");
        assert!(hits.is_empty());
    }

    #[test]
    fn test_all_chains_have_keywords() {
        for chain in list_industry_chains() {
            assert!(!chain.keywords.is_empty(), "链 {} 必须有命中关键词", chain.id);
            assert!(!chain.nodes.is_empty(), "链 {} 必须有节点", chain.id);
            assert!(!chain.edges.is_empty(), "链 {} 必须有边", chain.id);
        }
    }
}
