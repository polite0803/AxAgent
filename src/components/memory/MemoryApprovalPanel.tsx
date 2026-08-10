// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type { MemoryWriteApprovalConfig, PendingMemoryWrite } from "@/types";
import { App, Button, Empty, Space, Switch, Tag, Tooltip } from "antd";
import { CheckCircle2, XCircle } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

/** 记忆写审批门面板（借鉴 Hermes 的 memory.write_approval） */
export function MemoryApprovalPanel() {
  const { t } = useTranslation();
  const { message } = App.useApp();

  const [pending, setPending] = useState<PendingMemoryWrite[]>([]);
  const [config, setConfig] = useState<MemoryWriteApprovalConfig | null>(null);

  async function load() {
    try {
      const [items, cfg] = await Promise.all([
        invoke<PendingMemoryWrite[]>("get_pending_memory_writes"),
        invoke<MemoryWriteApprovalConfig>("get_memory_write_approval_config"),
      ]);
      setPending(items);
      setConfig(cfg);
    } catch (e) {
      message.error(String(e));
    }
  }

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function handleApprove(id: string) {
    try {
      await invoke<void>("approve_memory_write", { approvalId: id });
      message.success(t("skillLearning.approved"));
      load();
    } catch (e) {
      message.error(String(e));
    }
  }

  async function handleReject(id: string) {
    try {
      await invoke<void>("reject_memory_write", { approvalId: id });
      message.success(t("skillLearning.rejected"));
      load();
    } catch (e) {
      message.error(String(e));
    }
  }

  async function toggleGate(enabled: boolean) {
    if (!config) {
      return;
    }
    try {
      const next = { ...config, enabled };
      await invoke<void>("update_memory_write_approval_config", { config: next });
      setConfig(next);
    } catch (e) {
      message.error(String(e));
    }
  }

  return (
    <div style={{ padding: 16 }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 12 }}>
        <b>{t("skillLearning.memApprovalTitle")}</b>
        <Space>
          <span style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>
            {t("skillLearning.memApprovalGate")}
          </span>
          <Switch checked={config?.enabled ?? false} onChange={toggleGate} />
        </Space>
      </div>

      {pending.length === 0
        ? <Empty description={t("skillLearning.noPending")} />
        : (
          <Space direction="vertical" style={{ width: "100%" }}>
            {pending.map((item) => (
              <div
                key={item.id}
                style={{ border: "1px solid var(--color-border-secondary)", borderRadius: 8, padding: 10 }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                  <Tag color="purple">{item.namespace ?? "memory"}</Tag>
                  <span style={{ flex: 1 }} />
                  <Tooltip title={item.reason}>
                    <span style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>{item.id}</span>
                  </Tooltip>
                </div>
                <div
                  style={{
                    fontSize: 12,
                    color: "var(--color-text-secondary)",
                    marginBottom: 8,
                    maxHeight: 60,
                    overflow: "auto",
                    whiteSpace: "pre-wrap",
                  }}
                >
                  {item.content}
                </div>
                <div style={{ display: "flex", gap: 8 }}>
                  <Button
                    size="small"
                    type="primary"
                    icon={<CheckCircle2 />}
                    onClick={() => handleApprove(item.id)}
                  >
                    {t("skillLearning.approve")}
                  </Button>
                  <Button size="small" danger icon={<XCircle />} onClick={() => handleReject(item.id)}>
                    {t("skillLearning.reject")}
                  </Button>
                </div>
              </div>
            ))}
          </Space>
        )}
    </div>
  );
}
