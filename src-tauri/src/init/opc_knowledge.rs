// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 知识源初始化 — Wiki + Memory 种子化

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use axagent_dao::repo::{note, wiki};
use axagent_entities::{notes, wikis};
use axagent_harness::note_dtos::CreateNoteInput;

const OPC_WIKI_NAME: &str = "OPC 业务知识库";
const OPC_MEMORY_NAME: &str = "opc-business";

/// 在启动时播种 OPC 知识源
pub async fn seed_opc_knowledge(db: &DatabaseConnection) {
    let wiki_id = ensure_opc_wiki(db).await;
    if let Some(id) = wiki_id {
        seed_opc_notes(db, &id).await;
    }
    ensure_opc_memory(db).await;
}

/// 确保 OPC Wiki 存在，返回其 ID
async fn ensure_opc_wiki(db: &DatabaseConnection) -> Option<String> {
    // 检查是否已存在
    if let Ok(w) = wikis::Entity::find().filter(wikis::Column::Name.eq(OPC_WIKI_NAME)).one(db).await
    {
        if let Some(w) = w {
            tracing::info!("[opc-knowledge] Wiki already exists: {} ({})", w.name, w.id);
            return Some(w.id);
        }
    }

    // 创建 OPC Wiki
    let input = wiki::CreateWikiInput {
        name: OPC_WIKI_NAME.to_string(),
        description: Some(
            "OPC 一人公司业务文档——合同模板、税务政策、运营 SOP、项目管理规范".to_string(),
        ),
        root_path: "opc".to_string(),
        embedding_provider: None,
        knowledge_base_id: None,
    };
    match wiki::create_wiki(db, input).await {
        Ok(w) => {
            tracing::info!("[opc-knowledge] Created wiki: {} ({})", w.name, w.id);
            Some(w.id)
        },
        Err(e) => {
            tracing::warn!("[opc-knowledge] Failed to create wiki: {e}");
            None
        },
    }
}

/// 确保 OPC Memory namespace 存在
async fn ensure_opc_memory(db: &DatabaseConnection) -> Option<String> {
    use axagent_dao::repo::memory;

    if let Ok(namespaces) = memory::list_namespaces(db).await {
        if let Some(ns) = namespaces.into_iter().find(|n| n.name == OPC_MEMORY_NAME) {
            tracing::info!("[opc-knowledge] Memory namespace already exists: {}", ns.name);
            return Some(ns.id);
        }
    }

    let input = axagent_harness::types::CreateMemoryNamespaceInput {
        name: OPC_MEMORY_NAME.to_string(),
        scope: "global".to_string(),
        embedding_provider: None,
        embedding_dimensions: None,
        retrieval_threshold: None,
        retrieval_top_k: None,
        icon_type: Some("emoji".to_string()),
        icon_value: Some("🏢".to_string()),
    };
    match memory::create_namespace(db, input).await {
        Ok(ns) => {
            tracing::info!("[opc-knowledge] Created memory namespace: {}", ns.name);
            Some(ns.id)
        },
        Err(e) => {
            tracing::warn!("[opc-knowledge] Failed to create memory namespace: {e}");
            None
        },
    }
}

/// 播种初始 Wiki 页面
async fn seed_opc_notes(db: &DatabaseConnection, wiki_id: &str) {
    let seed_pages: Vec<(&str, &str, &str)> = vec![
        (
            "发票管理规范",
            "opc/invoice-policy",
            r#"# 发票管理规范

## 开票流程
1. 确认客户信息完整（名称、税号、地址、银行账户）
2. 根据服务/产品明细创建发票行项目
3. 设置合理的付款期限（默认 30 天）
4. 发票状态流转：草稿 → 发送 → 已收款/逾期 → 退款

## 税率标准
- 一般服务：6%
- 产品销售：13%
- 咨询服务：3%（小规模纳税人）

## 催款流程
- 逾期 7 天：发送友善提醒邮件
- 逾期 15 天：发送正式催款通知
- 逾期 30 天：升级处理

参见 [[客户管理规范]] 和 [[项目管理流程]]
"#,
        ),
        (
            "客户管理规范",
            "opc/customer-policy",
            r#"# 客户管理规范

## 客户状态定义
| 状态 | 说明 | 下一步动作 |
|------|------|-----------|
| Lead | 潜在线索 | 发起首次触达 |
| Prospect | 有意向客户 | 推动签约 |
| Active | 活跃客户 | 定期维护，挖掘增购 |
| Inactive | 非活跃客户 | 激活挽回 |
| Churned | 已流失 | 分析流失原因 |

## 客户来源追踪
- **Referral**：推荐客户 — 高转化率
- **Website**：网站获客 — 中等转化
- **SocialMedia**：社交媒体 — 品牌曝光
- **Marketplace**：平台获客 — 竞争环境
- **Direct**：直接联系 — 高意向

参见 [[发票管理规范]]
"#,
        ),
        (
            "项目管理流程",
            "opc/project-process",
            r#"# 项目管理流程

## SOP 标准流程
1. **项目规划** (Planning) — 明确范围、预算、时间线
2. **执行阶段** (Active) — 按里程碑推进，每周同步
3. **暂停处理** (Paused) — 客户变更/资源不足时暂停
4. **验收完成** (Completed) — 客户确认交付物后标记完成
5. **异常取消** (Cancelled) — 记录取消原因

## 里程碑管理
- 每个项目至少设置 3 个里程碑：启动、中期、交付
- 里程碑完成后更新状态并通知客户
- 延迟里程碑需记录原因和调整计划

## 预算控制
- 项目创建时设定预算上限
- 单笔支出超过预算 50% 需审批
- 项目结束时统计实际成本 vs 预算

参见 [[发票管理规范]] 和 [[客户管理规范]]
"#,
        ),
        (
            "税务与合规指南",
            "opc/tax-guide",
            r#"# 税务与合规指南

## 发票合规要求
- 发票抬头必须与客户营业执照一致
- 发票金额保留两位小数
- 发票备注中注明服务期间或合同编号
- 电子发票需归档备查

## 报税周期
- 增值税：每月 15 日前申报
- 所得税：每季度预缴，年度汇算清缴
- 附加税：随增值税同步申报

## 合规检查清单
- [ ] 客户资质审核（营业执照、税务登记）
- [ ] 合同签署（明确服务范围、金额、付款条件）
- [ ] 发票开具（信息准确、税率正确）
- [ ] 收款确认（到账核实、账务记录）

参见 [[发票管理规范]]
"#,
        ),
        (
            "客户沟通模板",
            "opc/communication-templates",
            r#"# 客户沟通模板

## 新客户欢迎邮件
```
主题：欢迎加入 {{公司名}} — 您的项目已启动

尊敬的 {{客户名}}，

感谢您选择我们的服务！您的项目 "{{项目名}}" 已正式启动。

项目负责人：{{负责人}}
预计交付日期：{{交付日期}}

我们将按里程碑定期向您同步进展。如有任何问题，随时联系我们。

此致
{{公司名}} 团队
```

## 发票催款模板
```
主题：关于发票 {{发票编号}} 的付款提醒

尊敬的 {{客户名}}，

您有一笔发票（编号：{{发票编号}}，金额：¥{{金额}}）已于 {{发送日期}} 发出，
付款期限为 {{到期日}}，目前尚未收到付款。

请尽快安排付款。如有疑问，请与我们联系。

此致
{{公司名}} 团队
```

参见 [[发票管理规范]] 和 [[客户管理规范]]
"#,
        ),
    ];

    for (title, file_path, content) in seed_pages {
        // 检查是否已存在相同路径的笔记
        let exists = notes::Entity::find()
            .filter(notes::Column::VaultId.eq(wiki_id))
            .filter(notes::Column::FilePath.eq(file_path))
            .one(db)
            .await
            .ok()
            .flatten()
            .is_some();

        if exists {
            tracing::info!("[opc-knowledge] Note already exists: {file_path}");
            continue;
        }

        let input = CreateNoteInput {
            vault_id: wiki_id.to_string(),
            title: title.to_string(),
            file_path: file_path.to_string(),
            content: content.to_string(),
            author: "system".to_string(),
            page_type: Some("doc".to_string()),
            source_refs: None,
        };

        match note::create_note(db, input).await {
            Ok(n) => tracing::info!("[opc-knowledge] Seeded note: {} ({})", n.title, n.file_path),
            Err(e) => tracing::warn!("[opc-knowledge] Failed to seed note {}: {e}", file_path),
        }
    }
}
