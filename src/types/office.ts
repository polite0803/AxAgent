// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Fleet（多办公室 AI 团队）类型定义。
 *
 * 与后端 `axagent_harness::fleet` DTO 一一对应。
 * 后端权威定义：src-tauri/crates/harness/src/fleet.rs
 */

/** 舰队（办公室）状态 */
export type FleetStatus = "active" | "paused" | "stopped";

/** 舰队成员状态 */
export type FleetMemberStatus =
  | "idle"
  | "busy"
  | "paused"
  | "error"
  | "offline";

/** 舰队元数据 — 业务层可扩展信息 */
export interface FleetMetadata {
  /** 业务描述 */
  description: string;
  /** 最大成员数（0 表示无限制） */
  maxMembers: number;
  /** 协作策略名称（如 "ecommerce_ops" / "customer_service"） */
  strategy?: string;
  /** 自定义标签 */
  tags: string[];
}

/** 舰队（办公室）— 一个正在运行的 AI 团队 */
export interface Fleet {
  /** 唯一 ID（UUID） */
  id: string;
  /** 显示名称 */
  name: string;
  /** 场景模板 slug（可选，下游业务系统可填） */
  sceneTemplateSlug?: string;
  /** 舰队状态 */
  status: FleetStatus;
  /** 创建时间（Unix 毫秒） */
  createdAt: number;
  /** 更新时间（Unix 毫秒） */
  updatedAt: number;
  /** 业务元数据 */
  metadata: FleetMetadata;
}

/** 舰队成员 — 办公室里的一个 agent */
export interface FleetMember {
  /** 唯一 ID（UUID） */
  id: string;
  /** 所属舰队 ID */
  fleetId: string;
  /** 关联的 AgentSession ID */
  agentId: string;
  /** agent slug（业务标识，用于 Dispatcher 路由） */
  agentSlug: string;
  /** 显示名称 */
  displayName: string;
  /** 角色描述（注入到 Dispatcher prompt；与 agentProfileId 二选一，均可） */
  role: string;
  /** 关联的 AgentProfile ID（AgentProfile = 角色 + 专家组合，定义成员智能体身份） */
  agentProfileId?: string;
  /** 房间 ID（前端 Phaser 渲染位置，如 "manager" / "meeting"） */
  roomId: string;
  /** 成员状态 */
  status: FleetMemberStatus;
  /** 加入时间（Unix 毫秒） */
  joinedAt: number;
  /** 今日 token 用量 */
  todayTokens: number;
  /** 累计 token 用量 */
  totalTokens: number;
}

// ── Dispatcher 事件流 ────────────────────────────────────────────────

/** 调度事件 — Dispatcher 在路由与执行过程中产生的事件流 */
export type DispatchEvent =
  | { type: "routing"; agentSlug: string; agentId: string; roomId: string; taskSummary: string }
  | { type: "process"; agentSlug: string; agentId: string; status: string }
  | { type: "agent_message"; agentSlug: string; agentId: string; content: string }
  | { type: "agent_status"; agentSlug: string; agentId: string; status: FleetMemberStatus }
  | { type: "token_usage"; agentSlug: string; agentId: string; inputTokens: number; outputTokens: number }
  | { type: "complete" }
  | { type: "error"; message: string };

/** 聊天消息（Dispatcher 输入） */
export interface DispatchChatMessage {
  /** 角色：user / assistant / system */
  role: string;
  /** 消息内容 */
  content: string;
  /** 关联的 agent slug（assistant 消息才有） */
  agentSlug?: string;
}

// ── 命令输入参数 ──────────────────────────────────────────────────────

/** 创建舰队输入 */
export interface CreateFleetInput {
  /** 显示名称 */
  name: string;
  /** 场景模板 slug（可选） */
  sceneTemplateSlug?: string;
  /** 业务元数据 */
  metadata?: FleetMetadata;
}

/** 添加成员输入 */
export interface AddMemberInput {
  /** 所属舰队 ID */
  fleetId: string;
  /** 关联的 AgentSession ID */
  agentId: string;
  /** agent slug */
  agentSlug: string;
  /** 显示名称 */
  displayName: string;
  /** 角色描述 */
  role?: string;
  /** 关联的 AgentProfile ID（定义成员智能体身份） */
  agentProfileId?: string;
  /** 房间 ID（默认 "workspace"） */
  roomId?: string;
}

/** 群聊智能路由输入 */
export interface DispatchInput {
  /** 舰队 ID */
  fleetId: string;
  /** 用户消息 */
  userMessage: string;
  /** 历史消息 */
  history?: DispatchChatMessage[];
}

/** 直接 DM 指定 agent 输入 */
export interface DirectMessageInput {
  /** 舰队 ID */
  fleetId: string;
  /** 目标 agent slug */
  agentSlug: string;
  /** 用户消息 */
  userMessage: string;
  /** 历史消息 */
  history?: DispatchChatMessage[];
}

// ── 前端 UI 辅助类型 ──────────────────────────────────────────────────

/** Phaser 场景模板（前端 Phaser 办公室渲染用） */
export interface OfficeSceneTemplate {
  /** 模板 slug（如 "default_office" / "ecommerce_showroom"） */
  slug: string;
  /** 显示名称（i18n key） */
  displayNameKey: string;
  /** 房间布局（房间 ID → 像素坐标） */
  rooms: Record<string, { x: number; y: number; width: number; height: number }>;
  /** 默认房间 ID */
  defaultRoomId: string;
}

/** Phaser agent 精灵状态（前端 Phaser 渲染用） */
export interface AgentSpriteState {
  /** 关联的成员 ID */
  memberId: string;
  /** agent slug */
  agentSlug: string;
  /** 当前房间 ID */
  roomId: string;
  /** 精灵动画状态：idle / walking / typing / celebrating */
  animation: "idle" | "walking" | "typing" | "celebrating";
  /** 朝向：left / right */
  facing: "left" | "right";
  /** 像素坐标 */
  x: number;
  y: number;
  /** 目标坐标（行走动画的目标点） */
  targetX?: number;
  targetY?: number;
}
