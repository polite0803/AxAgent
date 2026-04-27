# Phase 5: 个性化与持续学习 - 实施计划

> 文档版本: 1.0
> 创建日期: 2026-04-26
> 阶段周期: 2026-10-15 - 2027-01-01（2.5 个月）
> 基于版本: AxAgent 当前代码库基线

---

## 一、概述

### 1.1 阶段目标

建立深层次的用户偏好模型，实现个性化的 AI 助手体验。通过用户画像系统、风格迁移和主动助手能力，让 AxAgent 能够：

- 理解用户的编码风格、沟通偏好和工作习惯
- 自动适应用户的写作和代码风格
- 预测用户需求并提前准备相关资源和建议

### 1.2 核心模块

| 模块 | 描述 | 优先级 |
|------|------|--------|
| 模块 7 | 用户画像系统 | P0 |
| 模块 8 | 风格迁移引擎 | P0 |
| 模块 9 | 主动助手能力 | P1 |

### 1.3 技术挑战

1. **隐私与个性化平衡**：在提供个性化体验的同时保护用户隐私
2. **风格量化**：将主观的风格特征转化为可计算的向量表示
3. **预测准确性**：构建可靠的上下文预测模型
4. **实时适应**：在不过度干扰用户工作流的情况下学习偏好

---

## 二、模块 7: 用户画像系统

### 2.1 概述

构建全面的用户画像系统，从多个维度理解用户的偏好和行为模式。

### 2.2 当前基线

现有代码库状态：

- `src/stores/feature/userProfileStore.ts` - 仅包含基础用户信息（name, avatar）
- `src/stores/domain/preferenceStore.ts` - 包含会话级偏好（搜索、thinking budget 等）
- `src/stores/feature/memoryStore.ts` - 通用记忆存储，无结构化偏好提取
- 无 Rust 后端偏好学习模块

### 2.3 架构设计

#### 2.3.1 Rust 后端模块

新建 `src-tauri/crates/trajectory/` crate：

```
src-tauri/crates/trajectory/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── user_profile.rs      # 用户画像核心结构
    ├── preference_learner.rs # 偏好学习引擎
    ├── behavior_tracker.rs   # 行为追踪
    ├── pattern_analyzer.rs   # 模式分析
    └── storage.rs            # 画像持久化
```

#### 2.3.2 用户画像数据结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // 编码风格维度
    pub coding_style: CodingStyleProfile,

    // 沟通偏好维度
    pub communication: CommunicationProfile,

    // 工作习惯维度
    pub work_habits: WorkHabitProfile,

    // 领域知识维度
    pub domain_knowledge: DomainKnowledgeProfile,

    // 学习状态
    pub learning_state: LearningState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingStyleProfile {
    pub naming_conventions: NamingConvention,
    pub code_patterns: Vec<CodePattern>,
    pub framework_preferences: Vec<String>,
    pub indentation_style: IndentationStyle,
    pub comment_style: CommentStyle,
    pub module_organization: ModuleOrgStyle,
    pub confidence: f32, // 0.0 - 1.0, 学习置信度
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationProfile {
    pub detail_level: DetailLevel, // minimal, moderate, comprehensive
    pub tone: Tone,                // formal, neutral, casual
    pub format_preference: FormatPreference,
    pub language: String,
    pub response_length_pref: ResponseLength,
    pub explanation_depth: ExplanationDepth,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkHabitProfile {
    pub active_hours: TimeRange,      // 高效工作时段
    pub task_preferences: Vec<TaskType>,
    pub tool_usage_patterns: Vec<ToolUsagePattern>,
    pub workflow_preferences: WorkflowPreference,
    pub context_switch_tolerance: f32, // 0.0 - 1.0
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainKnowledgeProfile {
    pub expertise_areas: Vec<ExpertiseArea>,
    pub interest_topics: Vec<String>,
    pub skill_levels: HashMap<String, SkillLevel>,
    pub recent_topics: Vec<RecentTopic>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningState {
    pub total_interactions: u64,
    pub last_updated: DateTime<Utc>,
    pub learning_version: u32,
    pub stability_score: f32,        // 画像稳定性
    pub freshness_score: f32,        // 画像时效性
    pub explicitly_set: HashSet<String>, // 用户显式设置的偏好
}
```

### 2.4 实现细节

#### 2.4.1 行为追踪 (behavior_tracker.rs)

追踪用户交互行为，提取有意义的模式：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorEvent {
    pub event_type: BehaviorEventType,
    pub timestamp: DateTime<Utc>,
    pub context: EventContext,
    pub metadata: HashMap<String, String>,
    pub interaction_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BehaviorEventType {
    CodeGeneration {
        language: String,
        framework: Option<String>,
        line_count: u32,
        has_tests: bool,
    },
    SearchQuery {
        query_type: String,
        result_count: u32,
        clicked_result: Option<u32>,
    },
    ArtifactCreation {
        artifact_type: String,
        complexity: f32,
    },
    ConversationStart {
        intent: Option<String>,
    },
    ToolUsage {
        tool_name: String,
        success: bool,
        duration_ms: u64,
    },
    FeedbackGiven {
        feedback_type: FeedbackType,
        rating: Option<i32>,
    },
    PreferenceSet {
        setting_key: String,
        old_value: Option<String>,
        new_value: String,
    },
}
```

#### 2.4.2 偏好学习器 (preference_learner.rs)

从行为事件中学习用户偏好：

```rust
pub struct PreferenceLearner {
    profile: UserProfile,
    event_buffer: Vec<BehaviorEvent>,
    analyzer: PatternAnalyzer,
}

impl PreferenceLearner {
    pub fn process_event(&mut self, event: BehaviorEvent) -> ProfileUpdate {
        // 1. 更新事件缓冲区
        self.event_buffer.push(event);

        // 2. 提取模式
        let patterns = self.analyzer.extract_patterns(&self.event_buffer);

        // 3. 更新画像
        self.update_profile(patterns)
    }

    pub fn infer_coding_style(&self, samples: &[CodeSample]) -> CodingStyleProfile {
        // 使用 LLM 或规则分析代码样本
    }

    pub fn infer_communication_style(&self, samples: &[Message]) -> CommunicationProfile {
        // 分析历史消息
    }
}
```

### 2.5 前端集成

#### 2.5.1 扩展 userProfileStore

```typescript
// src/stores/feature/userProfileStore.ts

interface ExtendedUserProfile {
  // 基础信息
  name: string;
  avatarType: AvatarType;
  avatarValue: string;

  // 个性化偏好
  preferences: {
    coding: CodingPreferences;
    communication: CommunicationPreferences;
    workHabits: WorkHabitPreferences;
  };

  // 学习状态
  learning: {
    isEnabled: boolean;
    lastSynced: string;
    confidence: number;
    explicitlySet: string[];
  };
}

interface CodingPreferences {
  namingConvention: 'camelCase' | 'snake_case' | 'PascalCase' | 'kebab-case';
  indentationSize: number;
  useSemicolons: boolean;
  quoteStyle: 'single' | 'double';
  bracketStyle: 'same-line' | 'new-line';
}

interface CommunicationPreferences {
  detailLevel: 'minimal' | 'moderate' | 'comprehensive';
  tone: 'formal' | 'neutral' | 'casual';
  includeExplanations: boolean;
  responseLength: 'short' | 'medium' | 'long';
}
```

#### 2.5.2 新增组件

```
src/components/profile/
├── UserProfilePanel.tsx      # 用户画像总览面板
├── CodingStyleCard.tsx       # 编码风格显示/编辑
├── CommunicationPrefsCard.tsx # 沟通偏好显示/编辑
├── WorkHabitInsights.tsx     # 工作习惯洞察
├── DomainExpertiseCard.tsx   # 领域知识展示
├── LearningSettings.tsx       # 学习功能设置
└── ProfileSyncIndicator.tsx  # 同步状态指示器
```

### 2.6 数据库变更

新增 migration: `m20261015_000001_add_user_profiles.rs`

```sql
-- 用户画像主表
CREATE TABLE user_profiles (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- 编码风格 (JSON)
    coding_style JSONB NOT NULL DEFAULT '{}',

    -- 沟通偏好 (JSON)
    communication JSONB NOT NULL DEFAULT '{}',

    -- 工作习惯 (JSON)
    work_habits JSONB NOT NULL DEFAULT '{}',

    -- 领域知识 (JSON)
    domain_knowledge JSONB NOT NULL DEFAULT '{}',

    -- 学习状态 (JSON)
    learning_state JSONB NOT NULL DEFAULT '{}',

    -- 设置版本
    version INTEGER NOT NULL DEFAULT 1
);

-- 行为事件表
CREATE TABLE behavior_events (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT now(),
    context JSONB NOT NULL DEFAULT '{}',
    metadata JSONB NOT NULL DEFAULT '{}',
    interaction_id TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 用户显式偏好设置表
CREATE TABLE explicit_preferences (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL,
    category TEXT NOT NULL, -- 'coding', 'communication', 'work_habit', 'domain'
    setting_key TEXT NOT NULL,
    value JSONB NOT NULL,
    set_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE(user_id, category, setting_key)
);

-- 索引
CREATE INDEX idx_user_profiles_user_id ON user_profiles(user_id);
CREATE INDEX idx_behavior_events_user_id ON behavior_events(user_id);
CREATE INDEX idx_behavior_events_timestamp ON behavior_events(timestamp);
CREATE INDEX idx_explicit_preferences_user_id ON explicit_preferences(user_id);
```

### 2.7 文件清单

| 文件路径 | 描述 | 操作 |
|---------|------|------|
| `src-tauri/crates/trajectory/Cargo.toml` | 新建 crate 配置 | 新建 |
| `src-tauri/crates/trajectory/src/lib.rs` | 模块入口 | 新建 |
| `src-tauri/crates/trajectory/src/user_profile.rs` | 用户画像结构 | 新建 |
| `src-tauri/crates/trajectory/src/preference_learner.rs` | 偏好学习器 | 新建 |
| `src-tauri/crates/trajectory/src/behavior_tracker.rs` | 行为追踪 | 新建 |
| `src-tauri/crates/trajectory/src/pattern_analyzer.rs` | 模式分析 | 新建 |
| `src-tauri/crates/trajectory/src/storage.rs` | 持久化 | 新建 |
| `src-tauri/crates/trajectory/src/lib.rs` | 模块导出更新 | 修改 |
| `src-tauri/crates/core/src/entity/user_profile.rs` | 数据库实体 | 新建 |
| `src-tauri/crates/core/src/repo/user_profile.rs` | Repository | 新建 |
| `src-tauri/crates/core/src/repo/behavior_event.rs` | Repository | 新建 |
| `src-tauri/crates/core/src/lib.rs` | 导出更新 | 修改 |
| `src-tauri/crates/agent/src/lib.rs` | Agent 集成 | 修改 |
| `src/stores/feature/userProfileStore.ts` | 扩展前端 store | 修改 |
| `src/stores/feature/preferenceStore.ts` | 整合偏好 store | 修改 |
| `src/components/profile/*.tsx` | UI 组件 | 新建 |
| `src/types/profile.ts` | TypeScript 类型 | 新建 |
| `migrations/xxx_add_user_profiles.rs` | 数据库迁移 | 新建 |

### 2.8 验收标准

- [ ] 用户画像数据结构完整定义
- [ ] 行为追踪系统记录关键事件
- [ ] 偏好学习器能从事件中提取模式
- [ ] 前端能展示和编辑用户偏好
- [ ] 画像数据能持久化存储
- [ ] 与现有 Agent 系统集成

---

## 三、模块 8: 风格迁移引擎

### 3.1 概述

自动分析并适应用户的写作和编码风格，将学到的风格应用到新生成的内容中。

### 3.2 当前基线

无风格迁移功能。现有代码库仅有：

- 基础 Artifact 渲染
- 简单的模板替换机制
- 无风格向量表示

### 3.3 架构设计

#### 3.3.1 Rust 模块

```
src-tauri/crates/trajectory/src/
├── style_migrator.rs      # 风格迁移核心
├── style_vectorizer.rs    # 风格向量化
├── style_extractor.rs     # 风格特征提取
└── style_applier.rs       # 风格应用
```

#### 3.3.2 核心数据结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleVector {
    pub dimensions: StyleDimensions,
    pub source_confidence: f32,
    pub learned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleDimensions {
    // 编码风格维度
    pub naming_score: f32,           // 0.0 = snake_case, 1.0 = camelCase
    pub density_score: f32,          // 0.0 = compact, 1.0 = spacious
    pub comment_ratio: f32,          // 注释密度
    pub abstraction_level: f32,      // 抽象程度

    // 文档风格维度
    pub formality_score: f32,        // 0.0 = casual, 1.0 = formal
    pub structure_score: f32,        // 结构化程度
    pub technical_depth: f32,        // 技术深度
    pub explanation_length: f32,     // 解释详略
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeStyleTemplate {
    pub name: String,
    pub patterns: Vec<StylePattern>,
    pub templates: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylePattern {
    pub pattern_type: PatternType,
    pub original: String,
    pub transformed: String,
    pub context: String, // 使用场景
    pub usage_count: u32,
}
```

### 3.4 实现细节

#### 3.4.1 风格向量化 (style_vectorizer.rs)

将提取的风格特征转换为向量表示：

```rust
impl StyleVectorizer {
    pub fn from_coding_samples(&self, samples: &[CodeSample]) -> StyleVector {
        // 1. 统计命名模式
        let naming_score = self.analyze_naming_patterns(samples);

        // 2. 分析代码密度
        let density_score = self.analyze_density(samples);

        // 3. 计算注释比例
        let comment_ratio = self.analyze_comment_ratio(samples);

        // 4. 评估抽象级别
        let abstraction_level = self.analyze_abstraction(samples);

        StyleVector {
            dimensions: StyleDimensions {
                naming_score,
                density_score,
                comment_ratio,
                abstraction_level,
                formality_score: 0.5, // 代码风格默认中间值
                structure_score: 0.5,
                technical_depth: 0.5,
                explanation_length: 0.5,
            },
            source_confidence: self.calculate_confidence(samples),
            learned_at: Utc::now(),
        }
    }

    pub fn from_messages(&self, messages: &[Message]) -> StyleVector {
        // 分析文档/消息风格
    }

    pub fn to_embedding(&self) -> Vec<f32> {
        // 转换为用于相似度计算的 embedding
    }
}
```

#### 3.4.2 风格提取器 (style_extractor.rs)

从用户历史内容中提取风格特征：

```rust
impl StyleExtractor {
    pub fn extract_code_patterns(&self, samples: &[CodeSample]) -> Vec<CodeStylePattern> {
        let mut patterns = Vec::new();

        // 提取函数命名模式
        patterns.extend(self.extract_function_patterns(samples));

        // 提取变量命名模式
        patterns.extend(self.extract_variable_patterns(samples));

        // 提取代码结构模式
        patterns.extend(self.extract_structure_patterns(samples));

        // 提取注释模式
        patterns.extend(self.extract_comment_patterns(samples));

        patterns
    }

    pub fn extract_naming_conventions(&self, samples: &[CodeSample]) -> NamingConvention {
        // 分析命名约定
    }

    pub fn extract_formatting_preferences(&self, samples: &[CodeSample]) -> FormattingPrefs {
        // 分析格式偏好
    }
}
```

#### 3.4.3 风格应用器 (style_applier.rs)

将目标风格应用到生成的内容：

```rust
impl StyleApplier {
    pub fn apply_code_style(
        &self,
        code: &str,
        target_style: &StyleVector,
    ) -> String {
        // 1. 解析代码 AST
        let ast = self.parse_code(code);

        // 2. 应用命名转换
        let ast = self.apply_naming_transforms(ast, target_style);

        // 3. 应用格式转换
        let ast = self.apply_formatting_transforms(ast, target_style);

        // 4. 应用结构偏好
        let ast = self.apply_structure_preferences(ast, target_style);

        // 5. 序列化回代码
        self.serialize(ast)
    }

    pub fn apply_document_style(
        &self,
        content: &str,
        target_style: &StyleVector,
    ) -> String {
        // 应用于文档内容
    }

    fn apply_naming_transforms(
        &self,
        ast: Ast,
        style: &StyleVector,
    ) -> Ast {
        // 根据命名偏好转换标识符
    }
}
```

### 3.5 前端集成

#### 3.5.1 新增组件

```
src/components/style/
├── StylePreviewPanel.tsx    # 风格预览面板
├── StyleComparison.tsx      # 风格对比组件
├── CodeStyleSample.tsx      # 代码风格样本展示
└── StyleAdjustmentSlider.tsx # 风格调整滑块
```

#### 3.5.2 Store 扩展

```typescript
// src/stores/feature/styleStore.ts

interface StyleState {
  currentProfile: UserStyleProfile | null;
  appliedStyle: StyleVector | null;
  isApplying: boolean;

  // Actions
  loadStyleProfile: () => Promise<void>;
  applyStyleToCode: (code: string) => Promise<string>;
  applyStyleToDocument: (content: string) => Promise<string>;
  adjustStyleDimension: (dimension: string, value: number) => void;
  resetToDefaults: () => void;
}
```

### 3.6 与 Agent 集成

在 `src-tauri/crates/agent/src/` 中集成风格迁移：

```rust
// agent.rs 扩展

impl Agent {
    pub fn generate_with_style(
        &self,
        prompt: &str,
        style_profile: &StyleVector,
        content_type: ContentType,
    ) -> GeneratedContent {
        match content_type {
            ContentType::Code => {
                let code = self.generate_code(prompt);
                self.style_applier.apply_code_style(&code, style_profile)
            }
            ContentType::Document => {
                let doc = self.generate_document(prompt);
                self.style_applier.apply_document_style(&doc, style_profile)
            }
            ContentType::Explanation => {
                let explanation = self.generate_explanation(prompt);
                self.style_applier.apply_explanation_style(&explanation, style_profile)
            }
        }
    }
}
```

### 3.7 文件清单

| 文件路径 | 描述 | 操作 |
|---------|------|------|
| `src-tauri/crates/trajectory/src/style_migrator.rs` | 风格迁移核心 | 新建 |
| `src-tauri/crates/trajectory/src/style_vectorizer.rs` | 风格向量化 | 新建 |
| `src-tauri/crates/trajectory/src/style_extractor.rs` | 风格提取 | 新建 |
| `src-tauri/crates/trajectory/src/style_applier.rs` | 风格应用 | 新建 |
| `src-tauri/crates/trajectory/src/lib.rs` | 模块导出更新 | 修改 |
| `src-tauri/crates/agent/src/lib.rs` | Agent 集成 | 修改 |
| `src/stores/feature/styleStore.ts` | 风格 Store | 新建 |
| `src/components/style/*.tsx` | UI 组件 | 新建 |
| `src/types/style.ts` | TypeScript 类型 | 新建 |

### 3.8 验收标准

- [ ] 风格向量能准确表示用户偏好
- [ ] 风格迁移保持代码功能正确性
- [ ] 支持代码和文档两种风格迁移
- [ ] 前端能预览和应用风格调整
- [ ] 与 Agent 生成流程集成

---

## 四、模块 9: 主动助手能力

### 4.1 概述

基于用户画像和上下文，预测用户需求并主动提供帮助。包括上下文预测、主动建议、任务准备和例行提醒。

### 4.2 当前基线

无主动助手功能。现有代码库仅有：

- 基础的 Nudge 系统 (`nudgeStore.ts`)
- 被动响应用户指令
- 无预测模型

### 4.3 架构设计

#### 4.3.1 Rust 模块

```
src-tauri/crates/trajectory/src/
├── proactive_assistant.rs   # 主动助手核心
├── context_predictor.rs     # 上下文预测
├── suggestion_engine.rs     # 建议引擎
├── task_prefetcher.rs       # 任务预取
└── reminder_manager.rs     # 提醒管理
```

#### 4.3.2 核心数据结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveCapability {
    pub capability_type: CapabilityType,
    pub confidence: f32,
    pub trigger_conditions: Vec<TriggerCondition>,
    pub action: ProactiveAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CapabilityType {
    ContextPrediction,
    ProactiveSuggestion,
    TaskPrefetch,
    RoutineReminder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPrediction {
    pub predicted_intent: PredictedIntent,
    pub confidence: f32,
    pub reasoning: String,
    pub suggested_actions: Vec<SuggestedAction>,
    pub context_window: ContextWindow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PredictedIntent {
    CodeCompletion { language: String, context: String },
    Documentation { topic: String },
    Search { query_type: String },
    Refactoring { target: String },
    Debug { error: String },
    TestGeneration { target: String },
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveSuggestion {
    pub id: String,
    pub suggestion_type: SuggestionType,
    pub title: String,
    pub description: String,
    pub action: SuggestionAction,
    pub priority: Priority,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    Completion,    // 代码补全建议
    Refactor,      // 重构建议
    Documentation, // 文档建议
    Test,          // 测试建议
    Optimization,  // 优化建议
    Learning,      // 学习资源推荐
}
```

### 4.4 实现细节

#### 4.4.1 上下文预测器 (context_predictor.rs)

基于当前上下文预测用户意图：

```rust
impl ContextPredictor {
    pub fn predict(&self, context: &AgentContext) -> ContextPrediction {
        // 1. 提取上下文特征
        let features = self.extract_features(context);

        // 2. 使用规则和模式匹配进行预测
        let predictions = self.rule_based_predict(&features);

        // 3. 如果有足够的历史数据，使用 ML 模型增强
        if self.model.is_available() {
            let ml_predictions = self.ml_predict(&features);
            predictions.merge(ml_predictions);
        }

        // 4. 选择置信度最高的预测
        predictions.select_best()
    }

    fn extract_features(&self, context: &AgentContext) -> ContextFeatures {
        ContextFeatures {
            current_file: context.current_file.clone(),
            recent_actions: context.recent_actions.clone(),
            time_of_day: Utc::now().time(),
            day_of_week: Utc::now().weekday(),
            project_type: self.detect_project_type(context),
            user_state: self.get_user_state(context),
        }
    }

    fn rule_based_predict(&self, features: &ContextFeatures) -> Predictions {
        let mut predictions = Predictions::new();

        // 模式: 打开新文件 -> 可能需要补全
        if features.recent_actions.contains(&Action::FileOpened) {
            predictions.add(PredictedIntent::CodeCompletion {
                language: features.current_file.language(),
                context: features.current_file.content(),
            }, 0.8);
        }

        // 模式: 错误消息 -> 可能需要调试
        if let Some(error) = features.extract_error() {
            predictions.add(PredictedIntent::Debug {
                error: error.clone(),
            }, 0.9);
        }

        // 更多规则...
        predictions
    }
}
```

#### 4.4.2 建议引擎 (suggestion_engine.rs)

生成和管理主动建议：

```rust
impl SuggestionEngine {
    pub fn generate_suggestions(
        &self,
        context: &AgentContext,
        prediction: &ContextPrediction,
        user_profile: &UserProfile,
    ) -> Vec<ProactiveSuggestion> {
        let mut suggestions = Vec::new();

        // 基于预测生成建议
        match &prediction.predicted_intent {
            PredictedIntent::CodeCompletion { language, context } => {
                suggestions.extend(self.suggest_code_completions(language, context, user_profile));
            }
            PredictedIntent::Documentation { topic } => {
                suggestions.extend(self.suggest_documentation(topic, user_profile));
            }
            // ... 更多类型
            _ => {}
        }

        // 基于用户习惯生成建议
        suggestions.extend(self.suggest_based_on_habits(context, user_profile));

        // 排序和过滤
        suggestions.sort_by(|a, b| b.priority.cmp(&a.priority));
        suggestions.truncate(5); // 限制建议数量

        suggestions
    }

    fn suggest_code_completions(
        &self,
        language: &str,
        context: &str,
        profile: &UserProfile,
    ) -> Vec<ProactiveSuggestion> {
        vec![
            ProactiveSuggestion {
                id: format!("completion_{}", Ulid::new()),
                suggestion_type: SuggestionType::Completion,
                title: format!("Complete {} code", language),
                description: "为您准备代码补全",
                action: SuggestionAction::PrefetchCompletion {
                    language: language.to_string(),
                    context: context.to_string(),
                },
                priority: Priority::High,
                expires_at: Utc::now() + Duration::minutes(5),
            }
        ]
    }
}
```

#### 4.4.3 任务预取器 (task_prefetcher.rs)

提前准备可能需要的资源：

```rust
impl TaskPrefetcher {
    pub fn prefetch(&self, predictions: &[ContextPrediction]) -> PrefetchResults {
        let mut results = PrefetchResults::new();

        for prediction in predictions {
            match &prediction.predicted_intent {
                PredictedIntent::CodeCompletion { language, context } => {
                    results.add(self.prefetch_code_context(language, context));
                }
                PredictedIntent::Search { query_type } => {
                    results.add(self.prefetch_search_results(query_type));
                }
                PredictedIntent::Documentation { topic } => {
                    results.add(self.prefetch_documentation(topic));
                }
                _ => {}
            }
        }

        results
    }

    fn prefetch_code_context(&self, language: &str, context: &str) -> PrefetchResult {
        // 预取相关代码片段、API 文档等
    }
}
```

### 4.5 前端集成

#### 4.5.1 新增组件

```
src/components/proactive/
├── ProactiveSuggestionBar.tsx  # 主动建议横幅
├── SuggestionCard.tsx          # 建议卡片
├── ContextPredictionPanel.tsx  # 上下文预测面板
├── ReminderList.tsx            # 提醒列表
└── PrefetchIndicator.tsx       # 预取状态指示器
```

#### 4.5.2 Store 扩展

```typescript
// src/stores/feature/proactiveStore.ts

interface ProactiveState {
  suggestions: ProactiveSuggestion[];
  predictions: ContextPrediction[];
  reminders: Reminder[];
  isEnabled: boolean;

  // Actions
  dismissSuggestion: (id: string) => void;
  acceptSuggestion: (id: string) => void;
  snoozeSuggestion: (id: string, duration: number) => void;
  addReminder: (reminder: ReminderInput) => void;
  removeReminder: (id: string) => void;
  setEnabled: (enabled: boolean) => void;
}
```

### 4.6 与现有系统集成

#### 4.6.1 与 Nudge 系统集成

扩展现有的 `nudgeStore.ts`：

```typescript
// 主动建议可以作为 Nudge 的一种类型
interface ProactiveNudge extends Nudge {
  type: 'proactive_suggestion';
  suggestion: ProactiveSuggestion;
  onAccept: () => void;
  onDismiss: () => void;
}
```

#### 4.6.2 与 Agent 系统集成

```rust
// 在 Agent 循环中添加主动检查
impl Agent {
    pub async fn run_loop(&mut self, input: &str) -> Result<AgentResponse> {
        // 1. 处理用户输入
        let response = self.process_input(input).await?;

        // 2. 检查主动建议
        if self.config.proactive_enabled {
            let suggestions = self.suggestion_engine.generate(&self.context, &self.user_profile);
            response.add_suggestions(suggestions);
        }

        Ok(response)
    }
}
```

### 4.7 文件清单

| 文件路径 | 描述 | 操作 |
|---------|------|------|
| `src-tauri/crates/trajectory/src/proactive_assistant.rs` | 主动助手核心 | 新建 |
| `src-tauri/crates/trajectory/src/context_predictor.rs` | 上下文预测 | 新建 |
| `src-tauri/crates/trajectory/src/suggestion_engine.rs` | 建议引擎 | 新建 |
| `src-tauri/crates/trajectory/src/task_prefetcher.rs` | 任务预取 | 新建 |
| `src-tauri/crates/trajectory/src/reminder_manager.rs` | 提醒管理 | 新建 |
| `src-tauri/crates/trajectory/src/lib.rs` | 模块导出更新 | 修改 |
| `src-tauri/crates/agent/src/lib.rs` | Agent 集成 | 修改 |
| `src/stores/feature/proactiveStore.ts` | 主动助手 Store | 新建 |
| `src/stores/feature/nudgeStore.ts` | 扩展 Nudge Store | 修改 |
| `src/components/proactive/*.tsx` | UI 组件 | 新建 |
| `src/types/proactive.ts` | TypeScript 类型 | 新建 |

### 4.8 验收标准

- [ ] 上下文预测准确率达到基线水平
- [ ] 建议引擎生成有意义的建议
- [ ] 任务预取减少等待时间
- [ ] 前端正确显示和交互建议
- [ ] 与现有 Agent 系统无缝集成
- [ ] 用户可控制主动助手开关

---

## 五、测试策略

### 5.1 单元测试

- 各模块核心逻辑单元测试
- 风格迁移保持代码语义
- 预测算法的边界情况

### 5.2 集成测试

- 用户画像与 Agent 系统的集成
- 风格迁移在完整生成流程中的表现
- 主动助手与 Nudge 系统的协同

### 5.3 用户测试

- 个性化体验满意度调查
- 预测准确性反馈
- 风格迁移效果评估

---

## 六、依赖项

### 6.1 内部依赖

| 阶段 | 模块 | 依赖 |
|------|------|------|
| Phase 5 前 | Phase 4 | Agent、Memory 系统 |
| Phase 5 | 模块 7 | Phase 4 研究系统 |
| Phase 5 | 模块 8 | 模块 7 用户画像 |
| Phase 5 | 模块 9 | 模块 7 用户画像、模块 8 风格 |

### 6.2 外部依赖

无新增外部依赖。Phase 5 主要使用现有技术栈：

- Rust 异步运行时 (Tokio)
- SQLite/PostgreSQL (via existing db layer)
- LLM 用于风格分析 (复用现有 provider)

---

## 七、里程碑

| 里程碑 | 目标日期 | 交付内容 |
|--------|---------|---------|
| M1: 模块 7 核心 | 2026-11-01 | 用户画像基础结构、行为追踪、数据库 |
| M2: 模块 7 完成 | 2026-11-15 | 偏好学习器、前端展示完整 |
| M3: 模块 8 核心 | 2026-12-01 | 风格向量、提取器基础 |
| M4: 模块 8 完成 | 2026-12-15 | 风格迁移完整、前端集成 |
| M5: 模块 9 核心 | 2026-12-15 | 上下文预测、建议引擎基础 |
| M6: Phase 5 完成 | 2027-01-01 | 主动助手完整、系统集成、测试 |

---

## 八、风险与缓解

| 风险 | 影响 | 缓解策略 |
|------|------|---------|
| 用户隐私顾虑 | 高 | 透明化数据使用、本地优先、提供删除选项 |
| 风格迁移失真 | 中 | 保持原始语义、用户可调整、提供回退 |
| 预测准确率不足 | 中 | 多策略融合、用户反馈循环、渐进式学习 |
| 性能开销 | 低 | 异步处理、缓存、后台计算 |
| 画像稳定性差 | 中 | 时间衰减、批量更新、显式偏好优先 |

---

## 九、附录

### 9.1 术语表

| 术语 | 定义 |
|------|------|
| Style Vector | 表示用户风格的多维向量 |
| User Profile | 用户画像，包含多个维度的偏好信息 |
| Context Prediction | 基于当前上下文预测用户下一步意图 |
| Proactive Suggestion | 系统主动提出的建议 |
| Task Prefetch | 提前准备可能需要的资源和数据 |

### 9.2 参考文档

- [Phase 4 实施文档](./2026-04-26-phase4-implementation.md)
- [AxAgent 升级路线图](./2026-04-26-axagent-upgrade-roadmap.md)
- 现有 `userProfileStore.ts`
- 现有 `memoryStore.ts`
- 现有 `nudgeStore.ts`
