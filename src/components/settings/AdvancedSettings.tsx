import { useSettingsStore } from "@/stores";
import {
  Divider,
  InputNumber,
  Select,
  Slider,
  Switch,
} from "antd";
import { SettingsGroup } from "./SettingsGroup";

/** 类型安全地获取/设置扩展配置项（尚未加入 AppSettings 类型） */
function useExtSetting<T>(key: string, defaultVal: T): [T, (v: T) => void] {
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const val = (settings as unknown as Record<string, unknown>)[key] as T | undefined;
  return [
    (val ?? defaultVal) as T,
    (v: T) => saveSettings({ [key]: v } as unknown as Partial<typeof settings>),
  ];
}

// ---------------------------------------------------------------------------
// Bash 验证设置
// ---------------------------------------------------------------------------

function BashValidationSection() {
  const [dangerous, setDangerous] = useExtSetting("bash_validate_dangerous", true);
  const [network, setNetwork] = useExtSetting("bash_validate_network", true);
  const [timeout, setTimeout_] = useExtSetting("bash_timeout_secs", 120);

  return (
    <SettingsGroup title="Bash 命令安全验证">
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>启用危险命令检测</span>
        <Switch checked={dangerous} onChange={setDangerous} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>启用网络命令检测</span>
        <Switch checked={network} onChange={setNetwork} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>命令超时（秒）</span>
        <InputNumber min={5} max={600} value={timeout} onChange={(v) => v && setTimeout_(v)} style={{ width: 80 }} />
      </div>
    </SettingsGroup>
  );
}

// ---------------------------------------------------------------------------
// 权限执行器设置
// ---------------------------------------------------------------------------

function PermissionEnforcerSection() {
  const [permMode, setPermMode] = useExtSetting("permission_mode", "default");
  const [writeConfirm, setWriteConfirm] = useExtSetting("permission_write_confirm", true);
  const [netConfirm, setNetConfirm] = useExtSetting("permission_network_confirm", true);
  const [shellConfirm, setShellConfirm] = useExtSetting("permission_shell_confirm", true);

  return (
    <SettingsGroup title="权限执行策略">
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>默认权限模式</span>
        <Select value={permMode} options={[
          { value: "default", label: "默认（每次询问）" },
          { value: "accept_edits", label: "接受编辑" },
          { value: "full_access", label: "完全访问" },
        ]} onChange={setPermMode} style={{ width: 150 }} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>文件写入需确认</span><Switch checked={writeConfirm} onChange={setWriteConfirm} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>网络请求需确认</span><Switch checked={netConfirm} onChange={setNetConfirm} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>Shell 执行需确认</span><Switch checked={shellConfirm} onChange={setShellConfirm} />
      </div>
    </SettingsGroup>
  );
}

// ---------------------------------------------------------------------------
// 模式选择设置
// ---------------------------------------------------------------------------

function ModeSelectorSection() {
  const [agentMode, setAgentMode] = useExtSetting("agent_mode", "general");
  const [budget, setBudget] = useExtSetting("token_budget_limit", 180000);
  const [budgetEnabled, setBudgetEnabled] = useExtSetting("token_budget_enabled", true);

  return (
    <SettingsGroup title="Agent 运行模式">
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>默认模式</span>
        <Select value={agentMode} options={[
          { value: "general", label: "通用模式" },
          { value: "speed", label: "快速模式（轻量）" },
          { value: "deep", label: "深度模式（研究）" },
          { value: "plan", label: "计划模式" },
        ]} onChange={setAgentMode} style={{ width: 150 }} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>Token 预算上限</span>
        <InputNumber min={10000} max={500000} step={10000} value={budget} onChange={(v) => v && setBudget(v)} style={{ width: 100 }} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>启用 Token 预算检测</span>
        <Switch checked={budgetEnabled} onChange={setBudgetEnabled} />
      </div>
    </SettingsGroup>
  );
}

// ---------------------------------------------------------------------------
// 故障恢复设置
// ---------------------------------------------------------------------------

function RecoveryRecipesSection() {
  const [autoRetry, setAutoRetry] = useExtSetting("recovery_auto_retry", true);
  const [maxRetries, setMaxRetries] = useExtSetting("recovery_max_retries", 3);
  const [delay, setDelay] = useExtSetting("recovery_retry_delay_secs", 5);
  const [fallback, setFallback] = useExtSetting("recovery_model_fallback", true);

  return (
    <SettingsGroup title="故障恢复策略">
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>自动重试</span><Switch checked={autoRetry} onChange={setAutoRetry} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>最大重试次数</span>
        <InputNumber min={1} max={10} value={maxRetries} onChange={(v) => v && setMaxRetries(v)} style={{ width: 80 }} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>重试延迟（秒）</span>
        <InputNumber min={1} max={60} value={delay} onChange={(v) => v && setDelay(v)} style={{ width: 80 }} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>模型降级回退</span><Switch checked={fallback} onChange={setFallback} />
      </div>
    </SettingsGroup>
  );
}

function GreenContractSection() {
  const [cpuLimit, setCpuLimit] = useExtSetting("resource_cpu_limit", 80);
  const [memLimit, setMemLimit] = useExtSetting("resource_memory_limit_mb", 4096);
  const [idleDetect, setIdleDetect] = useExtSetting("resource_idle_detect", true);
  const [idleTimeout, setIdleTimeout] = useExtSetting("resource_idle_timeout_secs", 300);

  return (
    <SettingsGroup title="资源管控 & 环保策略">
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>CPU 使用率上限（%）</span>
        <Slider min={10} max={90} value={cpuLimit} onChange={setCpuLimit} style={{ width: 150 }} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>内存使用上限（MB）</span>
        <InputNumber min={256} max={32768} step={256} value={memLimit} onChange={(v) => v && setMemLimit(v)} style={{ width: 100 }} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>启用空转检测</span><Switch checked={idleDetect} onChange={setIdleDetect} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>空闲超时（秒）</span>
        <InputNumber min={30} max={3600} value={idleTimeout} onChange={(v) => v && setIdleTimeout(v)} style={{ width: 80 }} />
      </div>
    </SettingsGroup>
  );
}

function CompactionThresholdSection() {
  const [autoThresh, setAutoThresh] = useExtSetting("compact_auto_threshold", 13000);
  const [warnBuffer, setWarnBuffer] = useExtSetting("compact_warning_buffer", 20000);
  const [maxFails, setMaxFails] = useExtSetting("compact_max_failures", 3);
  const [memCompact, setMemCompact] = useExtSetting("session_memory_compact_enabled", true);

  return (
    <SettingsGroup title="上下文压缩阈值">
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>自动压缩阈值（token）</span>
        <InputNumber min={10000} max={200000} step={5000} value={autoThresh} onChange={(v) => v && setAutoThresh(v)} style={{ width: 100 }} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>警告阈值缓冲（token）</span>
        <InputNumber min={5000} max={100000} step={5000} value={warnBuffer} onChange={(v) => v && setWarnBuffer(v)} style={{ width: 100 }} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>最大连续压缩失败</span>
        <InputNumber min={1} max={10} value={maxFails} onChange={(v) => v && setMaxFails(v)} style={{ width: 80 }} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>启用会话记忆压缩</span><Switch checked={memCompact} onChange={setMemCompact} />
      </div>
    </SettingsGroup>
  );
}

function DreamConsolidationSection() {
  const [enabled, setEnabled] = useExtSetting("dream_enabled", true);
  const [intervalH, setIntervalH] = useExtSetting("dream_min_interval_hours", 1);
  const [minSessions, setMinSessions] = useExtSetting("dream_min_sessions", 3);
  const [maxDuration, setMaxDuration] = useExtSetting("dream_max_duration_secs", 120);

  return (
    <SettingsGroup title="Dream 后台巩固">
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>启用 Dream 巩固</span><Switch checked={enabled} onChange={setEnabled} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>最小间隔（小时）</span>
        <InputNumber min={1} max={24} value={intervalH} onChange={(v) => v && setIntervalH(v)} style={{ width: 80 }} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>最小新会话数</span>
        <InputNumber min={1} max={20} value={minSessions} onChange={(v) => v && setMinSessions(v)} style={{ width: 80 }} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>最大持续时间（秒）</span>
        <InputNumber min={30} max={600} value={maxDuration} onChange={(v) => v && setMaxDuration(v)} style={{ width: 80 }} />
      </div>
    </SettingsGroup>
  );
}

function LspDiagnosticsSection() {
  const [enabled, setEnabled] = useExtSetting("lsp_enabled", false);
  const [level, setLevel] = useExtSetting("lsp_diagnostic_level", "warning");

  return (
    <SettingsGroup title="LSP 语言服务器">
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>启用 LSP 诊断</span><Switch checked={enabled} onChange={setEnabled} />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div className="flex items-center justify-between" style={{ padding: "4px 0" }}>
        <span>诊断级别</span>
        <Select value={level} options={[
          { value: "error", label: "仅错误" },
          { value: "warning", label: "错误+警告" },
          { value: "information", label: "全部" },
        ]} onChange={setLevel} style={{ width: 130 }} />
      </div>
    </SettingsGroup>
  );
}

// ---------------------------------------------------------------------------
// 主面板
// ---------------------------------------------------------------------------

export function AdvancedSettings() {
  return (
    <div>
      <ModeSelectorSection />
      <CompactionThresholdSection />
      <BashValidationSection />
      <PermissionEnforcerSection />
      <RecoveryRecipesSection />
      <GreenContractSection />
      <DreamConsolidationSection />
      <LspDiagnosticsSection />
    </div>
  );
}
