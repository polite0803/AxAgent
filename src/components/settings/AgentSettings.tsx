import {
  AlertCircle,
  Brain,
  Cog,
  FlaskConical,
  RefreshCw,
  Save,
  Settings,
  Undo2,
} from "lucide-react";
import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Card,
  Divider,
  InputNumber,
  Select,
  Slider,
  Switch,
  Tag,
  Tooltip,
  Typography,
} from "antd";

const { Text, Title } = Typography;

interface ReActSettings {
  maxIterations: number;
  maxDepth: number;
  verificationEnabled: boolean;
  streamThoughts: boolean;
}

interface TaskDecompositionSettings {
  threshold: number;
  parallelExecution: boolean;
  maxSubtasks: number;
}

interface ErrorRecoverySettings {
  enabled: boolean;
  maxAttempts: number;
  baseDelayMs: number;
  exponentialBackoff: boolean;
}

interface ReflectionSettings {
  enabled: boolean;
  storeInsights: boolean;
  minQualityThreshold: number;
}

interface AgentSettingsProps {
  initialConfig?: Partial<FullAgentConfig>;
  onSave?: (config: FullAgentConfig) => void;
  onReset?: () => void;
}

interface FullAgentConfig {
  react: ReActSettings;
  taskDecomposition: TaskDecompositionSettings;
  errorRecovery: ErrorRecoverySettings;
  reflection: ReflectionSettings;
  debugMode: "off" | "basic" | "verbose";
}

const defaultConfig: FullAgentConfig = {
  react: {
    maxIterations: 50,
    maxDepth: 10,
    verificationEnabled: true,
    streamThoughts: true,
  },
  taskDecomposition: {
    threshold: 3,
    parallelExecution: true,
    maxSubtasks: 20,
  },
  errorRecovery: {
    enabled: true,
    maxAttempts: 3,
    baseDelayMs: 1000,
    exponentialBackoff: true,
  },
  reflection: {
    enabled: true,
    storeInsights: true,
    minQualityThreshold: 5,
  },
  debugMode: "off",
};

function SettingsSection({
  title,
  icon,
  children,
}: {
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <Card size="small" className="mb-4">
      <div className="flex items-center gap-2 mb-4">
        {icon}
        <Title level={5} style={{ margin: 0 }}>
          {title}
        </Title>
      </div>
      {children}
    </Card>
  );
}

function SettingRow({
  label,
  description,
  children,
}: {
  label: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between py-2">
      <div className="flex-1">
        <Text strong>{label}</Text>
        {description && (
          <Text type="secondary" className="text-xs block">
            {description}
          </Text>
        )}
      </div>
      <div className="ml-4">{children}</div>
    </div>
  );
}

function ReActSettingsPanel({
  config,
  onChange,
}: {
  config: ReActSettings;
  onChange: (c: ReActSettings) => void;
}) {
  return (
    <SettingsSection title="ReAct Engine" icon={<Brain size={16} />}>
      <SettingRow
        label="Max Iterations"
        description="Maximum reasoning iterations before stopping"
      >
        <InputNumber
          min={1}
          max={200}
          value={config.maxIterations}
          onChange={(v) => onChange({ ...config, maxIterations: v || 50 })}
          style={{ width: 100 }}
        />
      </SettingRow>

      <SettingRow
        label="Max Depth"
        description="Maximum task decomposition depth"
      >
        <InputNumber
          min={1}
          max={20}
          value={config.maxDepth}
          onChange={(v) => onChange({ ...config, maxDepth: v || 10 })}
          style={{ width: 100 }}
        />
      </SettingRow>

      <Divider className="my-2" />

      <SettingRow
        label="Result Verification"
        description="Verify results before proceeding"
      >
        <Switch
          checked={config.verificationEnabled}
          onChange={(v) => onChange({ ...config, verificationEnabled: v })}
        />
      </SettingRow>

      <SettingRow
        label="Stream Thoughts"
        description="Show reasoning chain in real-time"
      >
        <Switch
          checked={config.streamThoughts}
          onChange={(v) => onChange({ ...config, streamThoughts: v })}
        />
      </SettingRow>
    </SettingsSection>
  );
}

function TaskDecompositionSettingsPanel({
  config,
  onChange,
}: {
  config: TaskDecompositionSettings;
  onChange: (c: TaskDecompositionSettings) => void;
}) {
  return (
    <SettingsSection
      title="Task Decomposition"
      icon={<Settings size={16} />}
    >
      <SettingRow
        label="Decomposition Threshold"
        description="Number of subtasks to trigger decomposition"
      >
        <InputNumber
          min={1}
          max={10}
          value={config.threshold}
          onChange={(v) => onChange({ ...config, threshold: v || 3 })}
          style={{ width: 80 }}
        />
      </SettingRow>

      <SettingRow
        label="Max Subtasks"
        description="Maximum number of subtasks per decomposition"
      >
        <InputNumber
          min={2}
          max={50}
          value={config.maxSubtasks}
          onChange={(v) => onChange({ ...config, maxSubtasks: v || 20 })}
          style={{ width: 80 }}
        />
      </SettingRow>

      <Divider className="my-2" />

      <SettingRow
        label="Parallel Execution"
        description="Execute independent subtasks in parallel"
      >
        <Switch
          checked={config.parallelExecution}
          onChange={(v) => onChange({ ...config, parallelExecution: v })}
        />
      </SettingRow>
    </SettingsSection>
  );
}

function ErrorRecoverySettingsPanel({
  config,
  onChange,
}: {
  config: ErrorRecoverySettings;
  onChange: (c: ErrorRecoverySettings) => void;
}) {
  return (
    <SettingsSection
      title="Error Recovery"
      icon={<RefreshCw size={16} />}
    >
      <SettingRow
        label="Enable Retry"
        description="Automatically retry failed operations"
      >
        <Switch
          checked={config.enabled}
          onChange={(v) => onChange({ ...config, enabled: v })}
        />
      </SettingRow>

      <SettingRow
        label="Max Attempts"
        description="Maximum retry attempts per operation"
      >
        <InputNumber
          min={1}
          max={10}
          value={config.maxAttempts}
          onChange={(v) => onChange({ ...config, maxAttempts: v || 3 })}
          disabled={!config.enabled}
          style={{ width: 80 }}
        />
      </SettingRow>

      <SettingRow
        label="Base Delay (ms)"
        description="Initial delay between retries"
      >
        <InputNumber
          min={100}
          max={30000}
          step={100}
          value={config.baseDelayMs}
          onChange={(v) => onChange({ ...config, baseDelayMs: v || 1000 })}
          disabled={!config.enabled}
          style={{ width: 100 }}
        />
      </SettingRow>

      <Divider className="my-2" />

      <SettingRow
        label="Exponential Backoff"
        description="Increase delay exponentially on each retry"
      >
        <Switch
          checked={config.exponentialBackoff}
          onChange={(v) => onChange({ ...config, exponentialBackoff: v })}
          disabled={!config.enabled}
        />
      </SettingRow>
    </SettingsSection>
  );
}

function ReflectionSettingsPanel({
  config,
  onChange,
}: {
  config: ReflectionSettings;
  onChange: (c: ReflectionSettings) => void;
}) {
  return (
    <SettingsSection
      title="Reflection & Self-Improvement"
      icon={<Cog size={16} />}
    >
      <SettingRow
        label="Enable Reflection"
        description="Analyze task execution after completion"
      >
        <Switch
          checked={config.enabled}
          onChange={(v) => onChange({ ...config, enabled: v })}
        />
      </SettingRow>

      <SettingRow
        label="Store Insights"
        description="Save learned patterns for future tasks"
      >
        <Switch
          checked={config.storeInsights}
          onChange={(v) => onChange({ ...config, storeInsights: v })}
          disabled={!config.enabled}
        />
      </SettingRow>

      <SettingRow
        label="Quality Threshold"
        description="Minimum quality score to save insights"
      >
        <Slider
          min={1}
          max={10}
          value={config.minQualityThreshold}
          onChange={(v) =>
            onChange({ ...config, minQualityThreshold: v })
          }
          disabled={!config.enabled}
          style={{ width: 150 }}
          marks={{
            1: "1",
            5: "5",
            10: "10",
          }}
        />
      </SettingRow>
    </SettingsSection>
  );
}

function DebugSettingsPanel({
  debugMode,
  onChange,
}: {
  debugMode: "off" | "basic" | "verbose";
  onChange: (m: "off" | "basic" | "verbose") => void;
}) {
  return (
    <SettingsSection title="Debug Mode" icon={<FlaskConical size={16} />}>
      <SettingRow
        label="Debug Level"
        description="Control verbosity of debug output"
      >
        <Select
          value={debugMode}
          onChange={onChange}
          style={{ width: 120 }}
          options={[
            { value: "off", label: "Off" },
            { value: "basic", label: "Basic" },
            { value: "verbose", label: "Verbose" },
          ]}
        />
      </SettingRow>

      {debugMode !== "off" && (
        <Alert
          type="info"
          message={
            debugMode === "verbose"
              ? "Verbose mode: Full thought chain, all tool inputs/outputs, internal state"
              : "Basic mode: Thought chain steps, key decisions, errors"
          }
          className="mt-2"
        />
      )}
    </SettingsSection>
  );
}

export function AgentSettings({
  initialConfig,
  onSave,
  onReset,
}: AgentSettingsProps) {
  const [config, setConfig] = useState<FullAgentConfig>({
    ...defaultConfig,
    ...initialConfig,
  });
  const [hasChanges, setHasChanges] = useState(false);
  const [savedMessage, setSavedMessage] = useState(false);

  useEffect(() => {
    if (initialConfig) {
      setConfig({ ...defaultConfig, ...initialConfig });
    }
  }, [initialConfig]);

  const handleChange = (newConfig: Partial<FullAgentConfig>) => {
    setConfig((prev) => ({ ...prev, ...newConfig }));
    setHasChanges(true);
  };

  const handleSave = () => {
    onSave?.(config);
    setHasChanges(false);
    setSavedMessage(true);
    setTimeout(() => setSavedMessage(false), 2000);
  };

  const handleReset = () => {
    setConfig(defaultConfig);
    setHasChanges(true);
    onReset?.();
  };

  return (
    <div className="agent-settings p-4">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Brain size={20} />
          <Title level={4} style={{ margin: 0 }}>
            Agent Configuration
          </Title>
          {hasChanges && (
            <Tag color="orange" icon={<AlertCircle size={12} />}>
              Unsaved
            </Tag>
          )}
        </div>
        <div className="flex items-center gap-2">
          <Tooltip title="Reset to defaults">
            <Button
              type="text"
              icon={<Undo2 size={16} />}
              onClick={handleReset}
            >
              Reset
            </Button>
          </Tooltip>
          <Button
            type="primary"
            icon={<Save size={16} />}
            onClick={handleSave}
            disabled={!hasChanges}
          >
            {savedMessage ? "Saved!" : "Save"}
          </Button>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <div>
          <ReActSettingsPanel
            config={config.react}
            onChange={(react) => handleChange({ react })}
          />

          <TaskDecompositionSettingsPanel
            config={config.taskDecomposition}
            onChange={(taskDecomposition) => handleChange({ taskDecomposition })}
          />
        </div>

        <div>
          <ErrorRecoverySettingsPanel
            config={config.errorRecovery}
            onChange={(errorRecovery) => handleChange({ errorRecovery })}
          />

          <ReflectionSettingsPanel
            config={config.reflection}
            onChange={(reflection) => handleChange({ reflection })}
          />

          <DebugSettingsPanel
            debugMode={config.debugMode}
            onChange={(debugMode) => handleChange({ debugMode })}
          />
        </div>
      </div>

      <Divider />

      <Card size="small" className="bg-gray-50">
        <Text type="secondary" className="text-xs">
          <strong>Tip:</strong> Higher max iterations allow more complex reasoning
          but may increase response time. Enable parallel execution to speed up
          multi-subtask operations. Use debug mode to troubleshoot issues.
        </Text>
      </Card>
    </div>
  );
}

export interface AgentConfigState {
  config: FullAgentConfig;
  hasChanges: boolean;
  isDirty: boolean;
}

export function useAgentConfig(initialConfig?: Partial<FullAgentConfig>) {
  const [config, setConfig] = useState<FullAgentConfig>({
    ...defaultConfig,
    ...initialConfig,
  });
  const [isDirty, setIsDirty] = useState(false);
  const [history, setHistory] = useState<FullAgentConfig[]>([]);

  const updateConfig = (updates: Partial<FullAgentConfig>) => {
    setConfig((prev) => ({ ...prev, ...updates }));
    setIsDirty(true);
  };

  const saveConfig = () => {
    setHistory((prev) => [...prev, config]);
    setIsDirty(false);
  };

  const resetConfig = () => {
    setConfig(defaultConfig);
    setIsDirty(true);
  };

  const restoreFromHistory = (index: number) => {
    if (index >= 0 && index < history.length) {
      setConfig(history[index]);
      setIsDirty(true);
    }
  };

  return {
    config,
    isDirty,
    history,
    updateConfig,
    saveConfig,
    resetConfig,
    restoreFromHistory,
    AgentSettings,
  };
}
