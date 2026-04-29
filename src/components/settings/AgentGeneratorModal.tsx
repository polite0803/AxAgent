import { useState } from "react";
import { Button, Input, Modal, Spin, Typography, App, Alert } from "antd";
import { Sparkles, Check, Edit3 } from "lucide-react";
import { invoke } from "@/lib/invoke";

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
  const [description, setDescription] = useState("");
  const [generating, setGenerating] = useState(false);
  const [result, setResult] = useState<GeneratedAgentConfig | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { message } = App.useApp();

  const handleGenerate = async () => {
    if (!description.trim()) return;

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
        throw new Error("无法从响应中解析 JSON");
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
          throw new Error(`缺少必填字段: ${field}`);
        }
      }

      setResult(config);
      message.success("智能体配置生成成功");
    } catch (err) {
      const msg = err instanceof Error ? err.message : "生成失败";
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
          <span>自然语言创建智能体</span>
        </div>
      }
      open={open}
      onCancel={handleClose}
      footer={null}
      width={600}
      destroyOnClose
    >
      {!result ? (
        <div className="flex flex-col gap-4">
          <Text type="secondary">
            用自然语言描述你需要的智能体，AI 将自动生成完整的配置。
          </Text>
          <TextArea
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="例如：我需要一个专门审查 SQL 查询安全性的智能体..."
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
            <Button onClick={handleClose}>取消</Button>
            <Button
              type="primary"
              icon={<Sparkles size={16} />}
              onClick={handleGenerate}
              loading={generating}
              disabled={!description.trim()}
            >
              生成配置
            </Button>
          </div>
          {generating && (
            <div className="flex justify-center py-4">
              <Spin tip="AI 正在生成智能体配置..." />
            </div>
          )}
        </div>
      ) : (
        <div className="flex flex-col gap-4">
          <Alert
            type="success"
            message="配置生成完成"
            description="请检查并编辑后保存"
            showIcon
          />

          <div className="flex flex-col gap-2 p-3 rounded bg-gray-50 dark:bg-gray-800">
            <div><Text strong>类型：</Text><Text code>{result.agent_type}</Text></div>
            <div><Text strong>名称：</Text><Text>{result.display_name}</Text></div>
            <div><Text strong>描述：</Text><Text type="secondary">{result.description}</Text></div>
            <div><Text strong>权限：</Text>
              {result.permissions.map((p) => (
                <Text key={p} code style={{ marginRight: 4 }}>{p}</Text>
              ))}
            </div>
            <div><Text strong>推荐模型：</Text><Text code>{result.preferred_model}</Text></div>
            <div>
              <Text strong>系统提示：</Text>
              <Paragraph
                ellipsis={{ rows: 4, expandable: true, symbol: "展开" }}
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
              重新编辑
            </Button>
            <Button type="primary" icon={<Check size={16} />} onClick={handleSave}>
              保存配置
            </Button>
          </div>
        </div>
      )}
    </Modal>
  );
}
