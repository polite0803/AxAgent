import { z } from "zod";

export const ToolCallStateSchema = z.object({
  toolUseId: z.string(),
  toolName: z.string(),
  executionStatus: z.enum([
    "pending",
    "running",
    "completed",
    "failed",
    "cancelled",
  ]),
  approvalStatus: z.enum(["pending", "approved", "denied", "auto_approved"]),
  input: z.record(z.string(), z.unknown()).optional(),
  output: z.unknown().nullable().optional(),
  isError: z.boolean().optional(),
  startedAt: z.string().optional(),
  completedAt: z.string().optional(),
  durationMs: z.number().optional(),
});

export const AgentSessionSchema = z.object({
  conversationId: z.string(),
  model: z.string().optional(),
  provider: z.string().optional(),
  systemPrompt: z.string().optional(),
  cwd: z.string().optional(),
  permissionMode: z.enum(["default", "plan", "auto"]).optional(),
  status: z
    .enum(["idle", "running", "paused", "completed", "failed"])
    .optional(),
  createdAt: z.string().optional(),
  updatedAt: z.string().optional(),
});

export const PlanStepSchema = z.object({
  id: z.string(),
  description: z.string(),
  status: z.enum(["pending", "in_progress", "completed", "failed", "skipped"]),
  dependencies: z.array(z.string()).optional(),
  toolName: z.string().optional(),
  toolInput: z.record(z.string(), z.unknown()).optional(),
  result: z.unknown().nullable().optional(),
  error: z.string().optional(),
});

export const PlanSchema = z.object({
  id: z.string(),
  conversationId: z.string(),
  title: z.string(),
  status: z.enum([
    "draft",
    "approved",
    "executing",
    "completed",
    "failed",
    "cancelled",
  ]),
  steps: z.array(PlanStepSchema),
  createdAt: z.string().optional(),
  updatedAt: z.string().optional(),
});

export const ReplanRecordSchema = z.object({
  version: z.number(),
  reason: z.string(),
  actions: z.array(z.unknown()),
  timestamp: z.string(),
});

export const ToTStateSummarySchema = z.object({
  rootId: z.string(),
  nodes: z.array(
    z.object({
      id: z.string(),
      content: z.string(),
      evaluationScore: z.number().min(0).max(1),
      status: z.enum(["Generated", "Explored", "Pruned", "Selected"]),
      parentId: z.string().nullable(),
      childIds: z.array(z.string()),
    }),
  ),
  selectedPath: z.array(z.string()),
});

export const SemanticCacheStatsSchema = z.object({
  totalEntries: z.number(),
  hits: z.number(),
  misses: z.number(),
  hitRate: z.number(),
  avgAccessCount: z.number(),
  expiredCount: z.number(),
});

export const ErrorReportSchema = z.object({
  errorCode: z.string(),
  message: z.string(),
  context: z.object({
    sessionId: z.string().nullable().optional(),
    component: z.string(),
    operation: z.string(),
    retryCount: z.number(),
    metadata: z.record(z.string(), z.string()),
    timestamp: z.string(),
  }),
  sourceChain: z.array(z.string()),
  timestamp: z.string(),
  recoverable: z.boolean(),
});
