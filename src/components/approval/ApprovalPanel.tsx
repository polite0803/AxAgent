// SPDX-License-Identifier: AGPL-3.0-only

import { listen, logIpcError } from "@/lib/invoke";
import { useApprovalStore } from "@/stores";
import type { ApprovalRequest } from "@/types";
import { Badge, Empty, Input, Modal, Space, Spin } from "antd";
import { Eye } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { ApprovalCard } from "./ApprovalCard";

export function ApprovalPanel() {
  const { t } = useTranslation();
  const pendingApprovals = useApprovalStore((s) => s.pendingApprovals);
  const loading = useApprovalStore((s) => s.loading);
  const panelOpen = useApprovalStore((s) => s.panelOpen);
  const setPanelOpen = useApprovalStore((s) => s.setPanelOpen);
  const fetchPendingApprovals = useApprovalStore((s) => s.fetchPendingApprovals);

  const [note, setNote] = useState("");

  useEffect(() => {
    if (panelOpen) {
      fetchPendingApprovals().catch(logIpcError("ApprovalPanel: fetchPendingApprovals"));
    }
  }, [panelOpen, fetchPendingApprovals]);

  // 推送式唤醒：后端统一事件总线桥接的审批请求事件（workflow:approval-requested）
  // 到达时主动刷新待审批列表并打开面板，替代纯轮询。
  useEffect(() => {
    const unlistenPromise = listen<unknown>("workflow:approval-requested", () => {
      fetchPendingApprovals().catch(logIpcError("ApprovalPanel: on-approval-requested"));
      setPanelOpen(true);
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten()).catch(() => {});
    };
  }, [fetchPendingApprovals, setPanelOpen]);

  const handleApproved = useCallback(
    (_approvalId: string) => {
      fetchPendingApprovals().catch(logIpcError("ApprovalPanel: fetchPendingApprovals"));
      setNote("");
    },
    [fetchPendingApprovals],
  );

  const handleRejected = useCallback(
    (_approvalId: string) => {
      fetchPendingApprovals().catch(logIpcError("ApprovalPanel: fetchPendingApprovals"));
      setNote("");
    },
    [fetchPendingApprovals],
  );

  const pendingCount = pendingApprovals.filter((a) => a.status === "pending").length;

  return (
    <Modal
      title={
        <Space>
          <Eye size={18} />
          <span>{t("approval.panelTitle")}</span>
          {pendingCount > 0 && <Badge count={pendingCount} size="small" />}
        </Space>
      }
      open={panelOpen}
      onCancel={() => setPanelOpen(false)}
      width={600}
      footer={null}
      destroyOnHidden
    >
      <Input.TextArea
        placeholder={t("approval.notePlaceholder")}
        value={note}
        onChange={(e) => setNote(e.target.value)}
        rows={2}
        style={{ marginBottom: 12, fontSize: 12 }}
      />

      <Spin spinning={loading}>
        {pendingApprovals.length === 0 && !loading
          ? <Empty description={t("approval.noPending")} image={Empty.PRESENTED_IMAGE_SIMPLE} />
          : (
            <div className="flex flex-col gap-2 max-h-[400px] overflow-y-auto pr-1">
              {pendingApprovals.map((approval: ApprovalRequest) => (
                <ApprovalCard
                  key={approval.id}
                  approval={approval}
                  note={note}
                  onApproved={handleApproved}
                  onRejected={handleRejected}
                />
              ))}
            </div>
          )}
      </Spin>
    </Modal>
  );
}
