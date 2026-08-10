// SPDX-License-Identifier: AGPL-3.0-only

//! 叙事结构共享 DTO
//!
//! 本模块定义文学创作工作流中叙事结构的纯数据 DTO,
//! 包含角色弧线、交汇点、伏笔网络等核心叙事元素。
//!
//! 设计原则:仅 DTO + 构造器(builder 方法),不含业务逻辑。
//! 类型定义在 harness 层作为权威来源,供 rt-workflow、content_media 等模块复用。

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ── 叙事结构总览 ──

/// 叙事结构总览：包含所有弧线、交汇点和伏笔的完整定义
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeStructure {
    pub arcs: Vec<NarrativeArc>,
    pub confluences: Vec<ConfluencePoint>,
    pub foreshadows: Vec<Foreshadow>,
}

impl NarrativeStructure {
    pub fn new() -> Self {
        Self { arcs: Vec::new(), confluences: Vec::new(), foreshadows: Vec::new() }
    }

    pub fn with_arcs(mut self, arcs: Vec<NarrativeArc>) -> Self {
        self.arcs = arcs;
        self
    }

    pub fn with_confluences(mut self, confluences: Vec<ConfluencePoint>) -> Self {
        self.confluences = confluences;
        self
    }

    pub fn with_foreshadows(mut self, foreshadows: Vec<Foreshadow>) -> Self {
        self.foreshadows = foreshadows;
        self
    }

    /// 计算当前整体推进度（基于所有弧线的平均值）
    pub fn overall_progress(&self) -> f32 {
        if self.arcs.is_empty() {
            return 0.0;
        }
        let total: f32 = self.arcs.iter().map(|a| a.current_progress).sum();
        total / self.arcs.len() as f32
    }

    /// 获取指定章节的结构指令
    pub fn get_chapter_instructions(&self, chapter: u32) -> ChapterStructureInstruction {
        let mut arc_instructions = Vec::new();
        let mut foreshadow_instructions = Vec::new();
        let mut confluence_triggers = Vec::new();

        // 检查弧线阶段
        for arc in &self.arcs {
            for stage in &arc.stages {
                if stage.chapter == chapter {
                    arc_instructions.push(ArcInstruction {
                        arc_id: arc.id.clone(),
                        arc_type: arc.arc_type.clone(),
                        stage_name: stage.name.clone(),
                        stage_description: stage.description.clone(),
                    });
                }
            }
        }

        // 检查伏笔埋设/回收
        for fs in &self.foreshadows {
            if fs.setup_chapter == chapter {
                foreshadow_instructions.push(ForeshadowInstruction {
                    foreshadow_id: fs.id.clone(),
                    action: ForeshadowAction::Setup,
                    description: fs.description.clone(),
                });
            }
            if fs.payoff_chapter == Some(chapter) {
                foreshadow_instructions.push(ForeshadowInstruction {
                    foreshadow_id: fs.id.clone(),
                    action: ForeshadowAction::Payoff,
                    description: fs.payoff_description.clone().unwrap_or_default(),
                });
            }
        }

        // 检查交汇点触发
        for cp in &self.confluences {
            if cp.trigger_chapter == chapter {
                confluence_triggers.push(cp.clone());
            }
        }

        ChapterStructureInstruction {
            chapter,
            arc_instructions,
            foreshadow_instructions,
            confluence_triggers,
        }
    }
}

impl Default for NarrativeStructure {
    fn default() -> Self {
        Self::new()
    }
}

// ── 角色/主题弧线 ──

/// 弧线类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ArcType {
    /// 转换型：角色经历根本性变化
    Transformative,
    /// 坚定型：角色坚守信念并获得成长
    Steadfast,
    /// 扁平型：角色没有显著变化（作为对照组）
    Flat,
    /// 悲剧型：角色走向毁灭
    Tragic,
    /// 喜剧型：角色走向圆满
    Comedic,
}

/// 弧线阶段
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcStage {
    pub name: String,
    pub chapter: u32,
    pub description: String,
}

/// 角色/主题弧线
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeArc {
    pub id: String,
    pub arc_type: ArcType,
    pub subject: String,
    pub want: String,
    pub need: String,
    pub stages: Vec<ArcStage>,
    pub current_progress: f32,
}

impl NarrativeArc {
    pub fn new(id: String, arc_type: ArcType, subject: String) -> Self {
        Self {
            id,
            arc_type,
            subject,
            want: String::new(),
            need: String::new(),
            stages: Vec::new(),
            current_progress: 0.0,
        }
    }

    pub fn with_want(mut self, want: String) -> Self {
        self.want = want;
        self
    }

    pub fn with_need(mut self, need: String) -> Self {
        self.need = need;
        self
    }

    pub fn with_stages(mut self, stages: Vec<ArcStage>) -> Self {
        self.stages = stages;
        self
    }

    pub fn with_progress(mut self, progress: f32) -> Self {
        self.current_progress = progress.clamp(0.0, 100.0);
        self
    }

    /// 获取当前阶段名称
    pub fn current_stage(&self) -> Option<&ArcStage> {
        self.stages.first()
    }

    /// 更新推进度
    pub fn advance_progress(&mut self, delta: f32) {
        self.current_progress = (self.current_progress + delta).clamp(0.0, 100.0);
    }
}

// ── 交汇点 ──

/// 交汇点类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ConfluenceType {
    /// 冲突爆发：多条线索交汇产生激烈冲突
    ConflictBurst,
    /// 真相揭示：隐藏的真相被揭露
    RevealTruth,
    /// 视角转换：叙事视角发生重大转变
    ShiftPerspective,
}

/// 交汇点：多条线索/弧线在此汇聚、冲突或转折
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfluencePoint {
    pub id: String,
    pub trigger_chapter: u32,
    pub confluence_type: ConfluenceType,
    pub involved_arcs: Vec<String>,
    pub involved_foreshadows: Vec<String>,
    pub impact: String,
}

impl ConfluencePoint {
    pub fn new(id: String, trigger_chapter: u32, confluence_type: ConfluenceType) -> Self {
        Self {
            id,
            trigger_chapter,
            confluence_type,
            involved_arcs: Vec::new(),
            involved_foreshadows: Vec::new(),
            impact: String::new(),
        }
    }

    pub fn with_impact(mut self, impact: String) -> Self {
        self.impact = impact;
        self
    }

    pub fn with_involved_arcs(mut self, arcs: Vec<String>) -> Self {
        self.involved_arcs = arcs;
        self
    }

    pub fn with_involved_foreshadows(mut self, foreshadows: Vec<String>) -> Self {
        self.involved_foreshadows = foreshadows;
        self
    }
}

// ── 伏笔 ──

/// 伏笔状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ForeshadowStatus {
    /// 已埋设
    Setup,
    /// 已回收
    Payoff,
    /// 已废弃
    Abandoned,
}

/// 伏笔：追踪"埋设"与"回收"的完整生命周期
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Foreshadow {
    pub id: String,
    pub setup_chapter: u32,
    pub payoff_chapter: Option<u32>,
    pub status: ForeshadowStatus,
    pub description: String,
    pub payoff_description: Option<String>,
    pub related_arcs: Vec<String>,
}

impl Foreshadow {
    pub fn new(id: String, setup_chapter: u32, description: String) -> Self {
        Self {
            id,
            setup_chapter,
            payoff_chapter: None,
            status: ForeshadowStatus::Setup,
            description,
            payoff_description: None,
            related_arcs: Vec::new(),
        }
    }

    pub fn with_payoff(mut self, payoff_chapter: u32, payoff_description: String) -> Self {
        self.payoff_chapter = Some(payoff_chapter);
        self.payoff_description = Some(payoff_description);
        self
    }

    pub fn with_related_arcs(mut self, arcs: Vec<String>) -> Self {
        self.related_arcs = arcs;
        self
    }

    /// 标记为已回收
    pub fn mark_payoff(&mut self, chapter: u32, description: String) {
        self.payoff_chapter = Some(chapter);
        self.payoff_description = Some(description);
        self.status = ForeshadowStatus::Payoff;
    }

    /// 标记为已废弃
    pub fn mark_abandoned(&mut self) {
        self.status = ForeshadowStatus::Abandoned;
    }

    /// 检查是否应在指定章节埋设
    pub fn should_setup_in_chapter(&self, chapter: u32) -> bool {
        self.setup_chapter == chapter && self.status == ForeshadowStatus::Setup
    }

    /// 检查是否应在指定章节回收
    pub fn should_payoff_in_chapter(&self, chapter: u32) -> bool {
        self.payoff_chapter == Some(chapter) && self.status != ForeshadowStatus::Payoff
    }
}

// ── 章节结构指令 ──

/// 弧线指令
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcInstruction {
    pub arc_id: String,
    pub arc_type: ArcType,
    pub stage_name: String,
    pub stage_description: String,
}

/// 伏笔动作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ForeshadowAction {
    /// 埋设伏笔
    Setup,
    /// 回收伏笔
    Payoff,
}

/// 伏笔指令
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeshadowInstruction {
    pub foreshadow_id: String,
    pub action: ForeshadowAction,
    pub description: String,
}

/// 章节结构指令：为单章创作提供叙事约束
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterStructureInstruction {
    pub chapter: u32,
    pub arc_instructions: Vec<ArcInstruction>,
    pub foreshadow_instructions: Vec<ForeshadowInstruction>,
    pub confluence_triggers: Vec<ConfluencePoint>,
}

impl ChapterStructureInstruction {
    pub fn is_empty(&self) -> bool {
        self.arc_instructions.is_empty()
            && self.foreshadow_instructions.is_empty()
            && self.confluence_triggers.is_empty()
    }

    /// 生成 Prompt 用的结构约束文本
    pub fn to_prompt_constraints(&self) -> String {
        let mut constraints = Vec::new();

        if !self.arc_instructions.is_empty() {
            let arc_descriptions: Vec<String> = self
                .arc_instructions
                .iter()
                .map(|ai| {
                    format!("[{}] {} - {}", ai.arc_type_str(), ai.stage_name, ai.stage_description)
                })
                .collect();
            constraints
                .push(format!("【弧线推进】本章需推进以下弧线：{}", arc_descriptions.join("、")));
        }

        if !self.foreshadow_instructions.is_empty() {
            let fs_descriptions: Vec<String> = self
                .foreshadow_instructions
                .iter()
                .map(|fi| {
                    let action = match fi.action {
                        ForeshadowAction::Setup => "埋设伏笔",
                        ForeshadowAction::Payoff => "回收伏笔",
                    };
                    format!("（{}）{}", action, fi.description)
                })
                .collect();
            constraints.push(format!("【伏笔管理】本章需完成：{}", fs_descriptions.join("、")));
        }

        if !self.confluence_triggers.is_empty() {
            let cp_descriptions: Vec<String> = self
                .confluence_triggers
                .iter()
                .map(|cp| format!("【{}】{}", cp.confluence_type_str(), cp.impact))
                .collect();
            constraints.push(format!("【交汇点触发】本章关键事件：{}", cp_descriptions.join("、")));
        }

        if constraints.is_empty() {
            constraints.push("本章无特殊叙事结构约束，可自由推进剧情".to_string());
        }

        constraints.join("\n")
    }
}

// ── 辅助方法 ──

impl ArcType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ArcType::Transformative => "转换型",
            ArcType::Steadfast => "坚定型",
            ArcType::Flat => "扁平型",
            ArcType::Tragic => "悲剧型",
            ArcType::Comedic => "喜剧型",
        }
    }
}

impl ArcInstruction {
    pub fn arc_type_str(&self) -> &'static str {
        self.arc_type.as_str()
    }
}

impl ConfluenceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfluenceType::ConflictBurst => "冲突爆发",
            ConfluenceType::RevealTruth => "真相揭示",
            ConfluenceType::ShiftPerspective => "视角转换",
        }
    }
}

impl ConfluencePoint {
    pub fn confluence_type_str(&self) -> &'static str {
        self.confluence_type.as_str()
    }
}

impl ForeshadowStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ForeshadowStatus::Setup => "已埋设",
            ForeshadowStatus::Payoff => "已回收",
            ForeshadowStatus::Abandoned => "已废弃",
        }
    }
}

// ── 结构合规性检查结果 ──

/// 结构合规性检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureComplianceReport {
    pub chapter: u32,
    pub compliance_score: f32,
    pub arc_compliance: f32,
    pub foreshadow_compliance: f32,
    pub confluence_compliance: f32,
    pub deviations: Vec<StructureDeviation>,
    pub suggestions: Vec<String>,
}

impl StructureComplianceReport {
    pub fn new(chapter: u32) -> Self {
        Self {
            chapter,
            compliance_score: 0.0,
            arc_compliance: 0.0,
            foreshadow_compliance: 0.0,
            confluence_compliance: 0.0,
            deviations: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn is_critical(&self) -> bool {
        self.compliance_score < 50.0
    }

    pub fn needs_adjustment(&self) -> bool {
        self.compliance_score < 70.0
    }
}

/// 结构偏差
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureDeviation {
    pub deviation_type: DeviationType,
    pub description: String,
    pub affected_element: String,
    pub severity: DeviationSeverity,
}

/// 偏差类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DeviationType {
    /// 弧线推进偏离
    ArcDeviation,
    /// 伏笔未按时埋设
    ForeshadowSetupMissed,
    /// 伏笔未按时回收
    ForeshadowPayoffMissed,
    /// 交汇点未触发
    ConfluenceMissed,
    /// 叙事节奏问题
    PacingIssue,
}

/// 偏差严重程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DeviationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

// ── 动态调整建议 ──

/// 调整目标类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AdjustmentTargetType {
    Arc,
    Foreshadow,
    Confluence,
}

/// 结构调整建议
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureAdjustmentSuggestion {
    pub id: String,
    pub adjustment_type: AdjustmentType,
    pub description: String,
    pub affected_elements: Vec<String>,
    pub priority: AdjustmentPriority,
    pub rationale: String,
    /// 调整目标类型
    pub target_type: Option<AdjustmentTargetType>,
    /// 调整目标 ID
    pub target_id: Option<String>,
    /// 调整负载数据（如新增的阶段、伏笔等）
    pub payload: Option<JsonValue>,
}

/// 调整类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AdjustmentType {
    /// 延后伏笔回收
    DelayForeshadowPayoff,
    /// 提前伏笔回收
    AccelerateForeshadowPayoff,
    /// 增加弧线阶段
    AddArcStage,
    /// 调整弧线推进度
    AdjustArcProgress,
    /// 移动交汇点
    RepositionConfluence,
    /// 增加新伏笔
    AddForeshadow,
}

/// 调整优先级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AdjustmentPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_narrative_structure_new_is_empty() {
        let structure = NarrativeStructure::new();
        assert!(structure.arcs.is_empty());
        assert!(structure.confluences.is_empty());
        assert!(structure.foreshadows.is_empty());
    }

    #[test]
    fn test_narrative_structure_overall_progress_empty() {
        let structure = NarrativeStructure::new();
        assert_eq!(structure.overall_progress(), 0.0);
    }

    #[test]
    fn test_narrative_structure_overall_progress_calculation() {
        let structure = NarrativeStructure::new().with_arcs(vec![
            NarrativeArc::new("arc-1".into(), ArcType::Transformative, "主角".into())
                .with_progress(50.0),
            NarrativeArc::new("arc-2".into(), ArcType::Tragic, "反派".into()).with_progress(100.0),
        ]);
        assert_eq!(structure.overall_progress(), 75.0);
    }

    #[test]
    fn test_narrative_arc_advance_progress() {
        let mut arc = NarrativeArc::new("arc-1".into(), ArcType::Transformative, "主角".into());
        assert_eq!(arc.current_progress, 0.0);

        arc.advance_progress(30.0);
        assert_eq!(arc.current_progress, 30.0);

        arc.advance_progress(80.0);
        assert_eq!(arc.current_progress, 100.0); // clamped to max

        arc.advance_progress(-200.0);
        assert_eq!(arc.current_progress, 0.0); // clamped to min
    }

    #[test]
    fn test_narrative_arc_with_progress_clamps() {
        let arc = NarrativeArc::new("arc-1".into(), ArcType::Steadfast, "配角".into())
            .with_progress(150.0);
        assert_eq!(arc.current_progress, 100.0);

        let arc =
            NarrativeArc::new("arc-2".into(), ArcType::Flat, "对照组".into()).with_progress(-10.0);
        assert_eq!(arc.current_progress, 0.0);
    }

    #[test]
    fn test_foreshadow_lifecycle() {
        let mut fs = Foreshadow::new("fs-1".into(), 2, "神秘信件".into());
        assert_eq!(fs.status, ForeshadowStatus::Setup);
        assert!(fs.payoff_chapter.is_none());

        assert!(fs.should_setup_in_chapter(2));
        assert!(!fs.should_setup_in_chapter(3));
        assert!(!fs.should_payoff_in_chapter(8));

        fs.mark_payoff(8, "信件揭示真凶".into());
        assert_eq!(fs.status, ForeshadowStatus::Payoff);
        assert_eq!(fs.payoff_chapter, Some(8));
        assert!(fs.payoff_description.is_some());

        assert!(!fs.should_setup_in_chapter(2));
        assert!(!fs.should_payoff_in_chapter(8)); // already paid off
    }

    #[test]
    fn test_foreshadow_abandoned() {
        let mut fs = Foreshadow::new("fs-1".into(), 2, "废弃伏笔".into());
        fs.mark_abandoned();
        assert_eq!(fs.status, ForeshadowStatus::Abandoned);
    }

    #[test]
    fn test_get_chapter_instructions() {
        let structure = NarrativeStructure::new()
            .with_arcs(vec![
                NarrativeArc::new("arc-1".into(), ArcType::Transformative, "主角".into())
                    .with_stages(vec![ArcStage {
                        name: "转变".into(),
                        chapter: 3,
                        description: "角色觉醒".into(),
                    }]),
            ])
            .with_foreshadows(vec![Foreshadow::new("fs-1".into(), 3, "预言".into())])
            .with_confluences(vec![ConfluencePoint::new(
                "cp-1".into(),
                3,
                ConfluenceType::RevealTruth,
            )]);

        // Chapter with structure elements
        let inst = structure.get_chapter_instructions(3);
        assert_eq!(inst.chapter, 3);
        assert!(!inst.is_empty());
        assert_eq!(inst.arc_instructions.len(), 1);
        assert_eq!(inst.foreshadow_instructions.len(), 1);
        assert_eq!(inst.confluence_triggers.len(), 1);

        // Chapter without structure elements
        let inst = structure.get_chapter_instructions(5);
        assert!(inst.is_empty());
    }

    #[test]
    fn test_chapter_structure_instruction_empty_prompt() {
        let inst = ChapterStructureInstruction {
            chapter: 1,
            arc_instructions: vec![],
            foreshadow_instructions: vec![],
            confluence_triggers: vec![],
        };
        let prompt = inst.to_prompt_constraints();
        assert!(prompt.contains("本章无特殊叙事结构约束"));
    }

    #[test]
    fn test_structure_compliance_report_critical() {
        let mut report = StructureComplianceReport::new(1);
        assert!(report.is_critical()); // 0 < 50

        report.compliance_score = 60.0;
        assert!(!report.is_critical()); // 60 >= 50
        assert!(report.needs_adjustment()); // 60 < 70

        report.compliance_score = 80.0;
        assert!(!report.needs_adjustment()); // 80 >= 70
    }

    #[test]
    fn test_se_roundtrip() {
        let structure = NarrativeStructure::new().with_arcs(vec![
            NarrativeArc::new("arc-1".into(), ArcType::Transformative, "主角".into())
                .with_want("找到真相".into())
                .with_need("面对恐惧".into())
                .with_progress(50.0),
        ]);

        let json = serde_json::to_string(&structure).unwrap();
        let deserialized: NarrativeStructure = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.arcs.len(), 1);
        assert_eq!(deserialized.arcs[0].id, "arc-1");
        assert_eq!(deserialized.arcs[0].arc_type, ArcType::Transformative);
        assert_eq!(deserialized.arcs[0].current_progress, 50.0);
    }

    #[test]
    fn test_arc_type_as_str() {
        assert_eq!(ArcType::Transformative.as_str(), "转换型");
        assert_eq!(ArcType::Steadfast.as_str(), "坚定型");
        assert_eq!(ArcType::Flat.as_str(), "扁平型");
        assert_eq!(ArcType::Tragic.as_str(), "悲剧型");
        assert_eq!(ArcType::Comedic.as_str(), "喜剧型");
    }

    #[test]
    fn test_confluence_type_as_str() {
        assert_eq!(ConfluenceType::ConflictBurst.as_str(), "冲突爆发");
        assert_eq!(ConfluenceType::RevealTruth.as_str(), "真相揭示");
        assert_eq!(ConfluenceType::ShiftPerspective.as_str(), "视角转换");
    }

    #[test]
    fn test_foreshadow_status_as_str() {
        assert_eq!(ForeshadowStatus::Setup.as_str(), "已埋设");
        assert_eq!(ForeshadowStatus::Payoff.as_str(), "已回收");
        assert_eq!(ForeshadowStatus::Abandoned.as_str(), "已废弃");
    }
}
