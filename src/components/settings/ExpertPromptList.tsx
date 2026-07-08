/**
 * 股票分析专家 system_prompt 列表编辑器。
 * 从 agency_experts 表加载，支持展开编辑 + 保存。
 */
import { invoke } from "@/lib/invoke";
import { App, Button, Input, Spin, Tag } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface ExpertRow {
  id: string;
  name: string;
  description?: string | null;
  category: string;
  system_prompt: string;
  source_dir: string;
  is_enabled: boolean;
}

export function ExpertPromptList() {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const [experts, setExperts] = useState<ExpertRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [editText, setEditText] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let cancelled = false;
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      setLoading(true);
      return invoke<ExpertRow[]>("list_agency_experts");
    })
      .then((rows) => {
        if (cancelled) { return; }
        const filtered = Array.isArray(rows)
          ? rows.filter((r) =>
            r.source_dir === "stock-analysis" || r.category === "finance" || r.id.startsWith("agency-stock-analysis-")
          )
          : [];
        setExperts(filtered);
      })
      .catch(() => {
        if (!cancelled) { setExperts([]); }
      })
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const expand = (expert: ExpertRow) => {
    setExpandedId(expert.id);
    setEditText(expert.system_prompt);
  };

  const save = async () => {
    if (!expandedId) { return; }
    setSaving(true);
    try {
      await invoke("update_agency_expert", {
        request: { id: expandedId, system_prompt: editText },
      });
      message.success(t("common.saved"));
      setExperts((prev) => prev.map((e) => (e.id === expandedId ? { ...e, system_prompt: editText } : e)));
      setExpandedId(null);
    } catch {
      message.error(t("common.saveFailed"));
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div style={{ padding: 24, textAlign: "center" }}>
        <Spin />
      </div>
    );
  }

  return (
    <div>
      {experts.length === 0 && (
        <div style={{ color: "var(--muted)", padding: 12, textAlign: "center" }}>
          {t("stockAnalysis.settings.noExperts")}
        </div>
      )}
      {experts.map((expert) => (
        <div
          key={expert.id}
          style={{
            borderBottom: "1px solid var(--border)",
            padding: "8px 0",
          }}
        >
          <div
            style={{ display: "flex", alignItems: "center", justifyContent: "space-between", cursor: "pointer" }}
            onClick={() => (expandedId === expert.id ? setExpandedId(null) : expand(expert))}
          >
            <div style={{ flex: 1, minWidth: 0 }}>
              <span style={{ fontSize: 13, fontWeight: 500 }}>{expert.name}</span>
              <span style={{ fontSize: 11, color: "var(--muted)", marginLeft: 8 }}>
                {expert.description || expert.id.replace("agency-stock-analysis-", "")}
              </span>
            </div>
            <Tag color={expert.is_enabled ? "green" : "default"} style={{ fontSize: 10 }}>
              {expert.is_enabled ? t("common.enabled") : t("common.disabled")}
            </Tag>
          </div>
          {expandedId === expert.id && (
            <div style={{ marginTop: 8 }}>
              <Input.TextArea
                value={editText}
                onChange={(e) => setEditText(e.target.value)}
                rows={12}
                style={{ fontFamily: "monospace", fontSize: 12 }}
              />
              <div style={{ display: "flex", gap: 8, marginTop: 6, justifyContent: "flex-end" }}>
                <Button size="small" onClick={() => setExpandedId(null)}>{t("common.cancel")}</Button>
                <Button size="small" type="primary" loading={saving} onClick={save}>{t("common.save")}</Button>
              </div>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
