import { invoke } from "@/lib/invoke";
import { LintResult, useLlmWikiStore } from "@/stores/feature/llmWikiStore";
import { useWikiStore } from "@/stores/feature/wikiStore";
import {
  CheckCircleOutlined,
  EyeOutlined,
  InfoCircleOutlined,
  ReloadOutlined,
  WarningOutlined,
} from "@ant-design/icons";
import { Button, Card, Empty, message, Modal, Progress, Select, Space, Table, Tag, Tooltip, Typography } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface LintReportProps {
  wikiId: string;
}

export function LintReport({ wikiId }: LintReportProps) {
  const { t } = useTranslation();
  const { lintNote, updateLintScore } = useLlmWikiStore();
  const { notes } = useWikiStore();
  const [lintResults, setLintResults] = useState<LintResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedNoteId, setSelectedNoteId] = useState<string | null>(null);
  const [isDetailModalOpen, setIsDetailModalOpen] = useState(false);
  const [currentResult, setCurrentResult] = useState<LintResult | null>(null);

  useEffect(() => {
    loadLintResults();
  }, [wikiId]);

  const loadLintResults = async () => {
    setLoading(true);
    try {
      const results = await invoke<LintResult[]>("llm_wiki_lint_vault", { wikiId });
      setLintResults(results);
    } catch (e) {
      message.error(t("wiki.lint.loadError", { error: String(e) }));
    } finally {
      setLoading(false);
    }
  };

  const handleLintNote = async (noteId: string) => {
    const result = await lintNote(noteId);
    if (result) {
      setLintResults((prev) => {
        const index = prev.findIndex((r) => r.note_id === noteId);
        if (index >= 0) {
          const updated = [...prev];
          updated[index] = result;
          return updated;
        }
        return [...prev, result];
      });
      message.success(t("wiki.lint.success"));
    }
  };

  const handleUpdateScore = async (noteId: string) => {
    const score = await updateLintScore(noteId);
    if (score !== null) {
      message.success(t("wiki.lint.scoreUpdated", { score }));
    }
  };

  const getSeverityIcon = (severity: string) => {
    switch (severity) {
      case "Error":
        return <WarningOutlined style={{ color: "#ff4d4f" }} />;
      case "Warning":
        return <WarningOutlined style={{ color: "#faad14" }} />;
      case "Info":
        return <InfoCircleOutlined style={{ color: "#1890ff" }} />;
      default:
        return null;
    }
  };

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case "Error":
        return "red";
      case "Warning":
        return "orange";
      case "Info":
        return "blue";
      default:
        return "default";
    }
  };

  const columns = [
    {
      title: t("wiki.lint.note"),
      dataIndex: "note_id",
      key: "note_id",
      render: (noteId: string) => {
        const note = notes.find((n) => n.id === noteId);
        return note?.title || noteId;
      },
    },
    {
      title: t("wiki.lint.score"),
      dataIndex: "score",
      key: "score",
      width: 120,
      render: (score: number) => {
        const color = score >= 80 ? "#52c41a" : score >= 60 ? "#faad14" : "#ff4d4f";
        return (
          <Progress
            percent={score}
            size="small"
            strokeColor={color}
            format={(p) => `${p}`}
            style={{ width: 80 }}
          />
        );
      },
    },
    {
      title: t("wiki.lint.issues"),
      key: "issues",
      render: (_: unknown, record: LintResult) => {
        const errorCount = record.issues.filter((i) => i.severity === "Error").length;
        const warningCount = record.issues.filter((i) => i.severity === "Warning").length;
        const infoCount = record.issues.filter((i) => i.severity === "Info").length;

        return (
          <Space>
            {errorCount > 0 && <Tag color="red">{errorCount} errors</Tag>}
            {warningCount > 0 && <Tag color="orange">{warningCount} warnings</Tag>}
            {infoCount > 0 && <Tag color="blue">{infoCount} info</Tag>}
            {record.issues.length === 0 && (
              <Tag icon={<CheckCircleOutlined />} color="success">
                {t("wiki.lint.noIssues")}
              </Tag>
            )}
          </Space>
        );
      },
    },
    {
      title: t("wiki.common.actions"),
      key: "actions",
      width: 180,
      render: (_: unknown, record: LintResult) => (
        <Space>
          <Tooltip title={t("wiki.lint.viewDetails")}>
            <Button
              size="small"
              icon={<EyeOutlined />}
              onClick={() => {
                setCurrentResult(record);
                setIsDetailModalOpen(true);
              }}
            />
          </Tooltip>
          <Tooltip title={t("wiki.lint.rerun")}>
            <Button size="small" icon={<ReloadOutlined />} onClick={() => handleLintNote(record.note_id)} />
          </Tooltip>
          <Tooltip title={t("wiki.lint.updateScore")}>
            <Button size="small" onClick={() => handleUpdateScore(record.note_id)}>
              {t("wiki.lint.updateScore")}
            </Button>
          </Tooltip>
        </Space>
      ),
    },
  ];

  const averageScore = lintResults.length > 0
    ? Math.round(lintResults.reduce((sum, r) => sum + r.score, 0) / lintResults.length)
    : 0;

  const totalIssues = lintResults.reduce((sum, r) => sum + r.issues.length, 0);

  return (
    <Space direction="vertical" size="large" style={{ width: "100%" }}>
      <Card>
        <Space style={{ marginBottom: 16 }}>
          <Select
            placeholder={t("wiki.lint.selectNote")}
            allowClear
            style={{ width: 300 }}
            onChange={setSelectedNoteId}
            options={notes.map((n) => ({ label: n.title, value: n.id }))}
          />
          <Button
            type="primary"
            disabled={!selectedNoteId}
            onClick={() => selectedNoteId && handleLintNote(selectedNoteId)}
          >
            {t("wiki.lint.runLint")}
          </Button>
          <Button onClick={loadLintResults} loading={loading}>
            {t("wiki.common.refresh")}
          </Button>
        </Space>

        <div style={{ marginBottom: 16 }}>
          <Space split={<span style={{ color: "#d9d9d9" }}>|</span>}>
            <Text>
              {t("wiki.lint.totalNotes")}: <strong>{lintResults.length}</strong>
            </Text>
            <Text>
              {t("wiki.lint.averageScore")}: <strong>{averageScore}</strong>
            </Text>
            <Text>
              {t("wiki.lint.totalIssues")}: <strong>{totalIssues}</strong>
            </Text>
          </Space>
        </div>

        <Table
          dataSource={lintResults}
          rowKey="note_id"
          columns={columns}
          loading={loading}
          pagination={{ pageSize: 10 }}
          locale={{
            emptyText: (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description={t("wiki.lint.noResults")}
              />
            ),
          }}
        />
      </Card>

      <Modal
        title={t("wiki.lint.details")}
        open={isDetailModalOpen}
        onCancel={() => setIsDetailModalOpen(false)}
        footer={[
          <Button key="close" onClick={() => setIsDetailModalOpen(false)}>
            {t("wiki.common.close")}
          </Button>,
        ]}
        width={700}
      >
        {currentResult && (
          <Space direction="vertical" size="middle" style={{ width: "100%" }}>
            <Card size="small">
              <Space>
                <Text strong>{t("wiki.lint.note")}:</Text>
                <Text>{currentResult.note_id}</Text>
                <Progress
                  percent={currentResult.score}
                  size="small"
                  style={{ width: 100 }}
                  strokeColor={currentResult.score >= 80
                    ? "#52c41a"
                    : currentResult.score >= 60
                    ? "#faad14"
                    : "#ff4d4f"}
                />
              </Space>
            </Card>

            {currentResult.issues.length === 0
              ? <Empty description={t("wiki.lint.noIssues")} image={Empty.PRESENTED_IMAGE_SIMPLE} />
              : (
                <Table
                  dataSource={currentResult.issues}
                  rowKey="code"
                  size="small"
                  pagination={false}
                  columns={[
                    {
                      title: t("wiki.lint.severity"),
                      dataIndex: "severity",
                      key: "severity",
                      width: 100,
                      render: (severity: string) => (
                        <Space>
                          {getSeverityIcon(severity)}
                          <Tag color={getSeverityColor(severity)}>{severity}</Tag>
                        </Space>
                      ),
                    },
                    {
                      title: t("wiki.lint.code"),
                      dataIndex: "code",
                      key: "code",
                      width: 120,
                      render: (code: string) => <Text code>{code}</Text>,
                    },
                    {
                      title: t("wiki.lint.message"),
                      dataIndex: "message",
                      key: "message",
                    },
                    {
                      title: t("wiki.lint.line"),
                      dataIndex: "line",
                      key: "line",
                      width: 60,
                      render: (line?: number) => (line ? `#${line}` : "-"),
                    },
                  ]}
                />
              )}
          </Space>
        )}
      </Modal>
    </Space>
  );
}
