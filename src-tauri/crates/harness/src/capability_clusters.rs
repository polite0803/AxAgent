// SPDX-License-Identifier: AGPL-3.0-only
//! L2 能力集群清单 — 三层路由树中的第二层
//!
//! # 三层路由树架构
//! - L1: 域层(`CapabilityDomain` 硬划分,8 个业务域 + 1 个内部系统域)
//! - L2: 集群层(Domain 内功能集群,本模块定义,手动维护)
//! - L3: 具体能力层(来自索引)
//!
//! 集群是 Domain 内的"功能聚合",用于:
//! 1. 路由图(`RoutingGraph`)的邻接表节点
//! 2. 路径地址(`RoutingPath`)的中间段
//! 3. RAR 软引导 Prompt 的结构化输出
//!
//! # 集群 ID 约定
//! 格式: `{domain_as_str}_{path_segment}`,如 `general_file_ops`、`finance_market_data`。
//! 与 `CapabilityDomain::as_str()` + `path_segment` 拼接一致,保证确定性。
//!
//! # 数据排列
//! `all_clusters()` 中的集群按 `CapabilityDomain` 枚举顺序连续排列,
//! `clusters_by_domain` 借此直接返回静态子切片(零分配、零 leak)。

use crate::capability::CapabilityDomain;

// ── L2 集群定义 ──────────────────────────────────

/// L2 能力集群 — Domain 内的功能聚合
///
/// 字段全部使用 `&'static str`,保证 `all_clusters()` 可返回 `&'static [CapabilityCluster]`,
/// 零堆分配,符合 harness foundation 层零运行时开销原则。
#[derive(Debug, Clone, Copy)]
pub struct CapabilityCluster {
    /// 集群唯一 ID,格式 `{domain}_{cluster}`,如 `"general_file_ops"`
    pub cluster_id: &'static str,
    /// 所属业务域
    pub domain: CapabilityDomain,
    /// 集群中文名称,如 `"文件操作"`
    pub cluster_name: &'static str,
    /// 集群描述(中文)
    pub description: &'static str,
    /// 路径段(用于 `RoutingPath` 的 cluster 段),如 `"file_ops"`
    pub path_segment: &'static str,
    /// 关键词列表(用于从能力护照的 tags/name 推导所属集群)
    pub keywords: &'static [&'static str],
}

// 手动实现 serde，因为 &'static str/[&'static str] 无法派生 Deserialize
impl serde::Serialize for CapabilityCluster {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("CapabilityCluster", 6)?;
        s.serialize_field("cluster_id", self.cluster_id)?;
        s.serialize_field("domain", &self.domain)?;
        s.serialize_field("cluster_name", self.cluster_name)?;
        s.serialize_field("description", self.description)?;
        s.serialize_field("path_segment", self.path_segment)?;
        s.serialize_field("keywords", &self.keywords.to_vec())?;
        s.end()
    }
}

impl<'de> serde::Deserialize<'de> for CapabilityCluster {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct Helper {
            cluster_id: String,
            domain: CapabilityDomain,
            #[serde(default)]
            _cluster_name: Option<String>,
            #[serde(default)]
            _description: Option<String>,
            #[serde(default)]
            _path_segment: Option<String>,
            #[serde(default)]
            _keywords: Option<Vec<String>>,
        }
        let helper = Helper::deserialize(deserializer)?;
        // 查找匹配的静态集群
        let cluster = all_clusters()
            .iter()
            .find(|c| c.cluster_id == helper.cluster_id)
            .copied()
            .unwrap_or_else(|| {
                // 兜底：返回 domain 的第一个集群
                clusters_by_domain(helper.domain)
                    .first()
                    .copied()
                    .unwrap_or(all_clusters().first().copied().unwrap())
            });
        Ok(cluster)
    }
}

// ── 全量集群清单(手动维护) ────────────────────────

/// 全部 L2 集群清单(8 业务域 × 平均 3-4 个 = 27 个集群,不含内部 System 域)
///
/// **排列约定**:集群按 `CapabilityDomain` 枚举顺序连续排列,
/// `clusters_by_domain` 依赖此约定直接切片返回。
pub fn all_clusters() -> &'static [CapabilityCluster] {
    static CLUSTERS: &[CapabilityCluster] = &[
        // ── General 域（通用兜底，含原 Core 的必备工具集群）──
        CapabilityCluster {
            cluster_id: "general_file_ops",
            domain: CapabilityDomain::General,
            cluster_name: "文件操作",
            description: "文件读写、目录遍历、文件搜索等基础文件系统操作",
            path_segment: "file_ops",
            keywords: &["file", "read", "write", "glob", "grep", "list", "目录", "文件"],
        },
        CapabilityCluster {
            cluster_id: "general_text_ops",
            domain: CapabilityDomain::General,
            cluster_name: "文本处理",
            description: "字符串处理、文本编辑、正则匹配等文本操作",
            path_segment: "text_ops",
            keywords: &["text", "string", "字符串", "文本", "编辑", "正则", "regex"],
        },
        CapabilityCluster {
            cluster_id: "general_system_ops",
            domain: CapabilityDomain::General,
            cluster_name: "系统操作",
            description: "进程管理、系统信息、环境变量等系统级操作",
            path_segment: "system_ops",
            keywords: &["system", "process", "进程", "系统", "env", "环境变量", "shell"],
        },
        CapabilityCluster {
            cluster_id: "general_config_ops",
            domain: CapabilityDomain::General,
            cluster_name: "配置管理",
            description: "配置读写、设置管理、偏好存储",
            path_segment: "config_ops",
            keywords: &["config", "setting", "配置", "设置", "preference", "偏好"],
        },
        // ── General 域 ──
        CapabilityCluster {
            cluster_id: "general_search",
            domain: CapabilityDomain::General,
            cluster_name: "搜索",
            description: "网络搜索、知识检索、通用信息查询",
            path_segment: "search",
            keywords: &["search", "web", "搜索", "网络", "fetch", "检索", "query"],
        },
        CapabilityCluster {
            cluster_id: "general_summary",
            domain: CapabilityDomain::General,
            cluster_name: "摘要",
            description: "文本摘要、内容总结、信息抽取",
            path_segment: "summary",
            keywords: &["summary", "summarize", "摘要", "总结", "概括"],
        },
        CapabilityCluster {
            cluster_id: "general_translation",
            domain: CapabilityDomain::General,
            cluster_name: "翻译",
            description: "多语言翻译、语言转换",
            path_segment: "translation",
            keywords: &["translate", "translation", "翻译", "语言转换"],
        },
        // ── Devops 域 ──
        CapabilityCluster {
            cluster_id: "devops_deploy",
            domain: CapabilityDomain::Devops,
            cluster_name: "部署",
            description: "应用部署、发布、容器编排",
            path_segment: "deploy",
            keywords: &["deploy", "部署", "发布", "release", "docker", "k8s", "container"],
        },
        CapabilityCluster {
            cluster_id: "devops_monitor",
            domain: CapabilityDomain::Devops,
            cluster_name: "监控",
            description: "系统监控、指标采集、告警",
            path_segment: "monitor",
            keywords: &["monitor", "监控", "指标", "metric", "alert", "告警", "prometheus"],
        },
        CapabilityCluster {
            cluster_id: "devops_cicd",
            domain: CapabilityDomain::Devops,
            cluster_name: "持续集成",
            description: "CI/CD 流水线、构建、自动化测试",
            path_segment: "cicd",
            keywords: &["ci", "cd", "pipeline", "流水线", "构建", "build", "jenkins", "gitlab"],
        },
        // ── AiMedia 域 ──
        CapabilityCluster {
            cluster_id: "ai_media_image_gen",
            domain: CapabilityDomain::AiMedia,
            cluster_name: "图像生成",
            description: "AI 图像生成、图片处理、视觉创作",
            path_segment: "image_gen",
            keywords: &["image", "图片", "图", "生成图", "draw", "paint", "sd", "midjourney"],
        },
        CapabilityCluster {
            cluster_id: "ai_media_video_gen",
            domain: CapabilityDomain::AiMedia,
            cluster_name: "视频生成",
            description: "AI 视频生成、视频编辑",
            path_segment: "video_gen",
            keywords: &["video", "视频", "动画", "animation", "movie"],
        },
        CapabilityCluster {
            cluster_id: "ai_media_audio_gen",
            domain: CapabilityDomain::AiMedia,
            cluster_name: "音频生成",
            description: "语音合成、音频生成、TTS",
            path_segment: "audio_gen",
            keywords: &["audio", "音频", "语音", "tts", "speech", "声音", "music", "音乐"],
        },
        // ── Finance 域（业务标签 axinvest）──
        CapabilityCluster {
            cluster_id: "finance_market_data",
            domain: CapabilityDomain::Finance,
            cluster_name: "行情数据",
            description: "股票行情、市场数据、实时报价",
            path_segment: "market_data",
            keywords: &["market", "quote", "行情", "市场", "股价", "price", "ticker", "kline"],
        },
        CapabilityCluster {
            cluster_id: "finance_trading",
            domain: CapabilityDomain::Finance,
            cluster_name: "交易",
            description: "下单、撤单、交易执行",
            path_segment: "trading",
            keywords: &["trade", "trading", "交易", "下单", "order", "buy", "sell", "委托"],
        },
        CapabilityCluster {
            cluster_id: "finance_risk_control",
            domain: CapabilityDomain::Finance,
            cluster_name: "风控",
            description: "风险评估、仓位控制、止损止盈",
            path_segment: "risk_control",
            keywords: &["risk", "风控", "风险", "止损", "止盈", "exposure", "敞口"],
        },
        CapabilityCluster {
            cluster_id: "finance_portfolio",
            domain: CapabilityDomain::Finance,
            cluster_name: "组合管理",
            description: "持仓管理、投资组合、仓位分析",
            path_segment: "portfolio",
            keywords: &["portfolio", "持仓", "组合", "仓位", "position", "holding", "资产配置"],
        },
        // ── Automation 域（业务标签 axopc）──
        CapabilityCluster {
            cluster_id: "automation_workflow",
            domain: CapabilityDomain::Automation,
            cluster_name: "工作流自动化",
            description: "工作流自动化、RPA、任务编排",
            path_segment: "workflow",
            keywords: &["automation", "自动化", "workflow", "工作流", "rpa", "机器人"],
        },
        CapabilityCluster {
            cluster_id: "automation_schedule",
            domain: CapabilityDomain::Automation,
            cluster_name: "定时任务",
            description: "定时调度、Cron 任务、周期执行",
            path_segment: "schedule",
            keywords: &["schedule", "cron", "定时", "任务", "timer", "周期", "定时器"],
        },
        // ── DataAnalysis 域 ──
        CapabilityCluster {
            cluster_id: "data_analysis_query",
            domain: CapabilityDomain::DataAnalysis,
            cluster_name: "数据查询",
            description: "SQL 查询、数据检索、数据探索",
            path_segment: "query",
            keywords: &["query", "sql", "查询", "数据库", "database", "select"],
        },
        CapabilityCluster {
            cluster_id: "data_analysis_visualize",
            domain: CapabilityDomain::DataAnalysis,
            cluster_name: "数据可视化",
            description: "图表生成、数据可视化、报表",
            path_segment: "visualize",
            keywords: &["chart", "visualize", "图表", "可视化", "plot", "graph", "report", "报表"],
        },
        CapabilityCluster {
            cluster_id: "data_analysis_etl",
            domain: CapabilityDomain::DataAnalysis,
            cluster_name: "数据流水线",
            description: "ETL、数据清洗、数据转换",
            path_segment: "etl",
            keywords: &["etl", "extract", "transform", "load", "数据流", "清洗", "转换"],
        },
        // ── ContentCreation 域 ──
        CapabilityCluster {
            cluster_id: "content_creation_writing",
            domain: CapabilityDomain::ContentCreation,
            cluster_name: "写作",
            description: "文案撰写、文章生成、内容创作",
            path_segment: "writing",
            keywords: &["write", "writing", "写作", "文章", "文案", "内容", "content", "draft"],
        },
        CapabilityCluster {
            cluster_id: "content_creation_design",
            domain: CapabilityDomain::ContentCreation,
            cluster_name: "设计",
            description: "UI 设计、视觉设计、排版",
            path_segment: "design",
            keywords: &["design", "设计", "ui", "ux", "排版", "layout", "prototype"],
        },
        // ── Communication 域 ──
        CapabilityCluster {
            cluster_id: "communication_im",
            domain: CapabilityDomain::Communication,
            cluster_name: "即时通讯",
            description: "IM 消息、聊天、群组通信",
            path_segment: "im",
            keywords: &["im", "message", "消息", "聊天", "wechat", "slack", "telegram", "discord"],
        },
        CapabilityCluster {
            cluster_id: "communication_email",
            domain: CapabilityDomain::Communication,
            cluster_name: "邮件",
            description: "邮件发送、邮件管理",
            path_segment: "email",
            keywords: &["email", "mail", "邮件", "smtp", "imap"],
        },
        CapabilityCluster {
            cluster_id: "communication_notification",
            domain: CapabilityDomain::Communication,
            cluster_name: "通知",
            description: "推送通知、告警通知、消息推送",
            path_segment: "notification",
            keywords: &["notification", "notify", "通知", "推送", "push", "alert"],
        },
    ];
    CLUSTERS
}

// ── 查询函数 ──────────────────────────────────────

/// 按 Domain 过滤集群(返回该域下所有集群)
///
/// 利用 `all_clusters()` 中集群按 domain 顺序连续排列的约定,
/// 直接返回静态数据的子切片,零分配。
pub fn clusters_by_domain(domain: CapabilityDomain) -> &'static [CapabilityCluster] {
    let all = all_clusters();
    let start = all.iter().position(|c| c.domain == domain);
    let end = all.iter().rposition(|c| c.domain == domain);
    match (start, end) {
        (Some(s), Some(e)) => &all[s..=e],
        // 理论上不会发生(8 个业务域均有集群定义;System 域无集群,返回空切片),防御性返回空切片
        _ => &[],
    }
}

// ── 集群查找 ──────────────────────────────────────

/// 按 cluster_id 查找集群
pub fn find_cluster(cluster_id: &str) -> Option<&'static CapabilityCluster> {
    all_clusters().iter().find(|c| c.cluster_id == cluster_id)
}

/// 按路径段(path_segment)和 domain 查找集群
pub fn find_cluster_by_segment(
    domain: CapabilityDomain,
    path_segment: &str,
) -> Option<&'static CapabilityCluster> {
    all_clusters().iter().find(|c| c.domain == domain && c.path_segment == path_segment)
}

// ── 路径推导 ──────────────────────────────────────

/// 根据能力护照元数据推导所属集群(用于 `RoutingPath` 构造)
///
/// # 推导规则
/// 1. 取护照的 `domain`
/// 2. 在该 domain 的集群中,按 `tags` 和 `name` 关键词匹配
/// 3. 若无精确匹配,返回该 domain 的第一个集群作为 fallback
///    (注意:此处不返回 General.search,因为跨域 fallback 会破坏 L1 硬划分语义)
///
/// # 参数
/// - `domain`: 能力所属业务域
/// - `tags`: 能力护照的 tags 列表
/// - `name`: 能力名称(用于关键词匹配)
pub fn derive_cluster_for_passport(
    domain: CapabilityDomain,
    tags: &[String],
    name: &str,
) -> &'static CapabilityCluster {
    // 无任何集群定义时防御性 fallback 到 General.search
    let domain_clusters = clusters_by_domain(domain);
    if domain_clusters.is_empty() {
        return all_clusters()
            .iter()
            .find(|c| c.domain == CapabilityDomain::General)
            .expect("General 域必须至少有一个集群");
    }

    // 至少有命中则返回最佳匹配,否则 fallback 到该 domain 第一个集群
    best_cluster_by_keywords(domain, tags, name).unwrap_or(&domain_clusters[0])
}

/// 关键词评分:返回该 domain 下命中关键词数最高的集群;零命中返回 `None`
///
/// 供 [`derive_cluster_for_passport`](含 fallback)与
/// [`derive_cluster_from_query`](无命中返回 None)复用,避免重复评分逻辑。
pub(crate) fn best_cluster_by_keywords(
    domain: CapabilityDomain,
    tags: &[String],
    name: &str,
) -> Option<&'static CapabilityCluster> {
    let domain_clusters = clusters_by_domain(domain);
    if domain_clusters.is_empty() {
        return None;
    }

    // 合并 tags 和 name 作为匹配源(统一小写)
    let name_lower = name.to_lowercase();
    let tag_lowers: Vec<String> = tags.iter().map(|t| t.to_lowercase()).collect();

    // 遍历该 domain 下的集群,找关键词命中数最高的
    let mut best_match: Option<&CapabilityCluster> = None;
    let mut best_score: usize = 0;
    for cluster in domain_clusters {
        let mut score: usize = 0;
        for keyword in cluster.keywords {
            let kw_lower = keyword.to_lowercase();
            // tag 完全匹配(权重 2)
            if tag_lowers.iter().any(|t| t == &kw_lower) {
                score += 2;
            }
            // tag 包含匹配(权重 1)
            else if tag_lowers.iter().any(|t| t.contains(&kw_lower)) {
                score += 1;
            }
            // name 包含匹配(权重 1)
            if name_lower.contains(&kw_lower) {
                score += 1;
            }
        }
        if score > best_score {
            best_score = score;
            best_match = Some(cluster);
        }
    }

    best_match
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::CapabilityDomain;

    const BUSINESS_DOMAINS: [CapabilityDomain; 8] = [
        CapabilityDomain::General,
        CapabilityDomain::Devops,
        CapabilityDomain::AiMedia,
        CapabilityDomain::DataAnalysis,
        CapabilityDomain::ContentCreation,
        CapabilityDomain::Communication,
        CapabilityDomain::Finance,
        CapabilityDomain::Automation,
    ];

    #[test]
    fn test_every_business_domain_has_clusters() {
        for domain in BUSINESS_DOMAINS {
            let clusters = clusters_by_domain(domain);
            assert!(!clusters.is_empty(), "业务域 {} 必须至少有一个集群", domain.as_str());
        }
        // System 域为内部域,不应有可发现集群
        assert!(clusters_by_domain(CapabilityDomain::System).is_empty());
    }

    #[test]
    fn test_cluster_id_prefix_matches_domain() {
        for cluster in all_clusters() {
            let prefix = format!("{}_", cluster.domain.as_str());
            assert!(
                cluster.cluster_id.starts_with(&prefix),
                "集群 {} 的 ID 前缀必须等于其域 {} 的 as_str()",
                cluster.cluster_id,
                cluster.domain.as_str()
            );
        }
    }

    #[test]
    fn test_same_domain_clusters_are_contiguous() {
        let all = all_clusters();
        for domain in BUSINESS_DOMAINS {
            let positions: Vec<usize> = all
                .iter()
                .enumerate()
                .filter(|(_, c)| c.domain == domain)
                .map(|(i, _)| i)
                .collect();
            // 同域集群必须连续(允许该域仅一个集群)
            for w in positions.windows(2) {
                assert_eq!(
                    w[1],
                    w[0] + 1,
                    "域 {} 的集群必须连续排列,位置 {} 与 {} 不连续",
                    domain.as_str(),
                    w[0],
                    w[1]
                );
            }
        }
    }

    #[test]
    fn test_cluster_count() {
        assert_eq!(all_clusters().len(), 27);
    }
}
