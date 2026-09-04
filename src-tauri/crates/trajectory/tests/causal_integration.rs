// SPDX-License-Identifier: AGPL-3.0-only

//! 因果边层集成测试
//!
//! 覆盖单元测试无法触达的 DB 路径：read-then-write 的 insert/update 分支、
//! BFS 剪枝与环防护、以及从真实轨迹抽取因果观测的端到端行为。

use axagent_entities::knowledge_relations;
use axagent_trajectory::{
    CausalEdgeStats, DEFAULT_HINT_MIN_CONFIDENCE, MessageRole, ToolCall, Trajectory,
    TrajectoryOutcome, TrajectoryStep, TrajectoryStorage, TrajectoryToolResult, build_delay_hints,
    get_edge, observe_edge, observe_from_trajectory, predict_chain, tool_entity,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::sync::Arc;

/// 建立临时测试库。返回的 handle 需由调用方在结束时清理 db 文件。
async fn setup() -> axagent_dao::db::DbHandle {
    axagent_dao::db::create_test_pool().await.expect("create_test_pool failed")
}

async fn count_causal_rows(db: &sea_orm::DatabaseConnection) -> usize {
    knowledge_relations::Entity::find()
        .filter(knowledge_relations::Column::RelationType.eq("causes"))
        .all(db)
        .await
        .expect("query failed")
        .len()
}

fn step(ts: u64, tools: &[&str], is_error: bool) -> TrajectoryStep {
    TrajectoryStep {
        timestamp_ms: ts,
        role: MessageRole::Assistant,
        content: String::new(),
        reasoning: None,
        tool_calls: Some(
            tools
                .iter()
                .map(|n| ToolCall {
                    id: format!("call_{n}"),
                    name: (*n).to_string(),
                    arguments: "{}".to_string(),
                })
                .collect(),
        ),
        tool_results: Some(
            tools
                .iter()
                .map(|n| TrajectoryToolResult {
                    tool_use_id: format!("call_{n}"),
                    tool_name: (*n).to_string(),
                    output: String::new(),
                    is_error,
                })
                .collect(),
        ),
    }
}

#[tokio::test]
async fn observe_edge_insert_then_update_single_row() {
    let handle = setup().await;
    let db = &handle.conn;

    let first = observe_edge(db, "tool:a", "tool:b", true, Some(100), "traj_1")
        .await
        .expect("first observe");
    assert_eq!(first.observations, 1);
    assert_eq!(first.positive, 1);
    assert_eq!(count_causal_rows(db).await, 1);

    // 第二次观测同一条边：必须更新而非新建行
    let second = observe_edge(db, "tool:a", "tool:b", false, Some(300), "traj_2")
        .await
        .expect("second observe");
    assert_eq!(second.observations, 2);
    assert_eq!(second.positive, 1);
    assert!((second.strength() - 0.5).abs() < 1e-9);
    // 这是核心回归点：upsert_relation 的 on_conflict 缺陷会在此产生 2 行
    assert_eq!(count_causal_rows(db).await, 1, "同一条边不得产生重复行");

    // 回读校验持久化结果
    let persisted =
        get_edge(db, "tool:a", "tool:b").await.expect("get_edge").expect("edge must exist");
    assert_eq!(persisted.observations, 2);
    assert_eq!(persisted.positive, 1);
    assert!((persisted.delay_mean_ms - 200.0).abs() < 1e-9);

    std::fs::remove_file(&handle.path).ok();
}

#[tokio::test]
async fn distinct_pairs_create_distinct_rows() {
    let handle = setup().await;
    let db = &handle.conn;

    observe_edge(db, "tool:a", "tool:b", true, None, "traj_1").await.expect("a->b");
    observe_edge(db, "tool:a", "tool:c", true, None, "traj_1").await.expect("a->c");
    // 反向边与正向边是不同语义，必须独立
    observe_edge(db, "tool:b", "tool:a", true, None, "traj_1").await.expect("b->a");

    assert_eq!(count_causal_rows(db).await, 3);
    assert_eq!(get_edge(db, "tool:a", "tool:b").await.expect("q").map(|s| s.observations), Some(1));
    assert_eq!(get_edge(db, "tool:b", "tool:a").await.expect("q").map(|s| s.observations), Some(1));

    std::fs::remove_file(&handle.path).ok();
}

#[tokio::test]
async fn get_edge_returns_none_when_absent() {
    let handle = setup().await;
    let db = &handle.conn;

    assert_eq!(get_edge(db, "tool:nope", "tool:missing").await.expect("q"), None);

    std::fs::remove_file(&handle.path).ok();
}

#[tokio::test]
async fn chain_bfs_prunes_low_strength_paths() {
    let handle = setup().await;
    let db = &handle.conn;

    // A→B 强度 1.0
    for _ in 0..5 {
        observe_edge(db, "tool:a", "tool:b", true, Some(100), "traj_x").await.expect("a->b");
    }
    // B→C 强度 0.1（1 正 9 负）
    observe_edge(db, "tool:b", "tool:c", true, Some(50), "traj_x").await.expect("b->c");
    for _ in 0..9 {
        observe_edge(db, "tool:b", "tool:c", false, Some(50), "traj_x").await.expect("b->c");
    }

    let chains = predict_chain(db, "tool:a", 5, 0.35).await.expect("predict");
    assert!(!chains.is_empty(), "A→B 强度 1.0 应被收录");
    assert!(
        chains.iter().any(|c| c.path == vec!["tool:a".to_string(), "tool:b".to_string()]),
        "应包含 A→B"
    );
    // 累计强度 1.0 * 0.1 = 0.1 < 0.35，A→B→C 必须被剪枝
    assert!(
        !chains.iter().any(|c| c.path.len() >= 3),
        "累计强度低于阈值的路径必须被剪枝，实际: {chains:?}"
    );

    std::fs::remove_file(&handle.path).ok();
}

#[tokio::test]
async fn chain_accumulates_delay_along_path() {
    let handle = setup().await;
    let db = &handle.conn;

    for _ in 0..5 {
        observe_edge(db, "tool:a", "tool:b", true, Some(200), "traj_x").await.expect("a->b");
        observe_edge(db, "tool:b", "tool:c", true, Some(300), "traj_x").await.expect("b->c");
    }

    let chains = predict_chain(db, "tool:a", 5, 0.35).await.expect("predict");
    let abc = chains
        .iter()
        .find(|c| c.path == vec!["tool:a".to_string(), "tool:b".to_string(), "tool:c".to_string()])
        .expect("A→B→C 应存在");
    assert_eq!(abc.total_delay_ms, 500, "累计延迟应为 200 + 300");

    std::fs::remove_file(&handle.path).ok();
}

#[tokio::test]
async fn chain_cycle_protection_terminates() {
    let handle = setup().await;
    let db = &handle.conn;

    // A→B→A 双向高强度，构成环
    for _ in 0..5 {
        observe_edge(db, "tool:a", "tool:b", true, Some(10), "traj_x").await.expect("a->b");
        observe_edge(db, "tool:b", "tool:a", true, Some(10), "traj_x").await.expect("b->a");
    }

    let chains = predict_chain(db, "tool:a", 5, 0.35).await.expect("predict");
    assert!(!chains.is_empty());
    for chain in &chains {
        let mut seen = std::collections::HashSet::new();
        for node in &chain.path {
            assert!(seen.insert(node.clone()), "路径内不得出现重复节点: {:?}", chain.path);
        }
    }
    // 无环时 A→B、B→A 各 1 条，加上深度受限的有限组合
    assert!(chains.len() <= 10, "环防护失效会导致路径爆炸，实际 {}", chains.len());

    std::fs::remove_file(&handle.path).ok();
}

#[tokio::test]
async fn observe_from_trajectory_extracts_all_edge_types() {
    let handle = setup().await;
    let db = &handle.conn;

    let t = Trajectory::new(
        "sess_1".to_string(),
        "user_1".to_string(),
        "Refactor Auth".to_string(),
        "summary".to_string(),
        TrajectoryOutcome::Success,
        3000,
        vec![
            step(0, &["read_file"], false),
            step(1000, &["edit_file"], false),
            step(2000, &["run_tests"], false),
        ],
    );

    let n = observe_from_trajectory(db, &t).await.expect("observe");
    // T 边: read→edit, edit→run_tests = 2；O 边: 3 个工具 → outcome = 3；P 边: 1
    assert_eq!(n, 6, "应产出 2 条 T 边 + 3 条 O 边 + 1 条 P 边");
    assert_eq!(count_causal_rows(db).await, 6);

    // T 边的延迟应来自相邻 step 的 timestamp 差值
    let read_edit = get_edge(db, &tool_entity("read_file"), &tool_entity("edit_file"))
        .await
        .expect("q")
        .expect("read→edit 必须存在");
    assert_eq!(read_edit.observations, 1);
    assert!((read_edit.delay_mean_ms - 1000.0).abs() < 1e-9);

    // P 边：话题规范化后指向结果
    let topic_edge = get_edge(db, "topic:refactor_auth", "outcome:success")
        .await
        .expect("q")
        .expect("topic→outcome 必须存在");
    assert_eq!(topic_edge.observations, 1);
    assert!((topic_edge.strength() - 1.0).abs() < 1e-9, "Success 记为命中");

    std::fs::remove_file(&handle.path).ok();
}

#[tokio::test]
async fn observe_from_trajectory_marks_tool_errors() {
    let handle = setup().await;
    let db = &handle.conn;

    let t = Trajectory::new(
        "sess_2".to_string(),
        "user_1".to_string(),
        "broken".to_string(),
        "summary".to_string(),
        TrajectoryOutcome::Failure,
        500,
        vec![step(0, &["bad_tool"], true)],
    );

    observe_from_trajectory(db, &t).await.expect("observe");

    let edge = get_edge(db, &tool_entity("bad_tool"), "outcome:failure")
        .await
        .expect("q")
        .expect("工具错误边必须存在");
    assert!((edge.strength() - 1.0).abs() < 1e-9, "is_error=true 且 outcome=failure 记为命中");

    let topic_edge =
        get_edge(db, "topic:broken", "outcome:failure").await.expect("q").expect("P 边必须存在");
    assert!((topic_edge.strength() - 0.0).abs() < 1e-9, "Failure 不记为命中");

    std::fs::remove_file(&handle.path).ok();
}

/// 工具表现与最终结果不一致时应记为未命中。
/// 这一条是 O 边语义的护栏：若退化为「工具是否报错」的统计，此测试会失败。
#[tokio::test]
async fn observe_from_trajectory_penalizes_disagreement() {
    let handle = setup().await;
    let db = &handle.conn;

    // 工具执行无报错，但整条轨迹失败
    let t = Trajectory::new(
        "sess_4".to_string(),
        "user_1".to_string(),
        "misleading".to_string(),
        "summary".to_string(),
        TrajectoryOutcome::Failure,
        500,
        vec![step(0, &["good_tool"], false)],
    );

    observe_from_trajectory(db, &t).await.expect("observe");

    let edge = get_edge(db, &tool_entity("good_tool"), "outcome:failure")
        .await
        .expect("q")
        .expect("边必须存在");
    assert!(
        (edge.strength() - 0.0).abs() < 1e-9,
        "工具正常但轨迹失败属于不一致，强度应为 0，实际 {}",
        edge.strength()
    );

    std::fs::remove_file(&handle.path).ok();
}

#[tokio::test]
async fn observe_from_trajectory_handles_empty_steps() {
    let handle = setup().await;
    let db = &handle.conn;

    let t = Trajectory::new(
        "sess_3".to_string(),
        "user_1".to_string(),
        "empty".to_string(),
        "summary".to_string(),
        TrajectoryOutcome::Abandoned,
        0,
        vec![],
    );

    let n = observe_from_trajectory(db, &t).await.expect("observe");
    assert_eq!(n, 1, "无 step 时仅产出 P 边");

    std::fs::remove_file(&handle.path).ok();
}

/// 建立带 TrajectoryStorage 的测试库。
/// 同时返回连接的共享句柄，供测试直接查询底层表（不为此污染生产 API）。
async fn setup_storage() -> (TrajectoryStorage, Arc<sea_orm::DatabaseConnection>, String) {
    let handle = axagent_dao::db::create_test_pool().await.expect("create_test_pool failed");
    let path = handle.path.clone();
    let conn = Arc::new(handle.conn);
    (TrajectoryStorage::new(conn.clone()), conn, path)
}

fn sample_trajectory(id_hint: &str) -> Trajectory {
    Trajectory::new(
        format!("sess_{id_hint}"),
        "user_1".to_string(),
        "Refactor Auth".to_string(),
        "summary".to_string(),
        TrajectoryOutcome::Success,
        3000,
        vec![
            step(0, &["read_file"], false),
            step(1000, &["edit_file"], false),
            step(2000, &["run_tests"], false),
        ],
    )
}

/// 开关默认关闭：保存轨迹不得产生任何因果边，行为与改动前一致
#[tokio::test]
async fn storage_causal_disabled_by_default() {
    let (storage, db, path) = setup_storage().await;
    assert!(!storage.is_causal_enabled());

    let t = sample_trajectory("off");
    storage.save_trajectory(&t).await.expect("save");

    assert_eq!(count_causal_rows(&db).await, 0, "默认关闭时不得产生因果边");

    std::fs::remove_file(&path).ok();
}

/// 开启后保存轨迹应自动抽取因果边
#[tokio::test]
async fn storage_causal_enabled_produces_edges() {
    let (mut storage, db, path) = setup_storage().await;
    storage.set_causal_enabled(true);
    assert!(storage.is_causal_enabled());

    let t = sample_trajectory("on");
    storage.save_trajectory(&t).await.expect("save");

    assert_eq!(count_causal_rows(&db).await, 6, "应产出 2 条 T 边 + 3 条 O 边 + 1 条 P 边");
    assert!(
        get_edge(&db, &tool_entity("read_file"), &tool_entity("edit_file"))
            .await
            .expect("q")
            .is_some(),
        "T 边 read→edit 应存在"
    );

    std::fs::remove_file(&path).ok();
}

/// 同一条轨迹重复保存时，边应被更新而非重复插入
#[tokio::test]
async fn storage_repeated_save_updates_instead_of_duplicating() {
    let (mut storage, db, path) = setup_storage().await;
    storage.set_causal_enabled(true);

    let mut t = sample_trajectory("repeat");
    storage.save_trajectory(&t).await.expect("first save");
    t.summary = "second run".to_string();
    storage.save_trajectory(&t).await.expect("second save");

    assert_eq!(count_causal_rows(&db).await, 6, "重复保存不得新增行");
    assert_eq!(
        get_edge(&db, &tool_entity("read_file"), &tool_entity("edit_file"))
            .await
            .expect("q")
            .map(|s| s.observations),
        Some(2),
        "同一条边应累计观测次数"
    );

    std::fs::remove_file(&path).ok();
}

/// 延迟提示应返回观测到的真实间隔
#[tokio::test]
async fn build_delay_hints_returns_observed_delay() {
    let handle = setup().await;
    let db = &handle.conn;

    for _ in 0..10 {
        observe_edge(db, "intent:search", "intent:code_completion", true, Some(2_500), "traj_x")
            .await
            .expect("observe");
    }

    let hints =
        build_delay_hints(db, "intent:search", DEFAULT_HINT_MIN_CONFIDENCE).await.expect("hints");
    assert_eq!(hints.len(), 1);
    assert_eq!(hints.get("intent:code_completion"), Some(&2_500));

    std::fs::remove_file(&handle.path).ok();
}

/// 置信度不足的边必须被过滤——预取时机对噪声敏感
#[tokio::test]
async fn build_delay_hints_filters_low_confidence() {
    let handle = setup().await;
    let db = &handle.conn;

    // 仅 1 次观测：confidence = 1/4 = 0.25 < 0.5 门槛
    observe_edge(db, "intent:search", "intent:debug", true, Some(9_999), "traj_x")
        .await
        .expect("observe");
    // 10 次观测：confidence = 10/13 ≈ 0.77 > 0.5
    for _ in 0..10 {
        observe_edge(db, "intent:search", "intent:refactoring", true, Some(700), "traj_x")
            .await
            .expect("observe");
    }

    let hints =
        build_delay_hints(db, "intent:search", DEFAULT_HINT_MIN_CONFIDENCE).await.expect("hints");
    assert_eq!(hints.len(), 1, "低置信度边必须被过滤");
    assert_eq!(hints.get("intent:refactoring"), Some(&700));
    assert_eq!(hints.get("intent:debug"), None);

    std::fs::remove_file(&handle.path).ok();
}

#[test]
fn default_stats_equality_guards_serde_defaults() {
    // 集成层与单测层共用同一 Default，防止两处行为漂移
    assert_eq!(CausalEdgeStats::default(), CausalEdgeStats::default());
    assert_eq!(CausalEdgeStats::default().observations, 0);
}

// ===== P2.2：因果建议与意图转移观测 =====

use axagent_trajectory::{
    ContextPrediction, ContextWindow, PredictedIntent, ProactiveSuggestionType,
};

/// 构造一个给定意图的预测（仅用于建议查询，字段值无业务含义）
fn search_prediction() -> ContextPrediction {
    ContextPrediction {
        predicted_intent: PredictedIntent::Search { query_type: "symbol".to_string() },
        confidence: 0.9,
        reasoning: String::new(),
        suggested_actions: vec![],
        context_window: ContextWindow {
            files: Vec::new(),
            recent_actions: Vec::new(),
            current_language: None,
            project_type: None,
        },
        created_at: std::time::SystemTime::now().into(),
    }
}

#[tokio::test]
async fn storage_causal_suggestions_disabled_by_default() {
    let handle = setup().await;
    let storage = TrajectoryStorage::new(Arc::new(handle.conn.clone()));

    observe_edge(&handle.conn, "intent:search", "intent:debug", true, Some(500), "t")
        .await
        .expect("observe");

    let suggestions = storage.causal_suggestions(&search_prediction(), 2).await;
    assert!(suggestions.is_empty(), "开关关闭时不得产出因果建议");

    std::fs::remove_file(&handle.path).ok();
}

#[tokio::test]
async fn storage_causal_suggestions_describe_evidence() {
    let handle = setup().await;
    let mut storage = TrajectoryStorage::new(Arc::new(handle.conn.clone()));
    storage.set_causal_enabled(true);

    for _ in 0..5 {
        observe_edge(&handle.conn, "intent:search", "intent:debug", true, Some(1_800), "t")
            .await
            .expect("observe");
    }

    let suggestions = storage.causal_suggestions(&search_prediction(), 2).await;
    assert_eq!(suggestions.len(), 1);
    let s = &suggestions[0];
    assert_eq!(s.suggestion_type, ProactiveSuggestionType::CausalInsight);
    assert!(s.description.contains("1.8"), "描述应含观测延迟，实际: {}", s.description);
    assert!(s.description.contains("5"), "描述应含观测次数，实际: {}", s.description);

    std::fs::remove_file(&handle.path).ok();
}

#[tokio::test]
async fn storage_causal_suggestions_filter_low_sample_edges() {
    let handle = setup().await;
    let mut storage = TrajectoryStorage::new(Arc::new(handle.conn.clone()));
    storage.set_causal_enabled(true);

    // 仅 2 次观测：confidence = 2/5 = 0.4 < 0.5，不得产出建议
    for _ in 0..2 {
        observe_edge(&handle.conn, "intent:search", "intent:debug", true, Some(500), "t")
            .await
            .expect("observe");
    }

    let suggestions = storage.causal_suggestions(&search_prediction(), 2).await;
    assert!(suggestions.is_empty(), "低样本边（n<3）不得产出建议");

    std::fs::remove_file(&handle.path).ok();
}

#[tokio::test]
async fn intent_transition_observation_writes_edge() {
    let handle = setup().await;
    let mut storage = TrajectoryStorage::new(Arc::new(handle.conn.clone()));
    storage.set_causal_enabled(true);

    storage.observe_intent_transition("intent:search", "intent:debug", Some(45_000)).await;
    storage.observe_intent_transition("intent:search", "intent:debug", Some(45_000)).await;
    storage.observe_intent_transition("intent:debug", "intent:search", None).await;
    // 同实体不成边
    storage.observe_intent_transition("intent:search", "intent:search", Some(1_000)).await;

    let edge = get_edge(&handle.conn, "intent:search", "intent:debug").await.expect("q");
    let edge = edge.expect("edge 应存在");
    assert_eq!(edge.observations, 2);
    assert_eq!(edge.delay_mean_ms, 45_000.0);

    // 反向边独立计数
    let rev = get_edge(&handle.conn, "intent:debug", "intent:search").await.expect("q");
    assert_eq!(rev.expect("反向边应存在").observations, 1);

    std::fs::remove_file(&handle.path).ok();
}

#[tokio::test]
async fn intent_transition_observation_noop_when_disabled() {
    let handle = setup().await;
    let storage = TrajectoryStorage::new(Arc::new(handle.conn.clone()));

    storage.observe_intent_transition("intent:search", "intent:debug", Some(45_000)).await;

    let rows = count_causal_rows(&handle.conn).await;
    assert_eq!(rows, 0, "开关关闭时意图转移不得写库");

    std::fs::remove_file(&handle.path).ok();
}
