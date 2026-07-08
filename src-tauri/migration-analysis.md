# evolution_drift → trajectory::skill_evolution 迁移分析

## 现状

### 当前实现（stock-analysis crate）

```
evolution_drift (100+ 行)                    weight_decay (~140 行)
├── load_current_weights()    ← DB 读取      └── compute_adjusted_weights()
├── load_performance_window() ← DB 读取               │
├── recalc_and_persist()      ← 写入 DB               ├── Beta(1,1) 贝叶斯平滑
├── get_dashboard()           ← 前端数据                ├── EWMA 平滑
├── get_timeline()            ← 前端数据                ├── log-饱和样本降权
└── record_performance()      ← 写入 DB                └── [0.05, 1.0] clamp
```

**特点**：纯函数算法层 + 薄数据层。约 250 行代码，测试覆盖率高（7 个 utest）。

### trajectory crate 的进化引擎

```
SkillEvolutionEngine (核心入口)
├── EvolutionConfig      ← 种群/变异/交叉/收敛阈值配置
├── EvolutionPopulation  ← SkillGenome[] + 适应度历史
├── tournament_select    ← 锦标赛选择
├── crossover_genomes    ← 单点交叉
├── mutate_genome        ← 四种变异（交换/加错误处理/加条件/微调）
├── evaluate_fitness_*   ← trajectory 驱动的适应度评估
└── CoevolutionEnvironment ← 自适应难度调节
```

**特点**：通用 GEPA 遗传算法引擎，但专为 Skill（文本步骤）设计，`SkillGenome` 包含 `content: String` 和 `steps: Vec<ProcedureStep>`。

## 能否直接复用？

| 模块                     | 直接复用？        | 理由                                  |
| ------------------------ | ----------------- | ------------------------------------- |
| `EvolutionConfig`        | ✅ 可直接 import  | 纯配置结构体，无领域依赖              |
| `tournament_select`      | ❌ 需重写         | 绑定在 `Vec<SkillGenome>` 上          |
| `crossover_genomes`      | ❌ 需重写         | 专为 `ProcedureStep` 数组设计         |
| `mutate_genome`          | ❌ 领域不匹配     | 变异的是文本步骤，不是数值参数        |
| `CoevolutionEnvironment` | ✅ 可直接复用     | 生成测试场景，可用于多市场条件回测    |
| `SandboxExecutor`        | ❌ 领域不匹配     | 执行的是 bash 命令，不是策略回测      |
| `evaluate_fitness_*`     | ❌ 输入类型不匹配 | 需要的是 `Trajectory`，不是策略表现行 |

## 推荐迁移路径：三层架构

不强制替换现有代码，而是新增一个**策略进化引擎**复用 trajectory 的配置+进化管道架构：

```
┌───────────────────────────────────────────────────────────┐
│                  继承 / 复用 trajectory 的架构              │
├───────────────────────────────────────────────────────────┤
│                                                           │
│  evolution_drift / weight_decay (保留, 作为"单代进化")     │
│  └── compute_adjusted_weights()                           │
│       ← 现有实现已经是成熟的单次权重调整算法                  │
│                                                           │
│  + StrategyEvolutionEngine (新增, 接入 trajectory 架构)    │
│    ├── 基因组编码: numeric (不是 SkillGenome)              │
│    │   基因 = [ewma_alpha, lookback_days, sample_saturation│
│    │          , weight_min, weight_max, confidence_method] │
│    ├── 种群初始化: 在 WeightDecayConfig 参数空间采样        │
│    ├── 适应度函数: 对各基因型运行回测, 以 Sharpe / PnL 评分  │
│    ├── crossover: 数值交叉 (BLX-α / SBX) 而非单点步骤交叉   │
│    ├── mutation: 高斯扰动数值而非交换步骤顺序                │
│    └── 输出: 最优 WeightDecayConfig 参数组合                │
│                                                           │
│  前端 EvolutionDriftPanel 扩展 (新增)                      │
│  └── 进化过程可视化 (种群收敛曲线 / 参数重要性排序)          │
└───────────────────────────────────────────────────────────┘
```

### 具体步骤

#### Step 1: 在 trajectory crate 新增 `NumericGenome` 类型

```rust
// trajectory/src/numeric_evolution.rs (新文件)

/// 数值型基因 — 用于参数空间的遗传算法搜索
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericGenome {
    pub params: HashMap<String, f64>,
    pub fitness: f64,
}

/// 参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub step: f64, // 0.0 表示连续
}

/// 数值进化引擎
pub struct NumericEvolutionEngine {
    config: EvolutionConfig,  // 复用 trajectory 的 EvolutionConfig
    param_defs: Vec<ParamDef>,
    population: Vec<NumericGenome>,
    fitness_history: Vec<f64>,
}
```

核心演化算子（已在上游实现过 pattern，在 numeric 场景重实现）：

| 算子 | trajectory 原版                       | 数值版             |
| ---- | ------------------------------------- | ------------------ |
| 选择 | `tournament_select(Vec<SkillGenome>)` | 保留，仅改泛型即可 |
| 交叉 | 单点截取 `ProcedureStep[]`            | BLX-α 算术交叉     |
| 变异 | 交换/添加/微调步骤                    | 高斯噪声扰动参数值 |

复用 `EvolutionConfig`、`CoevolutionEnvironment` 和 `EvolutionStats`。

#### Step 2: 在 stock-analysis crate 接入

```rust
// stock-analysis/src/evolution_drift.rs 新增函数

pub async fn evolve_strategy_params(
    db: &DatabaseConnection,
    performance_data: &[StrategyPerformanceRow],
) -> Result<WeightDecayConfig, String> {
    // 1. 初始化种群（在 WeightDecayConfig 参数空间采样）
    let param_defs = vec![
        ParamDef { name: "ewma_alpha", min: 0.05, max: 0.8, step: 0.0 },
        ParamDef { name: "lookback_days", min: 5.0, max: 180.0, step: 1.0 },
        ParamDef { name: "sample_saturation", min: 5.0, max: 100.0, step: 1.0 },
    ];
    let mut engine = NumericEvolutionEngine::new(
        EvolutionConfig { population_size: 30, max_generations: 50, ..default() },
        param_defs,
    );

    // 2. 适应度函数: 用该参数计算权重 → 模拟收益
    engine.set_fitness_fn(|genome: &NumericGenome| {
        let cfg = decode_config(genome);
        let adjusted = compute_adjusted_weights(history, &cfg, current);
        compute_sharpe_ratio(&adjusted)  // 以 Sharpe 比作为适应度
    });

    // 3. 进化
    let best = engine.run().await;
    Ok(decode_config(&best))
}
```

#### Step 3: 保留现有代码兼容

```rust
// 现有 recalc_and_persist 保持不变
pub async fn recalc_and_persist(db: &DatabaseConnection, as_of_date: Option<String>) -> Result<(), String> {
    // 现有逻辑：读取 → compute_adjusted_weights → 写入
    // ...
}

// 新增 end_of_day 自动进化
pub async fn auto_evolve(db: &DatabaseConnection) -> Result<(), String> {
    let perf = load_performance_window(db, 180).await?;
    let optimal_cfg = evolve_strategy_params(db, &perf).await?;
    info!("[AutoEvolve] 进化完成: alpha={}, lookback={}", optimal_cfg.ewma_alpha, optimal_cfg.lookback_days);
    Ok(())
}
```

## 工作量评估

| 步骤     | 内容                                   | 代码量      | 工期       |
| -------- | -------------------------------------- | ----------- | ---------- |
| 1        | `NumericEvolutionEngine`（含算子实现） | ~250 行     | 1 天       |
| 2        | stock-analysis 侧接入                  | ~80 行      | 0.5 天     |
| 3        | 前端进化过程可视化                     | ~150 行     | 0.5 天     |
| 4        | 测试覆盖                               | ~100 行     | 0.5 天     |
| **合计** |                                        | **~580 行** | **2.5 天** |

## 为什么不直接删掉 weight_decay？

`compute_adjusted_weights` 是经过充分测试的确定性算法（7 个单元测试），作为**单代进化**的基线与 fallback 很有价值：

- 每次 `recalc_and_persist` 调用时仍使用它做快速调整
- `NumericEvolutionEngine` 只在**后台/定时任务**中运行（如每日收盘后），搜索更好的参数组合
- 发现更优参数后，更新 `WeightDecayConfig` 的默认值

## 结论

| 方案                               | 优点                               | 缺点                                                |
| ---------------------------------- | ---------------------------------- | --------------------------------------------------- |
| 完整替换 weight_decay              | 复用最多上游代码                   | 两个领域耦合过高，SkillEvolutionEngine 改起来成本大 |
| **新增 NumericEvolutionEngine** ✅ | 保留现有代码稳定，新增进化搜索能力 | 需额外写约 250 行算子代码                           |
| 完全不复用                         | 成本 0，兼容性 100%                | 每次调整用手调参数，无自动化搜索能力                |

**推荐方案**：新增 `NumericEvolutionEngine`，复用 `EvolutionConfig` 和 `CoevolutionEnvironment`。
