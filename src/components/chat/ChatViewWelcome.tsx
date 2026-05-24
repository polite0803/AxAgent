import { ChartNoAxesColumn, Code, FileText, Languages, Lightbulb, Search, Share2, TrendingUp } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

export interface ChatViewWelcomeProps {
  loading: boolean;
  activeConversationId: string | null;
  onPromptClick: (info: {
    data: { label?: unknown; scenario?: string };
  }) => void;
}

export function ChatViewWelcome({
  loading,
  activeConversationId,
  onPromptClick,
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

  const promptItems = useMemo(
    () => [
      {
        key: "1",
        icon: <Code size={16} />,
        label: t("chat.welcomePromptCoding"),
        scenario: "coding",
      },
      {
        key: "2",
        icon: <Lightbulb size={16} />,
        label: t("chat.welcomePromptCreative"),
        scenario: "creative",
      },
      {
        key: "3",
        icon: <Languages size={16} />,
        label: t("chat.welcomePromptTranslation"),
        scenario: "translation",
      },
      {
        key: "4",
        icon: <FileText size={16} />,
        label: t("chat.welcomePromptWriting"),
        scenario: "writing",
      },
      {
        key: "5",
        icon: <Search size={16} />,
        label: t("chat.welcomePromptResearch"),
        scenario: "research",
      },
      {
        key: "6",
        icon: <ChartNoAxesColumn size={16} />,
        label: t("chat.welcomePromptAnalysis"),
        scenario: "analysis",
      },
      {
        key: "7",
        icon: <TrendingUp size={16} />,
        label: t("chat.welcomePromptInvestment"),
        scenario: "investment",
      },
      {
        key: "8",
        icon: <Share2 size={16} />,
        label: t("chat.welcomePromptSocialMedia"),
        scenario: "social_media",
      },
    ],
    [t],
  );

  if (activeConversationId && loading) {
    return (
      <div
        className="flex flex-col items-center justify-center h-full"
        style={{ gap: 12, padding: "0 24px", color: "var(--muted)" }}
      >
        <span>{t("chat.loadingConversation")}</span>
      </div>
    );
  }

  return (
    <div className="flex flex-col items-center justify-center h-full" style={{ padding: "0 24px" }}>
      <h2 className="ax-neon-text" style={{ marginBottom: 24, fontWeight: 500, fontSize: "1.5rem" }}>
        {greetingText}
      </h2>
      <div className="prompts-grid">
        {promptItems.map((item) => (
          <button
            key={item.key}
            className="prompt-card"
            onClick={() => onPromptClick({ data: { label: item.label, scenario: item.scenario } })}
          >
            <span className="prompt-icon">{item.icon}</span>
            <span className="prompt-label">{item.label}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
