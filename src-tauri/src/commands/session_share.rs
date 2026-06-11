// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::app_state::{ShareParticipant, SharePermissions, ShareSessionRecord};
use serde::Serialize;
use tauri::State;

// ─── 返回给前端的 DTO ───

/// 创建/加入共享会话后返回的会话信息
#[derive(Debug, Clone, Serialize)]
pub struct ShareSessionInfo {
    pub session_id: String,
    pub invite_code: String,
    pub conversation_id: String,
    pub permissions: SharePermissions,
    pub participant_count: u32,
    pub created_at: i64,
}

// ─── 辅助函数 ───

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn make_session_info(session: &ShareSessionRecord) -> ShareSessionInfo {
    ShareSessionInfo {
        session_id: session.session_id.clone(),
        invite_code: session.invite_code.clone(),
        conversation_id: session.conversation_id.clone(),
        permissions: session.permissions.clone(),
        participant_count: session.participants.len() as u32,
        created_at: session.created_at,
    }
}

fn generate_invite_code() -> String {
    uuid::Uuid::new_v4()
        .to_string()
        .split_at(8)
        .0
        .to_uppercase()
}

// ─── Tauri 命令 ───

/// 创建共享会话（或使用已有 conversation_id 更新权限），返回邀请码
#[tauri::command]
pub async fn create_share_session(
    state: State<'_, AppState>,
    conversation_id: String,
    permissions: SharePermissions,
) -> Result<ShareSessionInfo, String> {
    let store = &state.session_share_manager;
    let mut sessions = store.write().await;

    // 如果该 conversation_id 已有共享会话，则更新权限
    if let Some(session) = sessions.get_mut(&conversation_id) {
        session.permissions = permissions;
        Ok(make_session_info(session))
    } else {
        // 新建会话
        let session = ShareSessionRecord {
            session_id: uuid::Uuid::new_v4().to_string(),
            invite_code: generate_invite_code(),
            conversation_id: conversation_id.clone(),
            permissions,
            participants: vec![ShareParticipant {
                id: "owner".to_string(),
                name: "Owner".to_string(),
                joined_at: now_ms(),
            }],
            created_at: now_ms(),
        };
        let info = make_session_info(&session);
        sessions.insert(conversation_id, session);
        Ok(info)
    }
}

/// 通过邀请码加入共享会话
#[tauri::command]
pub async fn join_share_session(
    state: State<'_, AppState>,
    invite_code: String,
) -> Result<ShareSessionInfo, String> {
    let store = &state.session_share_manager;
    let mut sessions = store.write().await;

    // 按邀请码查找会话
    let conv_id = {
        let found = sessions
            .iter()
            .find(|(_, s)| s.invite_code.eq_ignore_ascii_case(&invite_code));
        match found {
            Some((cid, _)) => cid.clone(),
            None => return Err("无效的邀请码".to_string()),
        }
    };

    let session = sessions.get_mut(&conv_id).ok_or("会话不存在")?;

    // 检查是否已达到最大参与人数
    if session.participants.len() >= session.permissions.max_participants as usize {
        return Err("会话已满，无法加入".to_string());
    }

    // 添加参与者
    let participant_name = format!("Guest-{}", uuid::Uuid::new_v4().to_string().split_at(6).0);
    session.participants.push(ShareParticipant {
        id: uuid::Uuid::new_v4().to_string(),
        name: participant_name,
        joined_at: now_ms(),
    });

    Ok(make_session_info(session))
}

/// 获取会话的参与者列表
#[tauri::command]
pub async fn list_share_participants(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<ShareParticipant>, String> {
    let store = &state.session_share_manager;
    let sessions = store.read().await;

    let session = sessions
        .values()
        .find(|s| s.session_id == session_id)
        .ok_or("会话不存在")?;

    Ok(session.participants.clone())
}
