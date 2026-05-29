import { create } from "zustand";

export type ErrorSeverity = "info" | "warning" | "error" | "critical";
export type ErrorCategory =
  | "network"
  | "auth"
  | "not_found"
  | "validation"
  | "timeout"
  | "provider"
  | "storage"
  | "unknown";

export interface AppError {
  id: string;
  category: ErrorCategory;
  severity: ErrorSeverity;
  message: string;
  detail?: string;
  context?: string;
  retryable: boolean;
  retryFn?: () => Promise<unknown>;
  timestamp: number;
  dismissed: boolean;
}

const MAX_ERRORS = 100;

let _errorId = 0;

function classifyError(msg: string): { category: ErrorCategory; severity: ErrorSeverity; retryable: boolean } {
  const lower = msg.toLowerCase();

  if (
    lower.includes("connection") || lower.includes("refused") || lower.includes("reset")
    || lower.includes("econnrefused") || lower.includes("econnreset") || lower.includes("fetch")
    || lower.includes("network") || lower.includes("socket")
  ) {
    return { category: "network", severity: "error", retryable: true };
  }

  if (lower.includes("timeout") || lower.includes("timed out")) {
    return { category: "timeout", severity: "warning", retryable: true };
  }

  if (
    lower.includes("unauthorized") || lower.includes("forbidden") || lower.includes("auth") || lower.includes("api key")
  ) {
    return { category: "auth", severity: "critical", retryable: false };
  }

  if (lower.includes("not found") || lower.includes("notfound")) {
    return { category: "not_found", severity: "warning", retryable: false };
  }

  if (lower.includes("invalid") || lower.includes("validation") || lower.includes("bad request")) {
    return { category: "validation", severity: "warning", retryable: false };
  }

  if (lower.includes("provider") || lower.includes("model") || lower.includes("rate limit") || lower.includes("429")) {
    return { category: "provider", severity: "error", retryable: true };
  }

  if (lower.includes("database") || lower.includes("storage") || lower.includes("disk")) {
    return { category: "storage", severity: "critical", retryable: false };
  }

  return { category: "unknown", severity: "error", retryable: false };
}

interface ErrorNotificationState {
  errors: AppError[];
  unreadCount: number;

  pushError: (input: {
    message: string;
    detail?: string;
    context?: string;
    retryFn?: () => Promise<unknown>;
  }) => AppError;

  dismissError: (id: string) => void;
  dismissAll: () => void;

  retryError: (id: string) => Promise<void>;

  clearHistory: () => void;
}

export const useErrorNotificationStore = create<ErrorNotificationState>((set, get) => ({
  errors: [],
  unreadCount: 0,

  pushError: (input) => {
    const { category, severity, retryable } = classifyError(input.message);
    const appError: AppError = {
      id: `err-${++_errorId}`,
      category,
      severity,
      message: input.message,
      detail: input.detail,
      context: input.context,
      retryable: retryable && !!input.retryFn,
      retryFn: input.retryFn,
      timestamp: Date.now(),
      dismissed: false,
    };

    set((state) => {
      const errors = [appError, ...state.errors].slice(0, MAX_ERRORS);
      return {
        errors,
        unreadCount: state.unreadCount + 1,
      };
    });

    return appError;
  },

  dismissError: (id) => {
    set((state) => ({
      errors: state.errors.map((e) => e.id === id ? { ...e, dismissed: true } : e),
      unreadCount: Math.max(0, state.unreadCount - 1),
    }));
  },

  dismissAll: () => {
    set((state) => ({
      errors: state.errors.map((e) => ({ ...e, dismissed: true })),
      unreadCount: 0,
    }));
  },

  retryError: async (id) => {
    const error = get().errors.find((e) => e.id === id);
    if (!error?.retryFn) { return; }

    try {
      await error.retryFn();
      set((state) => ({
        errors: state.errors.map((e) => e.id === id ? { ...e, dismissed: true } : e),
        unreadCount: Math.max(0, state.unreadCount - 1),
      }));
    } catch (e) {
      get().pushError({
        message: e instanceof Error ? e.message : String(e),
        context: `retry:${error.context}`,
        retryFn: error.retryFn,
      });
    }
  },

  clearHistory: () => {
    set({ errors: [], unreadCount: 0 });
  },
}));
