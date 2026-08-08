// SPDX-License-Identifier: AGPL-3.0-only

import { getInjectionDetector } from "@/lib/security/injectionDetector";
import type { InjectionDetection } from "@/lib/security/injectionDetector";
import { getOutputSanitizer } from "@/lib/security/outputSanitizer";
import type { OutputRiskLevel } from "@/lib/security/outputSanitizer";
import { getPermissionGuard } from "@/lib/security/permissionGuard";
import type { PermissionLevel, PermissionRequest, ResourceType } from "@/lib/security/permissionGuard";
import { create } from "zustand";

interface SecurityState {
  recentDetections: InjectionDetection[];
  recentOutputRisks: Array<{
    contentPreview: string;
    riskLevel: OutputRiskLevel;
    timestamp: number;
  }>;
  activePermissions: Array<{
    resourceType: ResourceType;
    action: string;
    level: PermissionLevel;
    expiresAt: number;
  }>;
  stats: {
    totalChecks: number;
    blockedCount: number;
    warningCount: number;
    lastCheckAt: number | null;
  };

  checkInput: (input: string) => {
    isClean: boolean;
    sanitizedText: string;
    detections: InjectionDetection[];
  };
  sanitizeOutput: (output: string) => {
    content: string;
    riskLevel: OutputRiskLevel;
    warnings: string[];
  };
  requestPermission: (request: PermissionRequest) => {
    approved: boolean;
    reason?: string;
    constrainedPermissions: PermissionLevel[];
  };
  grantPermission: (
    request: PermissionRequest,
    level: PermissionLevel,
    durationMs?: number,
  ) => void;
  revokePermission: (request: PermissionRequest) => void;
  updateStats: (blocked: boolean, warned: boolean) => void;
}

export const useSecurityStore = create<SecurityState>((set, get) => ({
  recentDetections: [],
  recentOutputRisks: [],
  activePermissions: [],
  stats: {
    totalChecks: 0,
    blockedCount: 0,
    warningCount: 0,
    lastCheckAt: null,
  },

  checkInput: (input) => {
    const detector = getInjectionDetector();
    const result = detector.sanitize(input);

    set((s) => ({
      recentDetections: result.detections.slice(0, 20),
      stats: {
        ...s.stats,
        totalChecks: s.stats.totalChecks + 1,
        lastCheckAt: Date.now(),
      },
    }));

    if (result.detections.length > 0) {
      get().updateStats(
        result.detections.some((d) => d.severity === "critical"),
        result.detections.some((d) => d.severity === "high" || d.severity === "medium"),
      );
    }

    return {
      isClean: result.isClean,
      sanitizedText: result.sanitizedText,
      detections: result.detections,
    };
  },

  sanitizeOutput: (output) => {
    const sanitizer = getOutputSanitizer();
    const result = sanitizer.sanitize(output);

    set((s) => ({
      recentOutputRisks: [
        ...s.recentOutputRisks,
        {
          contentPreview: output.slice(0, 100),
          riskLevel: result.riskLevel,
          timestamp: Date.now(),
        },
      ].slice(-20),
    }));

    get().updateStats(
      result.riskLevel === "blocked",
      result.riskLevel === "warning",
    );

    return {
      content: result.content,
      riskLevel: result.riskLevel,
      warnings: result.warnings,
    };
  },

  requestPermission: (request) => {
    const guard = getPermissionGuard();
    const decision = guard.evaluate(request);

    if (decision.approved && decision.decidedBy === "user") {
      guard.grant(request, "execute");
    }

    set({
      activePermissions: guard.getActivePermissions(),
    });

    return {
      approved: decision.approved,
      reason: decision.reason,
      constrainedPermissions: decision.constrainedPermissions,
    };
  },

  grantPermission: (request, level, durationMs) => {
    const guard = getPermissionGuard();
    guard.grant(request, level, durationMs);
    set({
      activePermissions: guard.getActivePermissions(),
    });
  },

  revokePermission: (request) => {
    const guard = getPermissionGuard();
    guard.revoke(request);
    set({
      activePermissions: guard.getActivePermissions(),
    });
  },

  updateStats: (blocked, warned) => {
    set((s) => ({
      stats: {
        ...s.stats,
        blockedCount: s.stats.blockedCount + (blocked ? 1 : 0),
        warningCount: s.stats.warningCount + (warned ? 1 : 0),
      },
    }));
  },
}));
