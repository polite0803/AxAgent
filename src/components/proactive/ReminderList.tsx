import { type ReminderInput, useProactiveStore } from "@/stores/feature/proactiveStore";
import type { Reminder } from "@/types/proactive";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface ReminderListProps {
  showAddForm?: boolean;
}

export default function ReminderList({ showAddForm = false }: ReminderListProps) {
  const { t } = useTranslation();
  const {
    reminders,
    fetchReminders,
    completeReminder,
    removeReminder,
    addReminder,
    isLoading,
    error,
  } = useProactiveStore();

  const [showForm, setShowForm] = useState(showAddForm);
  const [newReminder, setNewReminder] = useState<ReminderInput>({
    title: "",
    description: "",
    scheduled_at: new Date().toISOString(),
  });

  useEffect(() => {
    fetchReminders();
  }, [fetchReminders]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newReminder.title.trim()) { return; }

    await addReminder(newReminder);
    setNewReminder({
      title: "",
      description: "",
      scheduled_at: new Date().toISOString(),
    });
    setShowForm(false);
  };

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    const now = new Date();
    const diff = date.getTime() - now.getTime();
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));

    if (days === 0) {
      return t("proactive.today");
    } else if (days === 1) {
      return t("proactive.tomorrow");
    } else if (days === -1) {
      return t("proactive.yesterday");
    } else if (days > 0 && days < 7) {
      return t("proactive.inDays", { days });
    } else {
      return date.toLocaleDateString();
    }
  };

  const formatTime = (dateString: string) => {
    const date = new Date(dateString);
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  };

  const getRecurrenceText = (reminder: Reminder) => {
    if (!reminder.recurrence) { return null; }
    const { frequency, interval } = reminder.recurrence;
    const intervalText = interval > 1 ? `every ${interval} ${frequency}s` : `every ${frequency.replace(/ly$/, "")}`;
    return intervalText;
  };

  const pendingReminders = reminders.filter((r) => !r.completed);
  const completedReminders = reminders.filter((r) => r.completed);

  return (
    <div className="bg-card border rounded-lg">
      <div className="px-4 py-3 border-b flex items-center justify-between">
        <h3 className="font-medium flex items-center gap-2">
          <svg className="w-4 h-4 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"
            />
          </svg>
          {t("proactive.reminders")}
          <span className="text-xs text-muted-foreground">
            ({pendingReminders.length})
          </span>
        </h3>
        <button
          onClick={() => setShowForm(!showForm)}
          className="p-1 text-muted-foreground hover:text-foreground transition-colors"
          title={t("proactive.addReminder")}
        >
          <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4" />
          </svg>
        </button>
      </div>

      <div className="p-4">
        {error && (
          <div className="mb-4 p-2 text-sm text-destructive bg-destructive/10 rounded">
            {error}
          </div>
        )}

        {showForm && (
          <form onSubmit={handleSubmit} className="mb-4 p-3 bg-muted/50 rounded-lg">
            <div className="space-y-3">
              <div>
                <label className="text-xs text-muted-foreground block mb-1">
                  {t("proactive.title")}
                </label>
                <input
                  type="text"
                  value={newReminder.title}
                  onChange={(e) => setNewReminder({ ...newReminder, title: e.target.value })}
                  className="w-full px-2 py-1 text-sm bg-background border rounded"
                  placeholder={t("proactive.titlePlaceholder")}
                  required
                />
              </div>
              <div>
                <label className="text-xs text-muted-foreground block mb-1">
                  {t("proactive.description")}
                </label>
                <textarea
                  value={newReminder.description}
                  onChange={(e) => setNewReminder({ ...newReminder, description: e.target.value })}
                  className="w-full px-2 py-1 text-sm bg-background border rounded resize-none"
                  rows={2}
                  placeholder={t("proactive.descriptionPlaceholder")}
                />
              </div>
              <div>
                <label className="text-xs text-muted-foreground block mb-1">
                  {t("proactive.scheduledAt")}
                </label>
                <input
                  type="datetime-local"
                  value={newReminder.scheduled_at.slice(0, 16)}
                  onChange={(e) =>
                    setNewReminder({ ...newReminder, scheduled_at: new Date(e.target.value).toISOString() })}
                  className="w-full px-2 py-1 text-sm bg-background border rounded"
                  required
                />
              </div>
              <div className="flex gap-2">
                <button
                  type="submit"
                  disabled={isLoading}
                  className="px-3 py-1 text-xs bg-primary text-primary-foreground rounded hover:bg-primary/90 disabled:opacity-50 transition-colors"
                >
                  {t("proactive.add")}
                </button>
                <button
                  type="button"
                  onClick={() => setShowForm(false)}
                  className="px-3 py-1 text-xs bg-muted rounded hover:bg-muted/80 transition-colors"
                >
                  {t("proactive.cancel")}
                </button>
              </div>
            </div>
          </form>
        )}

        {isLoading
          ? (
            <div className="flex items-center justify-center py-8">
              <div className="w-6 h-6 border-2 border-primary border-t-transparent rounded-full animate-spin" />
            </div>
          )
          : pendingReminders.length === 0 && completedReminders.length === 0
          ? (
            <div className="text-sm text-muted-foreground text-center py-4">
              {t("proactive.noReminders")}
            </div>
          )
          : (
            <div className="space-y-2">
              {pendingReminders.map((reminder) => (
                <div
                  key={reminder.id}
                  className="p-3 rounded-lg border hover:bg-muted/50 transition-colors"
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium">{reminder.title}</span>
                        {getRecurrenceText(reminder) && (
                          <span className="text-xs px-1.5 py-0.5 bg-primary/10 text-primary rounded">
                            {getRecurrenceText(reminder)}
                          </span>
                        )}
                      </div>
                      {reminder.description && (
                        <p className="text-xs text-muted-foreground mt-1">
                          {reminder.description}
                        </p>
                      )}
                      <div className="flex items-center gap-2 mt-2 text-xs text-muted-foreground">
                        <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
                          />
                        </svg>
                        <span>{formatDate(reminder.scheduled_at)}</span>
                        <span>{formatTime(reminder.scheduled_at)}</span>
                      </div>
                    </div>
                    <div className="flex gap-1">
                      <button
                        onClick={() => completeReminder(reminder.id)}
                        className="p-1 text-muted-foreground hover:text-green-500 transition-colors"
                        title={t("proactive.complete")}
                      >
                        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                          <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
                        </svg>
                      </button>
                      <button
                        onClick={() => removeReminder(reminder.id)}
                        className="p-1 text-muted-foreground hover:text-destructive transition-colors"
                        title={t("proactive.delete")}
                      >
                        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                          />
                        </svg>
                      </button>
                    </div>
                  </div>
                </div>
              ))}

              {completedReminders.length > 0 && (
                <div className="pt-2 mt-2 border-t">
                  <p className="text-xs text-muted-foreground mb-2">
                    {t("proactive.completed")}
                  </p>
                  {completedReminders.map((reminder) => (
                    <div
                      key={reminder.id}
                      className="p-2 rounded bg-muted/30 opacity-60"
                    >
                      <div className="flex items-center justify-between">
                        <span className="text-sm line-through">{reminder.title}</span>
                        <button
                          onClick={() => removeReminder(reminder.id)}
                          className="p-1 text-muted-foreground hover:text-destructive transition-colors"
                        >
                          <svg
                            className="w-3 h-3"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                            strokeWidth={2}
                          >
                            <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                          </svg>
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
      </div>
    </div>
  );
}
