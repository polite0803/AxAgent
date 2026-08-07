// SPDX-License-Identifier: AGPL-3.0-only

//! WorkItemService（DB 持久化，Self-Run 运行时）。
//!
//! 状态机纯函数在 `work_item` 模块（work_item.rs 上半部）；本文件
//! 是 DB 层：CRUD + 状态转换落库 + 依赖传播（doomed 判定）。

use crate::work_item::{ManagementMode, Phase, Transition, dependency_doomed, transition};
use axagent_entities::opc_work_items;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;

/// 创建 work item 的输入。
#[derive(Debug, Clone)]
pub struct NewWorkItem {
    pub id: String,
    pub title: String,
    pub owner_role_id: Option<String>,
    pub deps: Vec<String>,
    pub assignee_agent_id: Option<String>,
    pub management_mode: Option<ManagementMode>,
    pub manager_role_id: Option<String>,
}

impl NewWorkItem {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            owner_role_id: None,
            deps: Vec::new(),
            assignee_agent_id: None,
            management_mode: None,
            manager_role_id: None,
        }
    }

    pub fn owner(mut self, role: impl Into<String>) -> Self {
        self.owner_role_id = Some(role.into());
        self
    }

    pub fn dep(mut self, dep: impl Into<String>) -> Self {
        self.deps.push(dep.into());
        self
    }

    pub fn mode(mut self, mode: ManagementMode) -> Self {
        self.management_mode = Some(mode);
        self
    }
}

/// WorkItem 运行时服务：CRUD + 状态转换 + 依赖传播。
pub struct WorkItemService<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> WorkItemService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create(&self, input: NewWorkItem) -> crate::CompanyResult<opc_work_items::Model> {
        let now = chrono::Utc::now().timestamp();
        let am = opc_work_items::ActiveModel {
            id: Set(input.id.clone()),
            run_id: Set(None),
            phase: Set(Phase::Queued.as_str().to_string()),
            title: Set(input.title.clone()),
            owner_role_id: Set(input.owner_role_id.clone()),
            deps_json: Set(serde_json::to_string(&input.deps)?),
            assignee_agent_id: Set(input.assignee_agent_id.clone()),
            management_mode: Set(input
                .management_mode
                .map(|m| serde_json::to_string(&m).unwrap_or_default())),
            manager_role_id: Set(input.manager_role_id.clone()),
            last_error: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let model = am.insert(self.db).await?;
        Ok(model)
    }

    pub async fn get(&self, id: &str) -> crate::CompanyResult<opc_work_items::Model> {
        opc_work_items::Entity::find_by_id(id)
            .one(self.db)
            .await?
            .ok_or_else(|| crate::CompanyError::NotFound(format!("work item {id}")))
    }

    pub async fn list_by_phase(
        &self,
        phase: Option<Phase>,
    ) -> crate::CompanyResult<Vec<opc_work_items::Model>> {
        let mut q = opc_work_items::Entity::find();
        if let Some(p) = phase {
            q = q.filter(opc_work_items::Column::Phase.eq(p.as_str()));
        }
        Ok(q.all(self.db).await?)
    }

    /// 应用状态转换：读当前 phase → transition() → 写回。
    /// 若转换非法返回 Err，DB 不变。
    pub async fn apply(
        &self,
        id: &str,
        event: Transition,
    ) -> crate::CompanyResult<opc_work_items::Model> {
        let existing = self.get(id).await?;
        let current = existing
            .phase
            .parse::<Phase>()
            .map_err(|e| crate::CompanyError::State(e.to_string()))?;
        let next =
            transition(current, event).map_err(|e| crate::CompanyError::State(e.to_string()))?;

        let mut am: opc_work_items::ActiveModel = existing.into();
        am.phase = Set(next.as_str().to_string());
        am.updated_at = Set(chrono::Utc::now().timestamp());
        let model = am.update(self.db).await?;
        Ok(model)
    }

    /// 读取依赖项的 phase 列表（解析 deps_json）。
    pub async fn deps_phases(
        &self,
        item: &opc_work_items::Model,
    ) -> crate::CompanyResult<Vec<Phase>> {
        let deps: Vec<String> = serde_json::from_str(&item.deps_json)?;
        let mut out = Vec::new();
        for dep in deps {
            if let Ok(m) = self.get(&dep).await
                && let Ok(p) = m.phase.parse::<Phase>()
            {
                out.push(p);
            }
        }
        Ok(out)
    }

    /// 依赖传播：上游失败/取消 → 本项 doomed（应阻塞认领）。
    pub async fn is_doomed(&self, id: &str) -> crate::CompanyResult<bool> {
        let item = self.get(id).await?;
        let deps = self.deps_phases(&item).await?;
        Ok(dependency_doomed(&deps))
    }

    /// 认领执行：若依赖 doomed 则拒绝（返回 Err）。
    pub async fn start(&self, id: &str) -> crate::CompanyResult<opc_work_items::Model> {
        if self.is_doomed(id).await? {
            return Err(crate::CompanyError::State(format!(
                "{id} 的依赖已失败/取消（doomed），不可认领"
            )));
        }
        self.apply(id, Transition::Start).await
    }

    /// 批量更新（供外部联动，如 rt-workflow 事件回调）。
    pub async fn set_phase(
        &self,
        id: &str,
        phase: Phase,
    ) -> crate::CompanyResult<opc_work_items::Model> {
        let existing = self.get(id).await?;
        let mut am: opc_work_items::ActiveModel = existing.into();
        am.phase = Set(phase.as_str().to_string());
        am.updated_at = Set(chrono::Utc::now().timestamp());
        Ok(am.update(self.db).await?)
    }

    /// 依赖图：id → 直接依赖列表。
    pub async fn dep_map(
        &self,
        ids: &[String],
    ) -> crate::CompanyResult<HashMap<String, Vec<String>>> {
        let mut out = HashMap::new();
        for id in ids {
            if let Ok(m) = self.get(id).await {
                let deps: Vec<String> = serde_json::from_str(&m.deps_json)?;
                out.insert(id.clone(), deps);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod db_tests {
    use super::*;

    #[tokio::test]
    async fn work_item_crud_and_transition() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let svc = WorkItemService::new(db);

        let created =
            svc.create(NewWorkItem::new("wi-1", "撰写报告").owner("opc-cfo")).await.unwrap();
        assert_eq!(created.phase, "QUEUED");

        let started = svc.start("wi-1").await.unwrap();
        assert_eq!(started.phase, "IN_PROGRESS");

        let reviewed = svc.apply("wi-1", Transition::SubmitForReview).await.unwrap();
        assert_eq!(reviewed.phase, "REVIEW");

        let approved = svc.apply("wi-1", Transition::Approve).await.unwrap();
        assert_eq!(approved.phase, "APPROVED");

        // Approved 非终态：可 → DONE
        let done = svc.apply("wi-1", Transition::Start).await.unwrap();
        assert_eq!(done.phase, "DONE");

        // DONE 是终态：转换报错
        assert!(svc.apply("wi-1", Transition::Start).await.is_err());
    }

    #[tokio::test]
    async fn work_item_dependency_doomed() {
        let h = axagent_dao::db::create_test_pool().await.unwrap();
        let db = &h.conn;
        let svc = WorkItemService::new(db);

        // 父项失败
        svc.create(NewWorkItem::new("parent", "父任务")).await.unwrap();
        svc.apply("parent", Transition::Start).await.unwrap();
        svc.apply("parent", Transition::Fail).await.unwrap();

        // 子项依赖父项 → doomed
        svc.create(NewWorkItem::new("child", "子任务").dep("parent")).await.unwrap();
        assert!(svc.is_doomed("child").await.unwrap());
        assert!(svc.start("child").await.is_err(), "依赖 doomed 不可认领");

        // 父项正常 → 子项可认领
        svc.create(NewWorkItem::new("parent2", "父任务2")).await.unwrap();
        svc.create(NewWorkItem::new("child2", "子任务2").dep("parent2")).await.unwrap();
        assert!(!svc.is_doomed("child2").await.unwrap());
        assert!(svc.start("child2").await.is_ok());
    }
}
