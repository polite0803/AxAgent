import { useMemo } from "react";
import { useTranslation } from "react-i18next";

export interface ChatViewWelcomeProps {
  loading: boolean;
  activeConversationId: string | null;
}

export function ChatViewWelcome({
  loading,
  activeConversationId,
}: ChatViewWelcomeProps) {
  const { t } = useTranslation();

  const greetingText = useMemo(() => {
    const hour = new Date().getHours();
    let key: string;
    if (hour >= 5 && hour < 12) {
      key = "chat.greetingMorning";
    } else if (hour >= 12 && hour < 14) {
      key = "chat.greetingNoon";
    } else if (hour >= 14 && hour < 18) {
      key = "chat.greetingAfternoon";
    } else {
      key = "chat.greetingEvening";
    }
    return `\u{1F44B} ${t(key)}`;
  }, [t]);

  if (activeConversationId && loading) {
    return (
      <div
        className="flex flex-col items-center justify-center"
        style={{ flex: "1 1 0%", minHeight: 0, gap: 12, padding: "0 24px", color: "var(--muted)" }}
      >
        <span>{t("chat.loadingConversation")}</span>
      </div>
    );
  }

  return (
    <div
      className="flex flex-col items-center justify-center"
      style={{ flex: "1 1 0%", minHeight: 0, padding: "0 24px" }}
    >
      <h2 className="ax-neon-text" style={{ marginBottom: 24, fontWeight: 500, fontSize: "1.5rem" }}>
        {greetingText}
      </h2>
    </div>
  );
}
