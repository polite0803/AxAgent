// SPDX-License-Identifier: AGPL-3.0-only

import { safeJoinIds } from "../validators";

/** Permission level */
export type PermissionLevel = "read" | "write" | "execute" | "admin";

/** Resource type */
export type ResourceType =
  | "file"
  | "network"
  | "command"
  | "tool"
  | "database"
  | "clipboard"
  | "filesystem";

/** Permission request */
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

/** Permission decision */
export interface PermissionDecision {
  requestId: string;
  approved: boolean;
  decidedBy: "user" | "system" | "policy";
  constrainedPermissions: PermissionLevel[];
  expiresAt: number | null;
  reason?: string;
}

/** Permission policy */
export interface PermissionPolicy {
  /** Auto-approve low-risk operations */
  autoApprove: Array<{
    resourceType: ResourceType;
    action: string;
  }>;
  /** Always deny high-risk operations */
  autoDeny: Array<{
    resourceType: ResourceType;
    action: string;
  }>;
  /** Operations requiring user confirmation */
  requireConfirmation: Array<{
    resourceType: ResourceType;
    action: string;
    riskLevel: "medium" | "high" | "critical";
  }>;
  /** Default permission level */
  defaultLevel: PermissionLevel;
  /** Session timeout (ms) */
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

/** Permission guard */
export class PermissionGuard {
  private policy: PermissionPolicy;
  private activePermissions: Map<string, { level: PermissionLevel; expiresAt: number }>;

  constructor(policy?: Partial<PermissionPolicy>) {
    this.policy = { ...DEFAULT_POLICY, ...policy };
    this.activePermissions = new Map();
  }

  /** Update policy */
  updatePolicy(policy: Partial<PermissionPolicy>): void {
    this.policy = { ...this.policy, ...policy };
  }

  /** Check permission request */
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
        reason: `Operation ${request.action} is permanently denied by security policy`,
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
        reason: `User confirmation required: ${request.action} (risk level: ${requiresConfirm.riskLevel})`,
      };
    }

    return {
      requestId: request.id,
      approved: false,
      decidedBy: "policy",
      constrainedPermissions: [],
      expiresAt: null,
      reason: "Operation not in whitelist, denied by default",
    };
  }

  /** Grant permission */
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

  /** Revoke permission */
  revoke(request: PermissionRequest): void {
    this.activePermissions.delete(this.makeKey(request));
  }

  /** Clean expired permissions */
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

  /** Get current active permissions */
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

  /** Quick check: whether low-risk operations are allowed */
  canPerformLowRisk(resourceType: ResourceType, action: string): boolean {
    const request: PermissionRequest = {
      id: `quick-${Date.now()}`,
      resourceType,
      action,
      reason: "Quick permission check",
      riskLevel: "low",
      requiredPermissions: ["read"],
      context: {},
      timestamp: Date.now(),
    };
    const decision = this.evaluate(request);
    return decision.approved;
  }

  private makeKey(request: PermissionRequest): string {
    return safeJoinIds([request.resourceType, request.action], "::");
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
