// SPDX-License-Identifier: AGPL-3.0-only

/** 权限等级 */
export type PermissionLevel = "read" | "write" | "execute" | "admin";

/** 资源类型 */
export type ResourceType =
  | "file"
  | "network"
  | "command"
  | "tool"
  | "database"
  | "clipboard"
  | "filesystem";

/** 权限请求 */
export interface PermissionRequest {
  id: string;
  resourceType: ResourceType;
  action: string;
  reason: string;
  riskLevel: "low" | "medium" | "high" | "critical";
  requiredPermissions: PermissionLevel[];
  context: Record<string, unknown>;
  timestamp: number;
}

/** 权限审批结果 */
export interface PermissionDecision {
  requestId: string;
  approved: boolean;
  decidedBy: "user" | "system" | "policy";
  constrainedPermissions: PermissionLevel[];
  expiresAt: number | null;
  reason?: string;
}

/** 权限策略 */
export interface PermissionPolicy {
  /** 自动批准的低风险操作 */
  autoApprove: Array<{
    resourceType: ResourceType;
    action: string;
  }>;
  /** 永远拒绝的高风险操作 */
  autoDeny: Array<{
    resourceType: ResourceType;
    action: string;
  }>;
  /** 需要用户确认的操作 */
  requireConfirmation: Array<{
    resourceType: ResourceType;
    action: string;
    riskLevel: "medium" | "high" | "critical";
  }>;
  /** 默认权限等级 */
  defaultLevel: PermissionLevel;
  /** 会话超时时间（ms） */
  sessionTimeoutMs: number;
}

const DEFAULT_POLICY: PermissionPolicy = {
  autoApprove: [
    { resourceType: "file", action: "read" },
    { resourceType: "tool", action: "invoke" },
    { resourceType: "network", action: "http_get" },
  ],
  autoDeny: [
    { resourceType: "command", action: "rm -rf" },
    { resourceType: "file", action: "delete_system" },
    { resourceType: "database", action: "drop_table" },
  ],
  requireConfirmation: [
    { resourceType: "file", action: "write", riskLevel: "medium" },
    { resourceType: "file", action: "delete", riskLevel: "high" },
    { resourceType: "command", action: "execute", riskLevel: "high" },
    { resourceType: "network", action: "http_post", riskLevel: "medium" },
    { resourceType: "clipboard", action: "write", riskLevel: "medium" },
  ],
  defaultLevel: "read",
  sessionTimeoutMs: 30 * 60 * 1000,
};

/** 权限守卫 */
export class PermissionGuard {
  private policy: PermissionPolicy;
  private activePermissions: Map<string, { level: PermissionLevel; expiresAt: number }>;

  constructor(policy?: Partial<PermissionPolicy>) {
    this.policy = { ...DEFAULT_POLICY, ...policy };
    this.activePermissions = new Map();
  }

  /** 更新策略 */
  updatePolicy(policy: Partial<PermissionPolicy>): void {
    this.policy = { ...this.policy, ...policy };
  }

  /** 检查权限请求 */
  evaluate(request: PermissionRequest): PermissionDecision {
    const denied = this.policy.autoDeny.find(
      (rule) =>
        rule.resourceType === request.resourceType
        && (rule.action === request.action
          || request.action.startsWith(rule.action)),
    );
    if (denied) {
      return {
        requestId: request.id,
        approved: false,
        decidedBy: "policy",
        constrainedPermissions: [],
        expiresAt: null,
        reason: `操作 ${request.action} 被安全策略永久拒绝`,
      };
    }

    const approved = this.policy.autoApprove.find(
      (rule) =>
        rule.resourceType === request.resourceType
        && rule.action === request.action,
    );
    if (approved) {
      return {
        requestId: request.id,
        approved: true,
        decidedBy: "policy",
        constrainedPermissions: [this.policy.defaultLevel],
        expiresAt: null,
      };
    }

    const active = this.activePermissions.get(this.makeKey(request));
    if (active && active.expiresAt > Date.now()) {
      return {
        requestId: request.id,
        approved: true,
        decidedBy: "system",
        constrainedPermissions: [active.level],
        expiresAt: active.expiresAt,
      };
    }

    const requiresConfirm = this.policy.requireConfirmation.find(
      (rule) =>
        rule.resourceType === request.resourceType
        && rule.action === request.action,
    );
    if (requiresConfirm) {
      return {
        requestId: request.id,
        approved: false,
        decidedBy: "user",
        constrainedPermissions: [],
        expiresAt: null,
        reason: `需要用户确认：${request.action}（风险等级：${requiresConfirm.riskLevel}）`,
      };
    }

    return {
      requestId: request.id,
      approved: false,
      decidedBy: "policy",
      constrainedPermissions: [],
      expiresAt: null,
      reason: "操作不在白名单内，默认拒绝",
    };
  }

  /** 授予权限 */
  grant(
    request: PermissionRequest,
    level: PermissionLevel,
    durationMs?: number,
  ): void {
    const expiresAt = durationMs
      ? Date.now() + durationMs
      : Date.now() + this.policy.sessionTimeoutMs;
    this.activePermissions.set(this.makeKey(request), { level, expiresAt });
  }

  /** 撤销权限 */
  revoke(request: PermissionRequest): void {
    this.activePermissions.delete(this.makeKey(request));
  }

  /** 清理过期权限 */
  cleanupExpired(): number {
    const now = Date.now();
    let count = 0;
    for (const [key, val] of this.activePermissions) {
      if (val.expiresAt <= now) {
        this.activePermissions.delete(key);
        count++;
      }
    }
    return count;
  }

  /** 获取当前活跃权限 */
  getActivePermissions(): Array<{
    resourceType: ResourceType;
    action: string;
    level: PermissionLevel;
    expiresAt: number;
  }> {
    const result: Array<{
      resourceType: ResourceType;
      action: string;
      level: PermissionLevel;
      expiresAt: number;
    }> = [];
    for (const [key, val] of this.activePermissions) {
      if (val.expiresAt > Date.now()) {
        const [resourceType, action] = key.split("::");
        result.push({
          resourceType: resourceType as ResourceType,
          action,
          level: val.level,
          expiresAt: val.expiresAt,
        });
      }
    }
    return result;
  }

  /** 快速检查：低风险操作是否被允许 */
  canPerformLowRisk(resourceType: ResourceType, action: string): boolean {
    const request: PermissionRequest = {
      id: `quick-${Date.now()}`,
      resourceType,
      action,
      reason: "快速权限检查",
      riskLevel: "low",
      requiredPermissions: ["read"],
      context: {},
      timestamp: Date.now(),
    };
    const decision = this.evaluate(request);
    return decision.approved;
  }

  private makeKey(request: PermissionRequest): string {
    return `${request.resourceType}::${request.action}`;
  }
}

let _guard: PermissionGuard | null = null;
export function getPermissionGuard(): PermissionGuard {
  if (!_guard) {
    _guard = new PermissionGuard();
    setInterval(() => {
      _guard?.cleanupExpired();
    }, 5 * 60 * 1000);
  }
  return _guard;
}
