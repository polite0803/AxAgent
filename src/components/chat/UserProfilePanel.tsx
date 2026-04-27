import { invoke } from "@/lib/invoke";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface CommunicationStyle {
  verbosity: string;
  technical_level: string;
  preferred_format: string;
  preferred_language: string;
}

interface UserProfileData {
  id: string;
  preferences: Record<string, string>;
  communication_style: CommunicationStyle;
  expertise: Record<string, string>;
  goals: string[];
  behavior_patterns: { pattern: string; frequency: number; confidence: number }[];
}

export default function UserProfilePanel() {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const [profile, setProfile] = useState<UserProfileData | null>(null);
  const [newPrefKey, setNewPrefKey] = useState("");
  const [newPrefValue, setNewPrefValue] = useState("");

  const fetchProfile = useCallback(async () => {
    try {
      const p = await invoke<UserProfileData>("user_profile_get", {});
      setProfile(p);
    } catch (e) { /* ignore */ }
  }, []);

  useEffect(() => {
    if (!expanded) { return; }
    fetchProfile();
  }, [expanded, fetchProfile]);

  const addPreference = async () => {
    if (!newPrefKey.trim() || !newPrefValue.trim()) { return; }
    try {
      await invoke("user_profile_set_preference", { key: newPrefKey.trim(), value: newPrefValue.trim() });
      setNewPrefKey("");
      setNewPrefValue("");
      fetchProfile();
    } catch (e) {
      console.warn("[profile] set preference failed:", e);
    }
  };

  if (!expanded) {
    return (
      <div className="border-b border-border/50 px-3 py-2">
        <button
          onClick={() => setExpanded(true)}
          className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M15.75 6a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0zM4.501 10.5a7.5 7.5 0 0114.998 0V12a7.5 7.5 0 01-14.998 0v-1.5z"
            />
          </svg>
          {t("chat.profile")}
        </button>
      </div>
    );
  }

  const style = profile?.communication_style;
  const prefs = profile?.preferences ?? {};
  const expertise = profile?.expertise ?? {};
  const patterns = profile?.behavior_patterns ?? [];

  return (
    <div className="border-b border-border/50 px-3 py-2 space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-foreground/80">{t("chat.userProfile")}</span>
        <button
          onClick={() => setExpanded(false)}
          className="text-muted-foreground hover:text-foreground transition-colors"
        >
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      {/* Communication style */}
      {style && (
        <div>
          <div className="text-[10px] font-medium text-muted-foreground/70 uppercase tracking-wider">
            {t("chat.style")}
          </div>
          <div className="flex flex-wrap gap-1 mt-0.5">
            {style.verbosity !== "unchanged" && (
              <span className="px-1.5 py-0.5 rounded text-[10px] bg-blue-500/10 text-blue-500">{style.verbosity}</span>
            )}
            {style.technical_level !== "unchanged" && (
              <span className="px-1.5 py-0.5 rounded text-[10px] bg-purple-500/10 text-purple-500">
                {style.technical_level}
              </span>
            )}
            {style.preferred_format !== "unchanged" && (
              <span className="px-1.5 py-0.5 rounded text-[10px] bg-green-500/10 text-green-500">
                {style.preferred_format}
              </span>
            )}
            {style.preferred_language && (
              <span className="px-1.5 py-0.5 rounded text-[10px] bg-amber-500/10 text-amber-500">
                {style.preferred_language}
              </span>
            )}
            {style.verbosity === "unchanged" && style.technical_level === "unchanged"
              && style.preferred_format === "unchanged" && !style.preferred_language && (
              <span className="text-[10px] text-muted-foreground/50">{t("chat.adaptingFromInteractions")}</span>
            )}
          </div>
        </div>
      )}

      {/* Preferences */}
      {Object.keys(prefs).length > 0 && (
        <div>
          <div className="text-[10px] font-medium text-muted-foreground/70 uppercase tracking-wider">
            {t("chat.preferences")}
          </div>
          <div className="space-y-0.5 mt-0.5">
            {Object.entries(prefs).map(([k, v]) => (
              <div key={k} className="text-[11px] flex gap-1">
                <span className="text-muted-foreground">{k}:</span>
                <span className="text-foreground/80">{v}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Expertise */}
      {Object.keys(expertise).length > 0 && (
        <div>
          <div className="text-[10px] font-medium text-muted-foreground/70 uppercase tracking-wider">
            {t("chat.expertise")}
          </div>
          <div className="flex flex-wrap gap-1 mt-0.5">
            {Object.entries(expertise).map(([d, l]) => (
              <span
                key={d}
                className={`px-1.5 py-0.5 rounded text-[10px] ${
                  l === "Expert"
                    ? "bg-green-500/10 text-green-500"
                    : l === "Advanced"
                    ? "bg-blue-500/10 text-blue-500"
                    : l === "Beginner"
                    ? "bg-amber-500/10 text-amber-500"
                    : "bg-muted/30 text-muted-foreground"
                }`}
              >
                {d} ({l})
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Behavior patterns */}
      {patterns.length > 0 && (
        <div>
          <div className="text-[10px] font-medium text-muted-foreground/70 uppercase tracking-wider">
            {t("chat.patterns")}
          </div>
          <div className="space-y-0.5 mt-0.5">
            {patterns.filter(p => p.confidence >= 0.5).slice(0, 3).map((p, i) => (
              <div key={i} className="text-[11px] text-foreground/60">
                {p.pattern}{" "}
                <span className="text-muted-foreground/50">x{p.frequency} {Math.round(p.confidence * 100)}%</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Add preference */}
      <div className="flex gap-1">
        <input
          value={newPrefKey}
          onChange={(e) => setNewPrefKey(e.target.value)}
          placeholder="key"
          className="flex-1 text-[11px] px-1.5 py-0.5 rounded bg-muted/30 border-none outline-none placeholder:text-muted-foreground/40"
        />
        <input
          value={newPrefValue}
          onChange={(e) => setNewPrefValue(e.target.value)}
          placeholder="value"
          className="flex-1 text-[11px] px-1.5 py-0.5 rounded bg-muted/30 border-none outline-none placeholder:text-muted-foreground/40"
          onKeyDown={(e) => e.key === "Enter" && addPreference()}
        />
        <button
          onClick={addPreference}
          className="text-[11px] px-1.5 py-0.5 rounded bg-muted/30 hover:bg-muted/50 text-muted-foreground hover:text-foreground transition-colors"
        >
          +
        </button>
      </div>
    </div>
  );
}
