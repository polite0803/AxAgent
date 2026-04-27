import { Button, Card, Collapse, Tag, Tooltip, Typography } from "antd";
import {
  Brain,
  CheckCircle,
  XCircle,
  ChevronRight,
  Loader2,
  AlertTriangle,
  Lightbulb,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";

const { Text } = Typography;

interface ThoughtStep {
  id: number;
  state: string;
  reasoning: string;
  action: {
    action_type: string;
    tool_name?: string;
    tool_input?: Record<string, unknown>;
    llm_prompt?: string;
    requires_confirmation: boolean;
  } | null;
  observation?: string;
  result?: string;
  is_verified: boolean;
  timestamp: string;
}

interface ChainSummary {
  total_steps: number;
  iterations: number;
  current_state: string;
  steps: ThoughtStep[];
}

interface ThoughtChainPanelProps {
  chain: ChainSummary | null;
  activeIndex: number;
  isRunning: boolean;
  onConfirmAction?: (actionIndex: number) => void;
  collapsed?: boolean;
}

const stateColors: Record<string, string> = {
  Thinking: "blue",
  Planning: "purple",
  Acting: "orange",
  Observing: "cyan",
  Finished: "green",
  Failed: "red",
};

const stateIcons: Record<string, React.ReactNode> = {
  Thinking: <Brain size={14} />,
  Planning: <Lightbulb size={14} />,
  Acting: <Loader2 size={14} className="animate-spin" />,
  Observing: <AlertTriangle size={14} />,
  Finished: <CheckCircle size={14} />,
  Failed: <XCircle size={14} />,
};

function formatTimestamp(ts: string): string {
  try {
    const date = new Date(ts);
    return date.toLocaleTimeString();
  } catch {
    return ts;
  }
}

function ThoughtStepCard({
  step,
  isActive,
  onConfirm,
}: {
  step: ThoughtStep;
  isActive: boolean;
  onConfirm?: () => void;
}) {
  const actionType = step.action?.action_type || "none";
  const toolName = step.action?.tool_name;

  return (
    <Card
      size="small"
      className={`thought-step-card ${isActive ? "active" : ""}`}
      style={{
        borderColor: isActive ? "#1890ff" : undefined,
        backgroundColor: isActive ? "#f0f7ff" : undefined,
      }}
    >
      <div className="flex items-start gap-2 mb-2">
        <Tag color={stateColors[step.state] || "default"} icon={stateIcons[step.state]}>
          {step.state}
        </Tag>
        <Text type="secondary" className="text-xs">
          {formatTimestamp(step.timestamp)}
        </Text>
        {step.is_verified ? (
          <Tooltip title="已验证">
            <CheckCircle size={14} className="text-green-500" />
          </Tooltip>
        ) : step.result ? (
          <Tooltip title="未验证">
            <AlertTriangle size={14} className="text-yellow-500" />
          </Tooltip>
        ) : null}
      </div>

      <div className="mb-2">
        <Text strong className="text-sm">
          推理:
        </Text>
        <p className="text-sm mt-1 mb-0 whitespace-pre-wrap">{step.reasoning}</p>
      </div>

      {step.action && (
        <div className="mb-2 p-2 bg-gray-50 rounded">
          <Text strong className="text-xs text-gray-500">
            动作: {actionType}
          </Text>
          {toolName && (
            <div className="mt-1">
              <Tag>{toolName}</Tag>
            </div>
          )}
          {step.action.requires_confirmation && onConfirm && (
            <Button
              type="primary"
              size="small"
              className="mt-2"
              onClick={onConfirm}
            >
              确认执行
            </Button>
          )}
        </div>
      )}

      {step.observation && (
        <div className="mb-2">
          <Text strong className="text-sm">
            观察:
          </Text>
          <p className="text-sm mt-1 mb-0 text-gray-600 whitespace-pre-wrap">
            {step.observation}
          </p>
        </div>
      )}

      {step.result && (
        <div className="p-2 bg-gray-50 rounded">
          <Text strong className="text-xs text-gray-500">
            结果:
          </Text>
          <p className="text-sm mt-1 mb-0 whitespace-pre-wrap">{step.result}</p>
        </div>
      )}
    </Card>
  );
}

export function ThoughtChainPanel({
  chain,
  activeIndex,
  isRunning,
  onConfirmAction,
  collapsed = false,
}: ThoughtChainPanelProps) {
  const [expandedKeys, setExpandedKeys] = useState<number[]>([]);

  useEffect(() => {
    if (activeIndex >= 0) {
      setExpandedKeys((prev) => {
        if (!prev.includes(activeIndex)) {
          return [...prev, activeIndex];
        }
        return prev;
      });
    }
  }, [activeIndex]);

  const handleCollapseChange = useCallback(
    (keys: number[]) => {
      setExpandedKeys(keys);
    },
    []
  );

  if (!chain) {
    return (
      <Card size="small" className="thought-chain-panel">
        <div className="flex items-center justify-center h-32 text-gray-400">
          <Brain size={24} className="mr-2" />
          <Text type="secondary">暂无推理过程</Text>
        </div>
      </Card>
    );
  }

  const items = chain.steps.map((step, index) => ({
    key: index,
    label: (
      <div className="flex items-center gap-2">
        <Tag color={stateColors[step.state] || "default"} icon={stateIcons[step.state]}>
          {step.state}
        </Tag>
        <Text className="text-sm">
          {step.action?.tool_name || step.action?.action_type || "推理"}
        </Text>
        {step.is_verified ? (
          <CheckCircle size={12} className="text-green-500" />
        ) : step.result ? (
          <AlertTriangle size={12} className="text-yellow-500" />
        ) : null}
        {index === activeIndex && isRunning && (
          <Loader2 size={12} className="animate-spin text-blue-500" />
        )}
      </div>
    ),
    children: (
      <ThoughtStepCard
        step={step}
        isActive={index === activeIndex}
        onConfirm={
          step.action?.requires_confirmation && onConfirmAction
            ? () => onConfirmAction(index)
            : undefined
        }
      />
    ),
  }));

  return (
    <Card
      size="small"
      className="thought-chain-panel"
      title={
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Brain size={16} />
            <span>推理过程</span>
            <Tag>{chain.iterations} 轮</Tag>
            <Tag>{chain.total_steps} 步</Tag>
          </div>
          {isRunning && (
            <Tag color="blue" icon={<Loader2 size={12} className="animate-spin" />}>
              运行中
            </Tag>
          )}
        </div>
      }
    >
      {collapsed ? (
        <div className="flex items-center gap-1 overflow-x-auto py-2">
          {chain.steps.map((step, index) => (
            <Tooltip
              key={step.id}
              title={
                <div>
                  <div>{step.state}</div>
                  <div className="text-xs">{step.reasoning.slice(0, 50)}...</div>
                </div>
              }
            >
              <div
                className={`w-3 h-3 rounded-full shrink-0 ${
                  index === activeIndex && isRunning
                    ? "bg-blue-500 animate-pulse"
                    : step.is_verified
                      ? "bg-green-500"
                      : step.result
                        ? "bg-yellow-500"
                        : "bg-gray-300"
                }`}
              />
            </Tooltip>
          ))}
        </div>
      ) : (
        <Collapse
          activeKey={expandedKeys}
          onChange={(keys) => {
            // Ant Design Collapse onChange returns string[] or number[] depending on key type
            // Since our keys are numbers (index), we need to ensure type consistency
            const numericKeys = Array.isArray(keys) 
              ? keys.map((k) => Number(k))
              : [Number(keys)];
            handleCollapseChange(numericKeys);
          }}
          items={items}
          bordered={false}
          expandIcon={({ isActive }) => (
            <ChevronRight
              size={14}
              style={{ transform: isActive ? "rotate(90deg)" : undefined }}
            />
          )}
        />
      )}
    </Card>
  );
}

export function useThoughtChain() {
  const [chain, setChain] = useState<ChainSummary | null>(null);
  const [activeIndex, setActiveIndex] = useState(-1);
  const [isRunning, setIsRunning] = useState(false);

  const addStep = useCallback((step: ThoughtStep) => {
    setChain((prev) => {
      if (!prev) {
        return {
          total_steps: 1,
          iterations: 1,
          current_state: step.state,
          steps: [step],
        };
      }
      return {
        ...prev,
        total_steps: prev.total_steps + 1,
        current_state: step.state,
        steps: [...prev.steps, step],
      };
    });
    setActiveIndex((prev) => prev + 1);
  }, []);

  const updateStep = useCallback(
    (index: number, updates: Partial<ThoughtStep>) => {
      setChain((prev) => {
        if (!prev) return null;
        const newSteps = [...prev.steps];
        if (newSteps[index]) {
          newSteps[index] = { ...newSteps[index], ...updates };
        }
        return { ...prev, steps: newSteps };
      });
    },
    []
  );

  const reset = useCallback(() => {
    setChain(null);
    setActiveIndex(-1);
    setIsRunning(false);
  }, []);

  return {
    chain,
    activeIndex,
    isRunning,
    setIsRunning,
    addStep,
    updateStep,
    reset,
  };
}
