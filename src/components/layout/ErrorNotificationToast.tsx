import { useErrorNotificationStore } from "@/stores/shared/errorNotificationStore";
import { AnimatePresence, motion } from "framer-motion";
import { useTranslation } from "react-i18next";

const severityStyles: Record<string, string> = {
  info: "border-blue-500/40 bg-blue-500/10 text-blue-200",
  warning: "border-amber-500/40 bg-amber-500/10 text-amber-200",
  error: "border-red-500/40 bg-red-500/10 text-red-200",
  critical: "border-red-600/60 bg-red-600/20 text-red-100",
};

const categoryIcons: Record<string, string> = {
  network: "🌐",
  auth: "🔐",
  not_found: "🔍",
  validation: "⚠️",
  timeout: "⏱️",
  provider: "🤖",
  storage: "💾",
  unknown: "❓",
};

export function ErrorNotificationToast() {
  const { t } = useTranslation();
  const { errors, dismissError, retryError } = useErrorNotificationStore();

  const visible = errors.filter((e) => !e.dismissed).slice(0, 5);

  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-sm">
      <AnimatePresence mode="popLayout">
        {visible.map((error) => (
          <motion.div
            key={error.id}
            initial={{ opacity: 0, y: 20, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: -10, scale: 0.95 }}
            className={`p-3 rounded-lg border shadow-lg backdrop-blur-sm ${severityStyles[error.severity]}`}
          >
            <div className="flex items-start gap-2">
              <span className="text-sm leading-none mt-0.5">
                {categoryIcons[error.category] ?? categoryIcons.unknown}
              </span>
              <div className="flex-1 min-w-0">
                <p className="text-xs font-medium break-words">
                  {error.message.slice(0, 200)}
                </p>
                {error.context && (
                  <p className="text-xs opacity-60 mt-0.5">
                    {error.context}
                  </p>
                )}
              </div>
              <button
                onClick={() => dismissError(error.id)}
                className="text-xs opacity-50 hover:opacity-100 transition-opacity ml-1 shrink-0"
              >
                ✕
              </button>
            </div>

            {error.retryable && (
              <div className="mt-2 ml-5">
                <button
                  onClick={() => retryError(error.id)}
                  className="px-2 py-0.5 text-xs bg-white/10 hover:bg-white/20 rounded transition-colors"
                >
                  {t("errorNotification.retry")}
                </button>
              </div>
            )}
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
}
