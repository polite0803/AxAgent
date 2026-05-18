import { useSettingsStore } from "@/stores";
import { Divider, InputNumber, Select, Slider, Switch } from "antd";
import { useTranslation } from "react-i18next";
import { CacheConfigPanel } from "./CacheConfigPanel";
import { SettingsGroup } from "./SettingsGroup";

/** 类型安全地获取/设置扩展配置项（尚未加入 AppSettings 类型） */
function useExtSetting<T>(key: string, defaultVal: T): [T, (v: T) => void] {
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const val = (settings as unknown as Record<string, unknown>)[key] as
    | T
    | undefined;
  return [
    (val ?? defaultVal) as T,
    (v: T) => saveSettings({ [key]: v } as unknown as Partial<typeof settings>),
  ];
}

// ---------------------------------------------------------------------------
// Bash 验证设置
// ---------------------------------------------------------------------------

function BashValidationSection() {
  const { t } = useTranslation();
  const [dangerous, setDangerous] = useExtSetting(
    "bash_validate_dangerous",
    true,
  );
  const [network, setNetwork] = useExtSetting("bash_validate_network", true);
  const [timeout, setTimeout_] = useExtSetting("bash_timeout_secs", 120);

  return (
    <SettingsGroup title={t("advancedSettings.bashSecurity")}>
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advanced.dangerousCmdDetect")}</span>
        <Switch
          id="advanced-settings-switch-4"
          checked={dangerous}
          onChange={setDangerous}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.networkCmdDetect")}</span>
        <Switch
          id="advanced-settings-switch-5"
          checked={network}
          onChange={setNetwork}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advanced.cmdTimeout")}</span>
        <InputNumber
          id="advanced-settings-inputnumber-6"
          min={5}
          max={600}
          value={timeout}
          onChange={(v) => v && setTimeout_(v)}
          style={{ width: 80 }}
        />
      </div>
    </SettingsGroup>
  );
}

// ---------------------------------------------------------------------------
// 权限执行器设置
// ---------------------------------------------------------------------------

function PermissionEnforcerSection() {
  const { t } = useTranslation();
  const [permMode, setPermMode] = useExtSetting("permission_mode", "default");
  const [writeConfirm, setWriteConfirm] = useExtSetting(
    "permission_write_confirm",
    true,
  );
  const [netConfirm, setNetConfirm] = useExtSetting(
    "permission_network_confirm",
    true,
  );
  const [shellConfirm, setShellConfirm] = useExtSetting(
    "permission_shell_confirm",
    true,
  );

  return (
    <SettingsGroup title={t("advancedSettings.permissionStrategy")}>
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.defaultPermission")}</span>
        <Select
          id="advanced-settings-select-7"
          value={permMode}
          options={[
            { value: "default", label: t("advancedSettings.perm.default") },
            {
              value: "accept_edits",
              label: t("advancedSettings.perm.acceptEdits"),
            },
            {
              value: "full_access",
              label: t("advancedSettings.perm.fullAccess"),
            },
          ]}
          onChange={setPermMode}
          style={{ width: 150 }}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advanced.fileWriteConfirm")}</span>
        <Switch
          id="advanced-settings-switch-8"
          checked={writeConfirm}
          onChange={setWriteConfirm}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.networkConfirm")}</span>
        <Switch
          id="advanced-settings-switch-9"
          checked={netConfirm}
          onChange={setNetConfirm}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.shellConfirm")}</span>
        <Switch
          id="advanced-settings-switch-10"
          checked={shellConfirm}
          onChange={setShellConfirm}
        />
      </div>
    </SettingsGroup>
  );
}

// ---------------------------------------------------------------------------
// 模式选择设置
// ---------------------------------------------------------------------------

function ModeSelectorSection() {
  const { t } = useTranslation();
  const [agentMode, setAgentMode] = useExtSetting("agent_mode", "general");
  const [budget, setBudget] = useExtSetting("token_budget_limit", 180000);
  const [budgetEnabled, setBudgetEnabled] = useExtSetting(
    "token_budget_enabled",
    true,
  );

  return (
    <SettingsGroup title={t("advancedSettings.agentMode")}>
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.defaultMode")}</span>
        <Select
          id="advanced-settings-select-11"
          value={agentMode}
          options={[
            { value: "general", label: t("advancedSettings.mode.general") },
            { value: "speed", label: t("advancedSettings.mode.speed") },
            { value: "deep", label: t("advancedSettings.mode.deep") },
            { value: "plan", label: t("advancedSettings.mode.plan") },
          ]}
          onChange={setAgentMode}
          style={{ width: 150 }}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.tokenBudgetLimit")}</span>
        <InputNumber
          id="advanced-settings-inputnumber-12"
          min={10000}
          max={500000}
          step={10000}
          value={budget}
          onChange={(v) => v && setBudget(v)}
          style={{ width: 100 }}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.enableTokenBudget")}</span>
        <Switch
          id="advanced-settings-switch-13"
          checked={budgetEnabled}
          onChange={setBudgetEnabled}
        />
      </div>
    </SettingsGroup>
  );
}

// ---------------------------------------------------------------------------
// 故障恢复设置
// ---------------------------------------------------------------------------

function RecoveryRecipesSection() {
  const { t } = useTranslation();
  const [autoRetry, setAutoRetry] = useExtSetting("recovery_auto_retry", true);
  const [maxRetries, setMaxRetries] = useExtSetting("recovery_max_retries", 3);
  const [delay, setDelay] = useExtSetting("recovery_retry_delay_secs", 5);
  const [fallback, setFallback] = useExtSetting(
    "recovery_model_fallback",
    true,
  );

  return (
    <SettingsGroup title={t("advancedSettings.faultRecovery")}>
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.autoRetry")}</span>
        <Switch
          id="advanced-settings-switch-14"
          checked={autoRetry}
          onChange={setAutoRetry}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.maxRetries")}</span>
        <InputNumber
          id="advanced-settings-inputnumber-15"
          min={1}
          max={10}
          value={maxRetries}
          onChange={(v) => v && setMaxRetries(v)}
          style={{ width: 80 }}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.retryDelay")}</span>
        <InputNumber
          id="advanced-settings-inputnumber-16"
          min={1}
          max={60}
          value={delay}
          onChange={(v) => v && setDelay(v)}
          style={{ width: 80 }}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.modelFallback")}</span>
        <Switch
          id="advanced-settings-switch-17"
          checked={fallback}
          onChange={setFallback}
        />
      </div>
    </SettingsGroup>
  );
}

function GreenContractSection() {
  const { t } = useTranslation();
  const [cpuLimit, setCpuLimit] = useExtSetting("resource_cpu_limit", 80);
  const [memLimit, setMemLimit] = useExtSetting(
    "resource_memory_limit_mb",
    4096,
  );
  const [idleDetect, setIdleDetect] = useExtSetting(
    "resource_idle_detect",
    true,
  );
  const [idleTimeout, setIdleTimeout] = useExtSetting(
    "resource_idle_timeout_secs",
    300,
  );

  return (
    <SettingsGroup title={t("advancedSettings.resourceControl")}>
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.cpuLimit")}</span>
        <Slider
          min={10}
          max={90}
          value={cpuLimit}
          onChange={setCpuLimit}
          style={{ width: 150 }}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.memoryLimit")}</span>
        <InputNumber
          id="advanced-settings-inputnumber-18"
          min={256}
          max={32768}
          step={256}
          value={memLimit}
          onChange={(v) => v && setMemLimit(v)}
          style={{ width: 100 }}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.enableIdleDetection")}</span>
        <Switch
          id="advanced-settings-switch-19"
          checked={idleDetect}
          onChange={setIdleDetect}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.idleTimeout")}</span>
        <InputNumber
          id="advanced-settings-inputnumber-20"
          min={30}
          max={3600}
          value={idleTimeout}
          onChange={(v) => v && setIdleTimeout(v)}
          style={{ width: 80 }}
        />
      </div>
    </SettingsGroup>
  );
}

function CompactionThresholdSection() {
  const { t } = useTranslation();
  const [autoThresh, setAutoThresh] = useExtSetting(
    "compact_auto_threshold",
    13000,
  );
  const [warnBuffer, setWarnBuffer] = useExtSetting(
    "compact_warning_buffer",
    20000,
  );
  const [maxFails, setMaxFails] = useExtSetting("compact_max_failures", 3);
  const [memCompact, setMemCompact] = useExtSetting(
    "session_memory_compact_enabled",
    true,
  );

  return (
    <SettingsGroup title={t("advancedSettings.contextCompression")}>
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.autoCompressThreshold")}</span>
        <InputNumber
          id="advanced-settings-inputnumber-21"
          min={10000}
          max={200000}
          step={5000}
          value={autoThresh}
          onChange={(v) => v && setAutoThresh(v)}
          style={{ width: 100 }}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.warningBuffer")}</span>
        <InputNumber
          id="advanced-settings-inputnumber-22"
          min={5000}
          max={100000}
          step={5000}
          value={warnBuffer}
          onChange={(v) => v && setWarnBuffer(v)}
          style={{ width: 100 }}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.maxConsecutiveFailures")}</span>
        <InputNumber
          id="advanced-settings-inputnumber-23"
          min={1}
          max={10}
          value={maxFails}
          onChange={(v) => v && setMaxFails(v)}
          style={{ width: 80 }}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.enableMemoryCompression")}</span>
        <Switch
          id="advanced-settings-switch-24"
          checked={memCompact}
          onChange={setMemCompact}
        />
      </div>
    </SettingsGroup>
  );
}

function DreamConsolidationSection() {
  const { t } = useTranslation();
  const [enabled, setEnabled] = useExtSetting("dream_enabled", true);
  const [intervalH, setIntervalH] = useExtSetting(
    "dream_min_interval_hours",
    1,
  );
  const [minSessions, setMinSessions] = useExtSetting("dream_min_sessions", 3);
  const [maxDuration, setMaxDuration] = useExtSetting(
    "dream_max_duration_secs",
    120,
  );

  return (
    <SettingsGroup title={t("advancedSettings.dreamConsolidation")}>
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.enableDream")}</span>
        <Switch
          id="advanced-settings-switch-25"
          checked={enabled}
          onChange={setEnabled}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.minInterval")}</span>
        <InputNumber
          id="advanced-settings-inputnumber-26"
          min={1}
          max={24}
          value={intervalH}
          onChange={(v) => v && setIntervalH(v)}
          style={{ width: 80 }}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.minNewSessions")}</span>
        <InputNumber
          id="advanced-settings-inputnumber-27"
          min={1}
          max={20}
          value={minSessions}
          onChange={(v) => v && setMinSessions(v)}
          style={{ width: 80 }}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.maxDuration")}</span>
        <InputNumber
          id="advanced-settings-inputnumber-28"
          min={30}
          max={600}
          value={maxDuration}
          onChange={(v) => v && setMaxDuration(v)}
          style={{ width: 80 }}
        />
      </div>
    </SettingsGroup>
  );
}

function LspDiagnosticsSection() {
  const { t } = useTranslation();
  const [enabled, setEnabled] = useExtSetting("lsp_enabled", false);
  const [level, setLevel] = useExtSetting("lsp_diagnostic_level", "warning");

  return (
    <SettingsGroup title={t("advancedSettings.lspServer")}>
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.enableLspDiagnostics")}</span>
        <Switch
          id="advanced-settings-switch-29"
          checked={enabled}
          onChange={setEnabled}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
      >
        <span>{t("advancedSettings.diagnosticLevelLabel")}</span>
        <Select
          id="advanced-settings-select-30"
          value={level}
          options={[
            {
              value: "error",
              label: t("advancedSettings.diagnosticLevel.error"),
            },
            {
              value: "warning",
              label: t("advancedSettings.diagnosticLevel.warning"),
            },
            {
              value: "information",
              label: t("advancedSettings.diagnosticLevel.information"),
            },
          ]}
          onChange={setLevel}
          style={{ width: 130 }}
        />
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
      <CacheBreakpointSection />
    </div>
  );
}

/** Cache 断点设置区段 — 嵌入 CacheConfigPanel */
function CacheBreakpointSection() {
  const [cacheBreakpoints, setCacheBreakpoints] = useExtSetting(
    "enable_cache_breakpoints",
    false,
  );
  return (
    <CacheConfigPanel
      enableCacheBreakpoints={cacheBreakpoints}
      onToggleCacheBreakpoints={setCacheBreakpoints}
    />
  );
}
