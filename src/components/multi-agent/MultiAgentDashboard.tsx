// SPDX-License-Identifier: AGPL-3.0-only
/**
 * G5 Multi-Agent 固定角色 pool 主面板
 *
 * 功能：
 * - 顶部：三个固定角色卡片（analyst / implementer / reviewer），展示角色信息
 * - 中部：委派任务表单（选择角色 + 输入任务 + 可选上下文 JSON + 选择供应商/模型）
 * - 底部：委派历史记录列表（含成功/失败状态、token 用量、耗时）
 *
 * 数据源：useMultiAgentStore
 */

import { useMultiAgentStore } from "@/stores/feature/multiAgentStore";
import { useProviderStore } from "@/stores/feature/providerStore";
import type { MultiAgentRoleInfo, ProviderConfig } from "@/types";
import {
  CheckCircleOutlined,
  ClearOutlined,
  CloseCircleOutlined,
  DeleteOutlined,
  SendOutlined,
  TeamOutlined,
} from "@ant-design/icons";
import {
  Alert,
  Button,
  Card,
  Col,
  Empty,
  Input,
  message,
  Row,
  Segmented,
  Select,
  Space,
  Spin,
  Tag,
  Typography,
} from "antd";
import dayjs from "dayjs";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph, Title } = Typography;

// ── 工具函数 ──

/** 角色徽标颜色 */
function roleColor(roleId: string): string {
  switch (roleId) {
    case "analyst": {
      return "blue";
    }
    case "implementer": {
      return "green";
    }
    case "reviewer": {
      return "orange";
    }
    default: {
      return "default";
    }
  }
}

/** 角色图标首字母 */
function roleInitial(name: string): string {
  return name?.charAt(0) ?? "?";
}

// ── 子组件：角色卡片 ──

interface RoleCardProps {
  role: MultiAgentRoleInfo;
  selected: boolean;
  onSelect: (id: string) => void;
}

function RoleCard({ role, selected, onSelect }: RoleCardProps) {
  const { t } = useTranslation();
  return (
    <Card
      hoverable
      size="small"
      onClick={() => onSelect(role.id)}
      style={{
        borderColor: selected ? "#1677ff" : undefined,
        borderWidth: selected ? 2 : 1,
        cursor: "pointer",
      }}
    >
      <Space orientation="vertical" style={{ width: "100%" }} size="small">
        <Space style={{ width: "100%", justifyContent: "space-between" }}>
          <Tag color={roleColor(role.id)} style={{ margin: 0 }}>
            {roleInitial(role.name)}
          </Tag>
          <Text strong>{role.name}</Text>
        </Space>
        <Paragraph style={{ marginBottom: 0, fontSize: 12, color: "#666" }}>
          {role.description}
        </Paragraph>
        <Space size="small">
          <Tag>
            {t("multiAgent.maxConcurrent")}: {role.maxConcurrent}
          </Tag>
          <Tag>
            {t("multiAgent.timeoutSec")}: {role.timeoutSeconds}s
          </Tag>
        </Space>
      </Space>
    </Card>
  );
}

// ── 子组件：委派任务表单 ──

interface DelegateFormProps {
  roles: MultiAgentRoleInfo[];
  onSubmit: (input: {
    roleName: string;
    task: string;
    context: string;
    providerId: string;
    modelId: string;
    temperature: number;
    maxTokens: number;
  }) => void;
  submitting: boolean;
}

function DelegateForm({ roles, onSubmit, submitting }: DelegateFormProps) {
  const { t } = useTranslation();
  const { providers, fetchProviders } = useProviderStore();
  const [roleName, setRoleName] = useState<string>("");
  const [task, setTask] = useState<string>("");
  const [contextJson, setContextJson] = useState<string>("");
  const [providerId, setProviderId] = useState<string>("");
  const [modelId, setModelId] = useState<string>("");
  const [temperature, setTemperature] = useState<number>(0.2);
  const [maxTokens, setMaxTokens] = useState<number>(2048);
  const [jsonError, setJsonError] = useState<string>("");

  // 启动时拉取供应商列表
  useEffect(() => {
    if (providers.length === 0) {
      fetchProviders();
    }
  }, [providers.length, fetchProviders]);

  // 默认选中第一个角色
  useEffect(() => {
    if (!roleName && roles.length > 0) {
      setRoleName(roles[0].id);
    }
  }, [roles, roleName]);

  // 默认选中第一个供应商
  useEffect(() => {
    if (!providerId && providers.length > 0) {
      setProviderId(providers[0].id);
    }
  }, [providers, providerId]);

  const currentProvider: ProviderConfig | undefined = providers.find(
    (p) => p.id === providerId,
  );
  const availableModels = currentProvider?.models ?? [];

  // 供应商变化时，默认选中第一个模型
  useEffect(() => {
    if (availableModels.length > 0 && !modelId) {
      setModelId(availableModels[0].modelId);
    }
  }, [availableModels, modelId]);

  const validateContext = (value: string): boolean => {
    if (!value.trim()) {
      setJsonError("");
      return true;
    }
    try {
      JSON.parse(value);
      setJsonError("");
      return true;
    } catch (e) {
      setJsonError(String(e instanceof Error ? e.message : e));
      return false;
    }
  };

  const handleSubmit = () => {
    if (!roleName || !task.trim() || !providerId || !modelId) {
      message.warning(t("multiAgent.fillRequired"));
      return;
    }
    if (!validateContext(contextJson)) {
      message.warning(t("multiAgent.contextJsonInvalid"));
      return;
    }
    onSubmit({
      roleName,
      task: task.trim(),
      context: contextJson.trim(),
      providerId,
      modelId,
      temperature,
      maxTokens,
    });
  };

  return (
    <Card title={t("multiAgent.delegateTitle")} style={{ marginBottom: 16 }}>
      <Space orientation="vertical" style={{ width: "100%" }} size="middle">
        {/* 角色选择 */}
        <div>
          <Text type="secondary">{t("multiAgent.selectRole")}</Text>
          <Segmented
            block
            value={roleName}
            onChange={(v) => setRoleName(v as string)}
            options={roles.map((r) => ({
              label: r.name,
              value: r.id,
            }))}
          />
        </div>

        {/* 任务描述 */}
        <div>
          <Text type="secondary">{t("multiAgent.taskDescription")}</Text>
          <Input.TextArea
            value={task}
            onChange={(e) => setTask(e.target.value)}
            rows={4}
            placeholder={t("multiAgent.taskPlaceholder")}
          />
        </div>

        {/* 可选上下文 JSON */}
        <div>
          <Text type="secondary">{t("multiAgent.contextJsonOptional")}</Text>
          <Input.TextArea
            value={contextJson}
            onChange={(e) => {
              setContextJson(e.target.value);
              validateContext(e.target.value);
            }}
            rows={4}
            placeholder={t("multiAgent.contextJsonPlaceholder")}
            style={{ fontFamily: "monospace" }}
          />
          {jsonError && (
            <Alert
              type="error"
              title={jsonError}
              style={{ marginTop: 4 }}
              showIcon
            />
          )}
        </div>

        {/* 供应商/模型/温度/最大 tokens */}
        <Row gutter={16}>
          <Col span={6}>
            <Text type="secondary">{t("multiAgent.provider")}</Text>
            <Select
              style={{ width: "100%" }}
              value={providerId}
              onChange={setProviderId}
              options={providers.map((p) => ({ label: p.name, value: p.id }))}
            />
          </Col>
          <Col span={6}>
            <Text type="secondary">{t("multiAgent.model")}</Text>
            <Select
              style={{ width: "100%" }}
              value={modelId}
              onChange={setModelId}
              options={availableModels.map((m) => ({
                label: m.name,
                value: m.modelId,
              }))}
            />
          </Col>
          <Col span={6}>
            <Text type="secondary">{t("multiAgent.temperature")}</Text>
            <Select
              style={{ width: "100%" }}
              value={temperature}
              onChange={(v) => setTemperature(v as number)}
              options={[
                { label: `0.0 (${t("multiAgent.temperatureStrict")})`, value: 0.0 },
                { label: `0.2 (${t("multiAgent.temperatureDefault")})`, value: 0.2 },
                { label: "0.5", value: 0.5 },
                { label: `0.8 (${t("multiAgent.temperatureDivergent")})`, value: 0.8 },
              ]}
            />
          </Col>
          <Col span={6}>
            <Text type="secondary">{t("multiAgent.maxTokens")}</Text>
            <Select
              style={{ width: "100%" }}
              value={maxTokens}
              onChange={(v) => setMaxTokens(v as number)}
              options={[
                { label: "1024", value: 1024 },
                { label: `2048 (${t("multiAgent.maxTokensDefault")})`, value: 2048 },
                { label: "4096", value: 4096 },
                { label: "8192", value: 8192 },
              ]}
            />
          </Col>
        </Row>

        <Button
          type="primary"
          icon={<SendOutlined />}
          onClick={handleSubmit}
          loading={submitting}
          disabled={!roleName || !task.trim() || !providerId || !modelId}
        >
          {t("multiAgent.delegateBtn")}
        </Button>
      </Space>
    </Card>
  );
}

// ── 子组件：历史记录卡片 ──

interface HistoryItemProps {
  // 重新声明字段以避免类型循环
  entry: {
    delegationId: string;
    roleName: string;
    task: string;
    content: string;
    timestamp: number;
    durationMs: number;
    promptTokens: number;
    completionTokens: number;
    success: boolean;
    error?: string;
  };
}

function HistoryItem({ entry }: HistoryItemProps) {
  const { t } = useTranslation();
  return (
    <Card size="small" style={{ marginBottom: 8 }}>
      <Space orientation="vertical" style={{ width: "100%" }} size="small">
        <Space style={{ width: "100%", justifyContent: "space-between" }}>
          <Space>
            <Tag color={roleColor(entry.roleName)}>{entry.roleName}</Tag>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {dayjs(entry.timestamp).format("YYYY-MM-DD HH:mm:ss")}
            </Text>
            {entry.success
              ? (
                <Tag icon={<CheckCircleOutlined />} color="green">
                  {t("multiAgent.success")}
                </Tag>
              )
              : (
                <Tag icon={<CloseCircleOutlined />} color="red">
                  {t("multiAgent.failed")}
                </Tag>
              )}
          </Space>
          <Space size="small">
            <Tag>{entry.durationMs}ms</Tag>
            <Tag>
              {t("multiAgent.tokens")}: {entry.promptTokens}+{entry.completionTokens}
            </Tag>
          </Space>
        </Space>
        <div>
          <Text strong>{t("multiAgent.taskLabel")}:</Text>
          <Text>{entry.task}</Text>
        </div>
        {entry.success
          ? (
            entry.content && (
              <div>
                <Text strong>{t("multiAgent.responseLabel")}:</Text>
                <Paragraph
                  style={{
                    marginTop: 4,
                    marginBottom: 0,
                    whiteSpace: "pre-wrap",
                    maxHeight: 200,
                    overflow: "auto",
                    background: "#fafafa",
                    padding: 8,
                    borderRadius: 4,
                  }}
                >
                  {entry.content}
                </Paragraph>
              </div>
            )
          )
          : (
            <Alert
              type="error"
              title={entry.error ?? "Unknown error"}
              showIcon
            />
          )}
      </Space>
    </Card>
  );
}

// ── 主组件 ──

export function MultiAgentDashboard() {
  const { t } = useTranslation();
  const store = useMultiAgentStore();
  const [selectedRole, setSelectedRole] = useState<string>("");

  useEffect(() => {
    store.fetchRoles();
  }, []);

  // 当角色列表加载完毕后，默认选中第一个
  useEffect(() => {
    if (!selectedRole && store.roles.length > 0) {
      setSelectedRole(store.roles[0].id);
    }
  }, [store.roles, selectedRole]);

  const handleDelegate = async (input: {
    roleName: string;
    task: string;
    context: string;
    providerId: string;
    modelId: string;
    temperature: number;
    maxTokens: number;
  }) => {
    let context: Record<string, unknown> | null = null;
    if (input.context) {
      try {
        context = JSON.parse(input.context) as Record<string, unknown>;
      } catch {
        // DelegateForm 已校验，这里兜底
        message.error(t("multiAgent.contextJsonInvalid"));
        return;
      }
    }
    try {
      await store.delegateTask({
        roleName: input.roleName,
        task: input.task,
        context,
        providerId: input.providerId,
        modelId: input.modelId,
        temperature: input.temperature,
        maxTokens: input.maxTokens,
      });
      message.success(t("multiAgent.delegateSuccess"));
    } catch (e) {
      message.error(`${t("multiAgent.delegateFailed")}: ${e}`);
    }
  };

  const sortedHistory = useMemo(() => store.history, [store.history]);

  return (
    <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column", padding: 16, overflow: "auto" }}>
      {/* 顶部标题 */}
      <Card size="small" style={{ marginBottom: 16 }}>
        <Space>
          <TeamOutlined style={{ fontSize: 20, color: "#1677ff" }} />
          <Title level={5} style={{ margin: 0 }}>
            {t("multiAgent.title")}
          </Title>
          <Text type="secondary">{t("multiAgent.subtitle")}</Text>
        </Space>
      </Card>

      {/* 角色卡片 */}
      <Card
        title={t("multiAgent.rolesTitle")}
        extra={
          <Button
            size="small"
            icon={<ClearOutlined />}
            onClick={() => store.fetchRoles()}
            loading={store.loadingRoles}
          >
            {t("multiAgent.refreshRoles")}
          </Button>
        }
        style={{ marginBottom: 16 }}
      >
        {store.loadingRoles
          ? (
            <div style={{ textAlign: "center", padding: 24 }}>
              <Spin />
            </div>
          )
          : store.roles.length === 0
          ? <Empty description={t("multiAgent.noRoles")} />
          : (
            <Row gutter={[12, 12]}>
              {store.roles.map((role) => (
                <Col key={role.id} xs={24} sm={12} md={8}>
                  <RoleCard
                    role={role}
                    selected={selectedRole === role.id}
                    onSelect={setSelectedRole}
                  />
                </Col>
              ))}
            </Row>
          )}
      </Card>

      {/* 委派表单 */}
      <DelegateForm
        roles={store.roles}
        onSubmit={handleDelegate}
        submitting={store.delegating}
      />

      {/* 历史记录 */}
      <Card
        title={t("multiAgent.historyTitle")}
        extra={store.history.length > 0 && (
          <Button
            size="small"
            danger
            icon={<DeleteOutlined />}
            onClick={() => store.clearHistory()}
          >
            {t("multiAgent.clearHistory")}
          </Button>
        )}
      >
        {store.error && (
          <Alert
            type="error"
            title={store.error}
            style={{ marginBottom: 8 }}
            showIcon
            closable
            onClose={() => store.clearError()}
          />
        )}
        {sortedHistory.length === 0
          ? <Empty description={t("multiAgent.noHistory")} />
          : (
            sortedHistory.map((entry) => <HistoryItem key={entry.delegationId} entry={entry} />)
          )}
      </Card>
    </div>
  );
}
