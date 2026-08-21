// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { PlusOutlined, ProjectOutlined } from "@ant-design/icons";
import { Button, Card, Col, Descriptions, Empty, message, Modal, Row, Space, Tag, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { KanbanBoard, KanbanItem, SirResult } from "../utils/constants";
import { KANBAN_COLUMNS } from "../utils/constants";

const { Text } = Typography;

export function KanbanTab() {
  const { t } = useTranslation();
  const [board, setBoard] = useState<KanbanBoard>({});
  const [loading, setLoading] = useState(false);
  const [acting, setActing] = useState<string | null>(null);
  const [sirRunning, setSirRunning] = useState(false);
  const [sirResult, setSirResult] = useState<SirResult | null>(null);
  const [sirModalOpen, setSirModalOpen] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const data = await invoke<KanbanBoard>("opc_kanban_board");
      setBoard(data);
    } catch (e) {
      message.error(t("opc.common.loadFailed", { error: String(e) }));
      setBoard({});
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const act = async (id: string, cmd: string, extra?: Record<string, unknown>) => {
    setActing(id);
    try {
      await invoke(cmd, { id, ...extra });
      message.success(t("opc.kanban.opSuccess", { cmd }));
      refresh();
    } catch (e) {
      message.error(t("opc.kanban.opFailed", { cmd, error: String(e) }));
    } finally {
      setActing(null);
    }
  };

  const runSIR = async (id: string) => {
    setActing(id);
    setSirRunning(true);
    try {
      const result = await invoke<SirResult>("run_self_improving_opc_work_item", {
        task: id,
        maxRounds: 3,
      });
      setSirResult(result);
      setSirModalOpen(true);
    } catch (e) {
      message.error(t("opc.kanban.sirRunFailed", { error: String(e) }));
    } finally {
      setActing(null);
      setSirRunning(false);
    }
  };

  const createItem = async () => {
    const title = window.prompt(t("opc.kanban.createPrompt"), "");
    if (!title || !title.trim()) { return; }
    setActing("new");
    try {
      await invoke("opc_create_work_item", { title: title.trim() });
      message.success(t("opc.kanban.createSuccess"));
      refresh();
    } catch (e) {
      message.error(t("opc.kanban.createFailed", { error: String(e) }));
    } finally {
      setActing(null);
    }
  };

  return (
    <div>
      <Space style={{ marginBottom: 12 }}>
        <Button size="small" type="primary" icon={<ProjectOutlined />} onClick={refresh} loading={loading}>
          {t("opc.kanban.refresh")}
        </Button>
        <Button size="small" icon={<PlusOutlined />} onClick={createItem} loading={acting === "new"}>
          {t("opc.kanban.create")}
        </Button>
        <Text type="secondary">{t("opc.kanban.machineDesc")}</Text>
      </Space>
      <Row gutter={[12, 12]}>
        {KANBAN_COLUMNS.map((col) => {
          const items = board[col] ?? [];
          const colLabel = t(col);
          return (
            <Col key={col} xs={24} sm={12} md={4}>
              <Card
                size="small"
                title={
                  <Space>
                    {colLabel}
                    <Tag
                      color={colLabel === t("opc.kanban.colBlocked")
                        ? "red"
                        : colLabel === t("opc.kanban.colDone")
                        ? "green"
                        : "blue"}
                    >
                      {items.length}
                    </Tag>
                  </Space>
                }
                style={{ minHeight: 200, background: colLabel === t("opc.kanban.colBlocked") ? "#fff2f0" : undefined }}
              >
                {items.length === 0
                  ? <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={t("opc.kanban.empty")} />
                  : (
                    <Space direction="vertical" style={{ width: "100%" }}>
                      {items.map((it: KanbanItem) => (
                        <Card key={it.id} size="small" styles={{ body: { padding: 8 } }}>
                          <Text strong style={{ fontSize: 12 }}>
                            {it.title}
                          </Text>
                          <div style={{ fontSize: 11, color: "#888", marginTop: 4 }}>
                            <div>ID: {it.id}</div>
                            <div>{t("opc.kanban.owner", { id: it.owner_role_id ?? "-" })}</div>
                            {it.deps.length > 0 && <div>{t("opc.kanban.deps", { deps: it.deps.join(", ") })}</div>}
                            {it.last_error && <div style={{ color: "#cf1322" }}>⚠ {it.last_error}</div>}
                          </div>
                          <Space wrap style={{ marginTop: 6 }}>
                            <Button
                              size="small"
                              icon={<ProjectOutlined />}
                              loading={acting === it.id && sirRunning}
                              disabled={sirRunning && acting !== it.id}
                              onClick={() => runSIR(it.id)}
                              title={t("opc.kanban.sir")}
                            >
                              {sirRunning && acting === it.id ? t("opc.kanban.sirRunning") : t("opc.kanban.sir")}
                            </Button>
                            {it.phase === "QUEUED" && (
                              <Button
                                size="small"
                                type="primary"
                                loading={acting === it.id}
                                onClick={() => act(it.id, "opc_work_item_start")}
                              >
                                {t("opc.kanban.claim")}
                              </Button>
                            )}
                            {it.phase === "IN_PROGRESS" && (
                              <Button
                                size="small"
                                loading={acting === it.id}
                                onClick={() => act(it.id, "opc_work_item_review")}
                              >
                                {t("opc.kanban.submitReview")}
                              </Button>
                            )}
                            {it.phase === "REVIEW" && (
                              <Button
                                size="small"
                                type="primary"
                                loading={acting === it.id}
                                onClick={() => act(it.id, "opc_work_item_start")}
                              >
                                {t("opc.kanban.approveDone")}
                              </Button>
                            )}
                            {it.phase !== "BLOCKED"
                              && it.phase !== "DONE"
                              && it.phase !== "APPROVED"
                              && it.phase !== "FAILED"
                              && it.phase !== "CANCELLED" && (
                              <Button
                                size="small"
                                danger
                                loading={acting === it.id}
                                onClick={() => {
                                  const reason = window.prompt(
                                    t("opc.kanban.escalateReason"),
                                    t("opc.kanban.escalateDefault"),
                                  );
                                  if (reason !== null) { act(it.id, "opc_escalate_work_item", { reason }); }
                                }}
                              >
                                {t("opc.kanban.escalate")}
                              </Button>
                            )}
                            {it.phase === "BLOCKED" && (
                              <Button
                                size="small"
                                type="primary"
                                loading={acting === it.id}
                                onClick={() => act(it.id, "opc_work_item_unblock")}
                              >
                                {t("opc.kanban.unblock")}
                              </Button>
                            )}
                          </Space>
                        </Card>
                      ))}
                    </Space>
                  )}
              </Card>
            </Col>
          );
        })}
      </Row>

      <Modal
        open={sirModalOpen}
        title={t("opc.kanban.sirTitle")}
        onCancel={() => setSirModalOpen(false)}
        footer={null}
        width={720}
      >
        {sirResult && (
          <div>
            <Descriptions size="small" column={3} bordered style={{ marginBottom: 12 }}>
              <Descriptions.Item label={t("opc.kanban.sirScore")}>
                {(sirResult.finalScore * 100).toFixed(1)}%
              </Descriptions.Item>
              <Descriptions.Item label={t("opc.kanban.sirRounds")}>{sirResult.totalRounds}</Descriptions.Item>
              <Descriptions.Item label={t("opc.kanban.sirAccept")}>
                {sirResult.finalScore >= 0.85 ? "✅" : "⏳"}
              </Descriptions.Item>
              <Descriptions.Item label={t("opc.kanban.sirStrengths")} span={3}>
                {sirResult.strengths.length > 0 ? sirResult.strengths.join("；") : "-"}
              </Descriptions.Item>
              <Descriptions.Item label={t("opc.kanban.sirGaps")} span={3}>
                {sirResult.gaps.length > 0 ? sirResult.gaps.join("；") : "-"}
              </Descriptions.Item>
            </Descriptions>
            <pre
              style={{
                maxHeight: 320,
                overflow: "auto",
                fontSize: 12,
                background: "rgba(0,0,0,0.03)",
                padding: 12,
                borderRadius: 6,
                whiteSpace: "pre-wrap",
                wordBreak: "break-word",
              }}
            >
              {sirResult.text}
            </pre>
          </div>
        )}
      </Modal>
    </div>
  );
}
