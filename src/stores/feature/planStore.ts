import i18n from "@/i18n";
import { invoke, listen, type UnlistenFn } from "@/lib/invoke";
import type {
  Plan,
  PlanExecuteRequest,
  PlanExecutionCompleteEvent,
  PlanGeneratedEvent,
  PlanGenerateRequest,
  PlanModifyStepRequest,
  PlanStepStatus,
  PlanStepUpdateEvent,
} from "@/types";
import { message } from "antd";
import { create } from "zustand";

// ── Plan Event Types (frontend-only, derived from backend events) ─────

interface PlanStore {
  // ── State ──────────────────────────────────────────────────────────
  /** Active plan per conversation (only one active plan at a time per conversation) */
  activePlans: Record<string, Plan>;
  /** Completed/cancelled plans for history browsing */
  planHistory: Record<string, Plan[]>;
  /** Loading state per conversation */
  loading: Record<string, boolean>;
  /** Error state per conversation */
  errors: Record<string, string | null>;

  // ── Actions ────────────────────────────────────────────────────────
  /** Generate a plan for a conversation */
  generatePlan: (conversationId: string, content: string) => Promise<Plan>;
  /** Approve all steps in a plan and start execution */
  approvePlan: (conversationId: string, planId: string) => Promise<void>;
  /** Reject a plan entirely */
  rejectPlan: (conversationId: string, planId: string, reason?: string) => Promise<void>;
  /** Modify a single step (approve/reject/edit) */
  modifyStep: (
    conversationId: string,
    planId: string,
    stepId: string,
    modifications: { title?: string; description?: string; approved?: boolean },
  ) => Promise<void>;
  /** Execute an approved plan (or specific steps) */
  executePlan: (conversationId: string, planId: string, stepIds?: string[]) => Promise<void>;
  /** Resume a previously saved plan */
  resumePlan: (conversationId: string, planId: string) => Promise<void>;
  /** Cancel a plan that is in execution */
  cancelPlan: (conversationId: string, planId: string) => Promise<void>;
  /** Load plan history for a conversation */
  loadPlanHistory: (conversationId: string) => Promise<void>;
  /** Load the active plan from DB (for app restart recovery) */
  loadActivePlan: (conversationId: string) => Promise<void>;
  /** Clear active plan for a conversation */
  clearActivePlan: (conversationId: string) => void;
  /** Set loading state */
  setLoading: (conversationId: string, loading: boolean) => void;
  /** Set error state */
  setError: (conversationId: string, error: string | null) => void;

  // ── Event Handlers (called from conversationStore) ──────────────────
  handlePlanGenerated: (event: PlanGeneratedEvent) => void;
  handlePlanStepUpdate: (event: PlanStepUpdateEvent) => void;
  handlePlanExecutionComplete: (event: PlanExecutionCompleteEvent) => void;
  updatePlanStatus: (conversationId: string, planId: string, status: Plan["status"]) => void;
}

// ── Store ─────────────────────────────────────────────────────────────

export const usePlanStore = create<PlanStore>((set, get) => ({
  activePlans: {},
  planHistory: {},
  loading: {},
  errors: {},

  // ── Actions ──────────────────────────────────────────────────────────

  generatePlan: async (conversationId, content) => {
    set((s) => ({
      loading: { ...s.loading, [conversationId]: true },
      errors: { ...s.errors, [conversationId]: null },
    }));

    try {
      const request: PlanGenerateRequest = { conversationId, content };
      const plan: Plan = await invoke("plan_generate", { request });
      set((s) => ({
        activePlans: { ...s.activePlans, [conversationId]: plan },
        loading: { ...s.loading, [conversationId]: false },
      }));
      return plan;
    } catch (e) {
      const errMsg = String(e);
      message.error(errMsg);
      set((s) => ({
        loading: { ...s.loading, [conversationId]: false },
        errors: { ...s.errors, [conversationId]: errMsg },
      }));
      throw e;
    }
  },

  approvePlan: async (conversationId, planId) => {
    try {
      // Approve all pending steps before execution so plan_execute picks them up
      const plan = get().activePlans[conversationId];
      const pendingStepIds = plan?.steps
        .flatMap((s) => s.status === "pending" ? [s.id] : []) ?? [];

      await Promise.all(
        pendingStepIds.map((stepId) => invoke("plan_modify_step", { request: { planId, stepId, approved: true } })),
      );

      // 合并状态更新：清除 loading，若需要则同时更新 activePlans（单次 set 避免级联）
      set((s) => {
        const base = {
          loading: { ...s.loading, [conversationId]: false },
          errors: { ...s.errors, [conversationId]: null },
        };
        if (plan && pendingStepIds.length > 0) {
          return {
            ...base,
            activePlans: {
              ...s.activePlans,
              [conversationId]: {
                ...plan,
                steps: plan.steps.map((step) =>
                  pendingStepIds.includes(step.id)
                    ? { ...step, status: "approved" as const }
                    : step
                ),
              },
            },
          };
        }
        return base;
      });

      const allStepIds = plan?.steps.map((s) => s.id);
      const request: PlanExecuteRequest = { conversationId, planId, stepIds: allStepIds };
      await invoke("plan_execute", { request }, 0);
      // Plan status will be updated via planStepUpdate / planExecutionComplete events
    } catch (e) {
      const errMsg = String(e);
      message.error(errMsg);
      set((s) => ({
        loading: { ...s.loading, [conversationId]: false },
        errors: { ...s.errors, [conversationId]: errMsg },
      }));
    }
  },

  rejectPlan: async (conversationId, planId, reason) => {
    try {
      await invoke("plan_cancel", {
        request: { conversationId, planId, reason: reason || "User rejected the plan" },
      });
      // Move to history before clearing
      set((s) => {
        const plan = s.activePlans[conversationId];
        const history = s.planHistory[conversationId] || [];
        const { [conversationId]: _removed, ...restActive } = s.activePlans;
        return {
          activePlans: restActive,
          planHistory: plan
            ? { ...s.planHistory, [conversationId]: [{ ...plan, status: "cancelled" as const }, ...history] }
            : s.planHistory,
        };
      });
    } catch (e) {
      console.error("[planStore] rejectPlan failed:", e);
      message.error(i18n.t("planStore.rejectFailed", { error: String(e) }));
    }
  },

  modifyStep: async (conversationId, planId, stepId, modifications) => {
    try {
      const request: PlanModifyStepRequest = { planId, stepId, ...modifications };
      await invoke("plan_modify_step", { request });

      // Optimistic update
      const plan = get().activePlans[conversationId];
      if (plan && plan.id === planId) {
        const updatedSteps = plan.steps.map((step) => {
          if (step.id === stepId) {
            return {
              ...step,
              ...modifications,
              status: modifications.approved
                ? ("approved" as PlanStepStatus)
                : ("rejected" as PlanStepStatus),
            };
          }
          return step;
        });
        set((s) => ({
          activePlans: {
            ...s.activePlans,
            [conversationId]: { ...plan, steps: updatedSteps },
          },
        }));
      }
    } catch (e) {
      console.error("[planStore] modifyStep failed:", e);
      message.error(i18n.t("planStore.modifyStepFailed", { error: String(e) }));
    }
  },

  executePlan: async (conversationId, planId, stepIds) => {
    set((s) => ({
      loading: { ...s.loading, [conversationId]: true },
    }));

    try {
      const request: PlanExecuteRequest = { conversationId, planId, stepIds };
      await invoke("plan_execute", { request }, 0);
    } catch (e) {
      const errMsg = String(e);
      message.error(errMsg);
      set((s) => ({
        loading: { ...s.loading, [conversationId]: false },
        errors: { ...s.errors, [conversationId]: errMsg },
      }));
    }
  },

  resumePlan: async (conversationId, planId) => {
    set((s) => ({ loading: { ...s.loading, [conversationId]: true } }));
    try {
      const plan: Plan = await invoke("plan_activate", { request: { conversationId, planId } });
      set((s) => ({
        activePlans: { ...s.activePlans, [conversationId]: plan },
        loading: { ...s.loading, [conversationId]: false },
      }));
      message.success(i18n.t("planStore.planRestored"));
    } catch (e) {
      console.error("[planStore] resumePlan failed:", e);
      set((s) => ({ loading: { ...s.loading, [conversationId]: false } }));
      message.error(i18n.t("planStore.resumeFailed", { error: String(e) }));
    }
  },

  cancelPlan: async (conversationId, planId) => {
    try {
      await invoke("plan_cancel", { request: { conversationId, planId } });
      set((s) => {
        const plan = s.activePlans[conversationId];
        const history = s.planHistory[conversationId] || [];
        const { [conversationId]: _removed, ...restActive } = s.activePlans;
        return {
          activePlans: restActive,
          planHistory: plan
            ? { ...s.planHistory, [conversationId]: [{ ...plan, status: "cancelled" as const }, ...history] }
            : s.planHistory,
        };
      });
    } catch (e) {
      console.error("[planStore] cancelPlan failed:", e);
      message.error(i18n.t("planStore.cancelFailed", { error: String(e) }));
    }
  },

  loadPlanHistory: async (conversationId) => {
    try {
      const plans: Plan[] = await invoke("plan_list", {
        request: { conversationId, includeCompleted: true },
      });
      set((s) => ({
        planHistory: { ...s.planHistory, [conversationId]: plans },
      }));
    } catch (e) {
      console.error("[planStore] loadPlanHistory failed:", e);
      message.error(i18n.t("planStore.loadHistoryFailed", { error: String(e) }));
    }
  },

  /** Load the active plan for a conversation from DB (used for app restart recovery). */
  loadActivePlan: async (conversationId: string) => {
    try {
      const plans: Plan[] = await invoke("plan_list", {
        request: { conversationId, includeCompleted: false },
      });
      // Find the first reviewing/executing plan
      const activePlan = plans.find(
        (p: Plan) => p.is_active && (p.status === "reviewing" || p.status === "executing" || p.status === "draft"),
      );
      if (activePlan) {
        set((s) => ({
          activePlans: { ...s.activePlans, [conversationId]: activePlan },
          planHistory: { ...s.planHistory, [conversationId]: plans },
        }));
      }
    } catch (e) {
      // Silently ignore — plan loading is best-effort on startup
      console.debug("[planStore] loadActivePlan skipped:", e);
    }
  },

  clearActivePlan: (conversationId) => {
    set((s) => {
      const { [conversationId]: _removed, ...rest } = s.activePlans;
      return { activePlans: rest };
    });
  },

  setLoading: (conversationId, loading) => {
    set((s) => ({ loading: { ...s.loading, [conversationId]: loading } }));
  },

  setError: (conversationId, error) => {
    set((s) => ({ errors: { ...s.errors, [conversationId]: error } }));
  },

  // ── Event Handlers ───────────────────────────────────────────────────

  handlePlanGenerated: (event) => {
    const { conversationId, plan } = event;
    set((s) => {
      const oldPlan = s.activePlans[conversationId];
      const history = s.planHistory[conversationId] || [];
      return {
        // Archive old plan if present
        activePlans: { ...s.activePlans, [conversationId]: plan },
        // Move old plan to history (at the front)
        planHistory: oldPlan
          ? { ...s.planHistory, [conversationId]: [oldPlan, ...history] }
          : s.planHistory,
        loading: { ...s.loading, [conversationId]: false },
      };
    });
  },

  handlePlanStepUpdate: (event) => {
    const { conversationId, planId, stepId, status, result } = event;
    const plan = get().activePlans[conversationId];
    if (!plan || plan.id !== planId) { return; }

    const updatedSteps = plan.steps.map((step) => {
      if (step.id === stepId) {
        return { ...step, status, result: result ?? step.result };
      }
      return step;
    });

    // Determine overall plan status based on step states
    let planStatus = plan.status;
    const hasRunning = updatedSteps.some((s) => s.status === "running");
    const hasError = updatedSteps.some((s) => s.status === "error");
    const allDone = updatedSteps.every(
      (s) => s.status === "completed" || s.status === "rejected",
    );

    if (hasRunning) { planStatus = "executing"; }
    else if (allDone) { planStatus = hasError ? "partial" : "completed"; }

    set((s) => ({
      activePlans: {
        ...s.activePlans,
        [conversationId]: { ...plan, steps: updatedSteps, status: planStatus },
      },
    }));
  },

  handlePlanExecutionComplete: (event) => {
    const { conversationId, planId, status } = event;
    const plan = get().activePlans[conversationId];
    if (!plan || plan.id !== planId) { return; }

    const updatedPlan = { ...plan, status: status as Plan["status"] };

    // Move from active to history only
    set((s) => {
      const history = s.planHistory[conversationId] || [];
      const { [conversationId]: _removed, ...restActive } = s.activePlans;
      return {
        activePlans: restActive,
        planHistory: {
          ...s.planHistory,
          [conversationId]: [updatedPlan, ...history],
        },
      };
    });
  },

  updatePlanStatus: (conversationId, planId, status) => {
    const plan = get().activePlans[conversationId];
    if (!plan || plan.id !== planId) { return; }

    set((s) => ({
      activePlans: {
        ...s.activePlans,
        [conversationId]: { ...plan, status },
      },
    }));
  },
}));

// ── Event Listener Setup ───────────────────────────────────────────────
// Registered once, persisted across component mounts

let _planUnlisten: UnlistenFn | null = null;
let _planListenersInitialized = false;

export function setupPlanEventListeners(): () => void {
  if (_planListenersInitialized) {
    return () => {}; // Already set up
  }
  _planListenersInitialized = true;

  const unlisteners: UnlistenFn[] = [];

  listen<PlanGeneratedEvent>("plan-generated", (event) => {
    usePlanStore.getState().handlePlanGenerated(event.payload);
  }).then((fn) => unlisteners.push(fn));

  listen<PlanStepUpdateEvent>("plan-step-update", (event) => {
    usePlanStore.getState().handlePlanStepUpdate(event.payload);
  }).then((fn) => unlisteners.push(fn));

  listen<PlanExecutionCompleteEvent>("plan-execution-complete", (event) => {
    usePlanStore.getState().handlePlanExecutionComplete(event.payload);
  }).then((fn) => unlisteners.push(fn));

  _planUnlisten = () => {
    unlisteners.forEach((fn) => fn());
  };

  return _planUnlisten;
}
