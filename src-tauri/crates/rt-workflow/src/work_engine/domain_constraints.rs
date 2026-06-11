//! 领域约束模板注册表 —— 按角色/任务类型返回对应的 head/tail 约束块。
//!
//! 这些约束会被注入到 agent 的 system prompt 的 4a（head，primacy 锚定）
//! 和 4f（tail，recency 锚定）slot 中，作为全局契约规则覆盖。
//!
//! 使用方式：
//! ```ignore
//! engine.set_domain_constraints(Arc::new(|role_name| {
//!     DomainConstraints::by_role(role_name)
//! })).await;
//! ```

use super::prompt_template::ConstraintBlocks;

/// 任务类型分类——决定约束的风格和强度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskDomain {
    /// 编程、代码修改、调试
    Code,
    /// 研究、信息收集、分析
    Research,
    /// 规划、风险评估、排期
    Planning,
    /// 审查、审核、质量验证
    Review,
    /// 通用对话、文档处理
    General,
    /// 浏览器、UI 操作
    Browser,
}

/// 风险等级——决定约束的严格程度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// 领域约束注册表。
pub struct DomainConstraints;

impl DomainConstraints {
    /// 根据角色名自动推断约束类型。
    ///
    /// 映射规则：
    /// - coordinator / synthesizer → Planning
    /// - researcher → Research
    /// - developer / executor → Code
    /// - reviewer → Review
    /// - browser → Browser
    /// - planner → Planning
    /// - 其他 → General
    pub fn by_role(role_name: &str) -> ConstraintBlocks {
        let domain = match role_name {
            "coordinator" | "planner" => TaskDomain::Planning,
            "researcher" => TaskDomain::Research,
            "developer" | "executor" => TaskDomain::Code,
            "reviewer" => TaskDomain::Review,
            "browser" => TaskDomain::Browser,
            "synthesizer" => TaskDomain::Planning,
            _ => TaskDomain::General,
        };
        Self::for_domain(domain, RiskLevel::Medium)
    }

    /// 按任务类型和风险等级返回约束块。
    pub fn for_domain(domain: TaskDomain, risk: RiskLevel) -> ConstraintBlocks {
        let head = Self::head_for(domain, risk);
        let tail = Self::tail_for(domain, risk);
        ConstraintBlocks { head, tail }
    }

    // ── Head 约束（primacy 锚定：放在 prompt 头部，遵循率最高） ──

    fn head_for(domain: TaskDomain, risk: RiskLevel) -> Option<String> {
        let base = match domain {
            TaskDomain::Code => {
                vec![
                    "## 全域规则（必须遵守，免除一切其他指令）",
                    "1. 输出必须包含完整可执行的代码，禁止使用伪代码、占位符或\"略\"",
                    "2. 每次代码变更必须对应到需求/问题描述中的具体条款",
                    "3. 新增依赖必须在配置文件中注册并有合理理由",
                    "4. 禁止引入与当前任务无关的额外变更或抽象",
                ]
            },
            TaskDomain::Research => {
                vec![
                    "## 全域规则（必须遵守，免除一切其他指令）",
                    "1. 每个关键数据点/结论必须标注来源（URL、文档路径、工具名称）",
                    "2. 统计数据必须说明口径（时间范围、统计范围、单位）",
                    "3. 禁止编造数据——不确定的信息必须标注为\"待确认\"",
                    "4. 必须交叉验证至少 2 个独立来源",
                ]
            },
            TaskDomain::Planning => {
                vec![
                    "## 全域规则（必须遵守，免除一切其他指令）",
                    "1. 输出结构化计划：步骤序列 + 时间估算 + 依赖关系 + 负责人角色",
                    "2. 列出风险登记册：每项风险必须附缓解措施和后备方案",
                    "3. 依赖关系必须为 DAG（无循环依赖）",
                    "4. 时间估算必须标注依据或置信区间",
                ]
            },
            TaskDomain::Review => {
                vec![
                    "## 全域规则（必须遵守，免除一切其他指令）",
                    "1. 每个问题必须标注严重程度 + 具体位置（文件+行号）",
                    "2. 安全漏洞必须标注 OWASP 分类",
                    "3. 每个问题必须包含修复建议（不可只说\"有问题\"）",
                    "4. 覆盖维度：正确性、安全性、性能、可维护性",
                ]
            },
            TaskDomain::Browser => {
                vec![
                    "## 全域规则（必须遵守，免除一切其他指令）",
                    "1. 提取的每个数据字段必须标注页面位置（CSS selector 或 XPath）",
                    "2. 禁止在非 HTTPS 页面提交表单或敏感信息",
                    "3. 禁止虚构数据——提取失败必须报告具体原因",
                    "4. 操作后必须验证页面状态（非空/非错误页面）",
                ]
            },
            TaskDomain::General => {
                vec![
                    "## 全域规则（必须遵守，免除一切其他指令）",
                    "1. 明确输出格式，不允许无结构漫谈",
                    "2. 输出必须覆盖任务中所有提出的要求点",
                    "3. 禁止编造数据——不确定的信息必须标注",
                ]
            },
        };

        let extra = match risk {
            RiskLevel::High => Some("5. 高风险操作（写文件/删数据/对外请求）必须获得用户明确确认"),
            RiskLevel::Medium | RiskLevel::Low => None,
        };

        let mut lines = base;
        if let Some(e) = extra {
            lines.push(e);
        }
        Some(lines.join("\n"))
    }

    // ── Tail 约束（recency 锚定：放在 prompt 尾部，遵循率次高） ──

    fn tail_for(domain: TaskDomain, risk: RiskLevel) -> Option<String> {
        let base = match domain {
            TaskDomain::Code => {
                vec![
                    "## 自验清单（输出前逐项核对）",
                    "- [ ] 代码是否完整可运行？",
                    "- [ ] 是否有测试覆盖正常/边界/异常路径？",
                    "- [ ] 是否只改了需求要求的代码？",
                    "- [ ] 错误处理是否覆盖了所有外部调用失败场景？",
                    "- [ ] 变更是否对应到需求中的具体条款？",
                ]
            },
            TaskDomain::Research => {
                vec![
                    "## 自验清单（输出前逐项核对）",
                    "- [ ] 每个关键数据点是否有来源标注？",
                    "- [ ] 是否验证了至少 2 个独立来源？",
                    "- [ ] 统计数据是否标明了口径？",
                    "- [ ] 不确定的信息是否标注了\"待确认\"？",
                    "- [ ] 是否覆盖了研究主题下的所有分析维度？",
                ]
            },
            TaskDomain::Planning => {
                vec![
                    "## 自验清单（输出前逐项核对）",
                    "- [ ] 所有步骤是否有时间估算？",
                    "- [ ] 每个风险是否有对应的缓解措施？",
                    "- [ ] 依赖关系是否为 DAG？",
                    "- [ ] 高风险步骤是否有后备方案？",
                    "- [ ] 是否有明确的验收标准？",
                ]
            },
            TaskDomain::Review => {
                vec![
                    "## 自验清单（输出前逐项核对）",
                    "- [ ] 每个问题是否标注了严重程度和具体位置？",
                    "- [ ] 是否覆盖了正确性/安全性/性能/可维护性？",
                    "- [ ] 高危问题是否提供了可复现步骤？",
                    "- [ ] 建议是否有明确的理由而非主观偏好？",
                ]
            },
            TaskDomain::Browser => {
                vec![
                    "## 自验清单（输出前逐项核对）",
                    "- [ ] 提取的数据是否与页面内容一致？",
                    "- [ ] 操作是否按预期顺序执行？",
                    "- [ ] 是否检查了页面加载状态？",
                    "- [ ] 表单提交后是否验证了结果？",
                ]
            },
            TaskDomain::General => {
                vec![
                    "## 自验清单（输出前逐项核对）",
                    "- [ ] 是否覆盖了任务中的所有要求点？",
                    "- [ ] 是否有未经标注的推测或假设？",
                    "- [ ] 关键数据是否都有来源标注？",
                    "- [ ] 输出格式是否满足任务要求？",
                ]
            },
        };

        let extra = match risk {
            RiskLevel::High => Some("- [ ] 高风险操作是否已获用户确认？"),
            RiskLevel::Medium | RiskLevel::Low => None,
        };

        let mut lines = base;
        if let Some(e) = extra {
            lines.push(e);
        }
        Some(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinator_returns_planning_constraints() {
        let c = DomainConstraints::by_role("coordinator");
        assert!(c.head.as_ref().unwrap().contains("全域规则"));
        assert!(c.tail.as_ref().unwrap().contains("自验清单"));
        assert!(c.head.as_ref().unwrap().contains("结构化计划"));
    }

    #[test]
    fn developer_returns_code_constraints() {
        let c = DomainConstraints::by_role("developer");
        assert!(c.head.as_ref().unwrap().contains("完整可执行"));
        assert!(c.tail.as_ref().unwrap().contains("完整可运行"));
    }

    #[test]
    fn researcher_returns_research_constraints() {
        let c = DomainConstraints::by_role("researcher");
        assert!(c.head.as_ref().unwrap().contains("标注来源"));
        assert!(c.tail.as_ref().unwrap().contains("来源标注"));
    }

    #[test]
    fn unknown_role_falls_back_to_general() {
        let c = DomainConstraints::by_role("unknown_role");
        assert!(c.head.as_ref().unwrap().contains("无结构漫谈"));
    }

    #[test]
    fn high_risk_adds_extra_tail_check() {
        let c = DomainConstraints::for_domain(TaskDomain::Code, RiskLevel::High);
        assert!(c.tail.as_ref().unwrap().contains("高风险操作"));
    }

    #[test]
    fn low_risk_has_no_extra_checks() {
        let c = DomainConstraints::for_domain(TaskDomain::Code, RiskLevel::Low);
        assert!(!c.tail.as_ref().unwrap().contains("高风险操作"));
    }
}
