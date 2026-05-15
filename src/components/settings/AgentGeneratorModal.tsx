import { invoke } from "@/lib/invoke";
import { Alert, App, Button, Input, Modal, Spin, Typography } from "antd";
import { Check, Edit3, Sparkles } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const { TextArea } = Input;
const { Text, Paragraph } = Typography;

const AGENT_GENERATE_META_PROMPT = `你是一个智能体配置生成器。根据用户的自然语言描述，生成 JSON 格式的智能体定义。

必须包含以下字段：
- agent_type: 智能体类型标识符（英文小写，用连字符分隔）
- display_name: 中文显示名称
- description: 一句话描述此智能体的用途
- system_prompt: 详细的系统提示词（用中文描述智能体的角色、能力、行为准则）
- permissions: 权限列表（可选值："read", "write", "bash", "network", "grep"）
- preferred_model: 推荐使用的模型

只输出 JSON，不要有其他内容。

用户描述：`;

export interface GeneratedAgentConfig {
  agent_type: string;
  display_name: string;
  description: string;
  system_prompt: string;
  permissions: string[];
  preferred_model: string;
}

interface AgentGeneratorModalProps {
  open: boolean;
  onClose: () => void;
  onSave: (config: GeneratedAgentConfig) => void;
  conversationId: string;
}

export function AgentGeneratorModal({ open, onClose, onSave, conversationId }: AgentGeneratorModalProps) {
  const { t } = useTranslation();
  const [description, setDescription] = useState("");
  const [generating, setGenerating] = useState(false);
  const [result, setResult] = useState<GeneratedAgentConfig | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { message } = App.useApp();

  const handleGenerate = async () => {
    if (!description.trim()) { return; }

    setGenerating(true);
    setError(null);
    setResult(null);

    try {
      const prompt = AGENT_GENERATE_META_PROMPT + description.trim();
      // Use a one-shot completion to generate the agent config
      const response = await invoke<string>("simple_chat_completion", {
        input: {
          conversation_id: conversationId,
          messages: [
            { role: "system", content: "只输出 JSON，不要有其他内容。" },
            { role: "user", content: prompt },
          ],
          temperature: 0.3,
          max_tokens: 4000,
        },
      });

      // Try to extract JSON from the response
      const jsonMatch = response.match(/\{[\s\S]*\}/);
      if (!jsonMatch) {
        throw new Error(t("agentGenerator.parseJsonError"));
      }

      const config: GeneratedAgentConfig = JSON.parse(jsonMatch[0]);

      // Validate required fields
      const requiredFields: (keyof GeneratedAgentConfig)[] = [
        "agent_type",
        "display_name",
        "description",
        "system_prompt",
        "permissions",
        "preferred_model",
      ];
      for (const field of requiredFields) {
        if (!config[field]) {
          throw new Error(t("agentGenerator.missingField", { field }));
        }
      }

      setResult(config);
      message.success(t("agentGenerator.success"));
    } catch (err) {
      const msg = err instanceof Error ? err.message : t("agentGenerator.failed");
      setError(msg);
      message.error(msg);
    } finally {
      setGenerating(false);
    }
  };

  const handleSave = () => {
    if (result) {
      onSave(result);
      handleClose();
    }
  };

  const handleClose = () => {
    setDescription("");
    setResult(null);
    setError(null);
    onClose();
  };

  return (
    <Modal
      title={
        <div className="flex items-center gap-2">
          <Sparkles size={18} />
          <span>{t("agentGenerator.title")}</span>
        </div>
      }
      open={open}
      onCancel={handleClose}
      footer={null}
      width={600}
      destroyOnHidden
    >
      {!result
        ? (
          <div className="flex flex-col gap-4">
            <Text type="secondary">
              {t("agentGenerator.desc")}
            </Text>
            <TextArea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={t("agentGenerator.placeholder")}
              rows={4}
              autoFocus
            />
            {error && (
              <Alert
                type="error"
                message={error}
                showIcon
                closable
                onClose={() => setError(null)}
              />
            )}
            <div className="flex justify-end gap-2">
              <Button onClick={handleClose}>{t("common.cancel")}</Button>
              <Button
                type="primary"
                icon={<Sparkles size={16} />}
                onClick={handleGenerate}
                loading={generating}
                disabled={!description.trim()}
              >
                {t("agentGenerator.generate")}
              </Button>
            </div>
            {generating && (
              <div className="flex justify-center py-4">
                <Spin tip={t("agentGenerator.generating")} />
              </div>
            )}
          </div>
        )
        : (
          <div className="flex flex-col gap-4">
            <Alert
              type="success"
              message={t("agentGenerator.generatedTitle")}
              description={t("agentGenerator.generatedDesc")}
              showIcon
            />

            <div className="flex flex-col gap-2 p-3 rounded bg-gray-50 dark:bg-gray-800">
              <div>
                <Text strong>{t("agentGenerator.field.type")}：</Text>
                <Text code>{result.agent_type}</Text>
              </div>
              <div>
                <Text strong>{t("agentGenerator.field.name")}：</Text>
                <Text>{result.display_name}</Text>
              </div>
              <div>
                <Text strong>{t("agentGenerator.field.desc")}：</Text>
                <Text type="secondary">{result.description}</Text>
              </div>
              <div>
                <Text strong>{t("agentGenerator.field.permissions")}：</Text>
                {result.permissions.map((p) => <Text key={p} code style={{ marginRight: 4 }}>{p}</Text>)}
              </div>
              <div>
                <Text strong>{t("agentGenerator.field.recommendedModel")}：</Text>
                <Text code>{result.preferred_model}</Text>
              </div>
              <div>
                <Text strong>{t("agentGenerator.field.systemPrompt")}：</Text>
                <Paragraph
                  ellipsis={{ rows: 4, expandable: true, symbol: t("agentGenerator.expand") }}
                  type="secondary"
                  style={{ marginTop: 4 }}
                >
                  {result.system_prompt}
                </Paragraph>
              </div>
            </div>

            <div className="flex justify-end gap-2">
              <Button
                icon={<Edit3 size={14} />}
                onClick={() => {
                  setResult(null);
                  setError(null);
                }}
              >
                {t("agentGenerator.reedit")}
              </Button>
              <Button type="primary" icon={<Check size={16} />} onClick={handleSave}>
                {t("agentGenerator.save")}
              </Button>
            </div>
          </div>
        )}
    </Modal>
  );
}
