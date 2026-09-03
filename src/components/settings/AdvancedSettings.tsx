// SPDX-License-Identifier: AGPL-3.0-only

import { useProviderStore, useSettingsStore } from "@/stores";
import type { SmartRouterTierMapping } from "@/types";
import { Divider, InputNumber, Radio, Select, Slider, Switch } from "antd";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { CacheConfigPanel } from "./CacheConfigPanel";
import { SettingsGroup } from "./SettingsGroup";

/** 类型安全地获取/设置扩展配置项（尚未加入 AppSettings 类型） */
function useExtSetting<T>(key: string, defaultVal: T): [T, (v: T) => void] {
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  // SAFE: dynamic key-based settings access for extended configuration not yet in AppSettings type
  const val = (settings as unknown as Record<string, unknown>)[key] as
    | T
    | undefined;
  return [
    (val ?? defaultVal) as T,
    (v: T) => saveSettings({ [key]: v } as unknown as Partial<typeof settings>), // SAFE: dynamic key update on settings
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
        data-search-key="advanced:dangerousCmdDetect"
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
        data-search-key="advanced:networkCmdDetect"
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
        data-search-key="advanced:cmdTimeout"
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
        data-search-key="advanced:defaultPermission"
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
        data-search-key="advanced:fileWriteConfirm"
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
        data-search-key="advanced:networkConfirm"
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
        data-search-key="advanced:shellConfirm"
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
        data-search-key="advanced:defaultMode"
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
        data-search-key="advanced:tokenBudgetLimit"
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
        data-search-key="advanced:enableTokenBudget"
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
        data-search-key="advanced:autoRetry"
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
        data-search-key="advanced:maxRetries"
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
        data-search-key="advanced:retryDelay"
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
        data-search-key="advanced:modelFallback"
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
        data-search-key="advanced:cpuLimit"
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
        data-search-key="advanced:memoryLimit"
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
        data-search-key="advanced:enableIdleDetection"
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
        data-search-key="advanced:idleTimeout"
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
        data-search-key="advanced:autoCompressThreshold"
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
        data-search-key="advanced:warningBuffer"
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
        data-search-key="advanced:maxConsecutiveFailures"
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
        data-search-key="advanced:enableMemoryCompression"
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
        data-search-key="advanced:enableDream"
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
        data-search-key="advanced:dreamMinInterval"
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
        data-search-key="advanced:dreamMinSessions"
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
        data-search-key="advanced:dreamMaxDuration"
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
        data-search-key="advanced:enableLspDiagnostics"
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
        data-search-key="advanced:diagnosticLevel"
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
// 2.7 P1:隐私控制 / 遥测级别三级开关
// ---------------------------------------------------------------------------

function PrivacyControlSection() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  // 后端默认 "off";旧版 settings 若无此字段也回退到 "off"。
  const level = settings.telemetryLevel ?? "off";

  return (
    <SettingsGroup title={t("advancedSettings.privacyControl")}>
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
        data-search-key="advanced:telemetryLevel"
      >
        <span>{t("advancedSettings.telemetryLevelLabel")}</span>
        <Radio.Group
          value={level}
          onChange={(e) => saveSettings({ telemetryLevel: e.target.value })}
          optionType="button"
          buttonStyle="solid"
          size="small"
        >
          <Radio.Button value="off">
            {t("advancedSettings.telemetryLevel.off")}
          </Radio.Button>
          <Radio.Button value="minimal">
            {t("advancedSettings.telemetryLevel.minimal")}
          </Radio.Button>
          <Radio.Button value="full">
            {t("advancedSettings.telemetryLevel.full")}
          </Radio.Button>
        </Radio.Group>
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="text-xs"
        style={{
          padding: "4px 0",
          color: "var(--ant-color-text-tertiary)",
          lineHeight: 1.6,
        }}
      >
        {t("advancedSettings.telemetryLevelHint")}
      </div>
    </SettingsGroup>
  );
}

// ---------------------------------------------------------------------------
// Smart Router 智能路由 — tier → provider/model 映射配置
// ---------------------------------------------------------------------------

function SmartRouterSection() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const providers = useProviderStore((s) => s.providers);
  const fetchProviders = useProviderStore((s) => s.fetchProviders);

  // 确保 provider 列表已加载（用户可能未访问过 provider 设置页）。
  useEffect(() => {
    if (providers.length === 0) {
      void fetchProviders();
    }
  }, [providers.length, fetchProviders]);

  const enabled = settings.smartRouterEnabled ?? false;
  const mappings = settings.smartRouterTierMappings ?? {};

  const updateTier = (tier: string, patch: Partial<SmartRouterTierMapping>) => {
    const next: Record<string, SmartRouterTierMapping> = { ...mappings };
    next[tier] = { ...next[tier], ...patch };
    saveSettings({ smartRouterTierMappings: next });
  };

  const tiers: Array<{ key: string; label: string }> = [
    { key: "budget", label: t("advancedSettings.smartRouter.tierBudget") },
    { key: "balanced", label: t("advancedSettings.smartRouter.tierBalanced") },
    { key: "premium", label: t("advancedSettings.smartRouter.tierPremium") },
  ];

  return (
    <SettingsGroup title={t("advancedSettings.smartRouter.title")}>
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
        data-search-key="advanced:smartRouterEnabled"
      >
        <span>{t("advancedSettings.smartRouter.enableLabel")}</span>
        <Switch
          checked={enabled}
          onChange={(v) => saveSettings({ smartRouterEnabled: v })}
        />
      </div>
      <div
        className="text-xs"
        style={{
          padding: "4px 0",
          color: "var(--ant-color-text-tertiary)",
          lineHeight: 1.6,
        }}
      >
        {t("advancedSettings.smartRouter.hint")}
      </div>
      {tiers.map((tier) => {
        const m = mappings[tier.key] ?? {};
        const selectedProvider = providers.find((p) => p.id === m.providerId);
        return (
          <div key={tier.key}>
            <Divider style={{ margin: "4px 0" }} />
            <div
              className="flex items-center justify-between gap-2"
              style={{ padding: "4px 0" }}
              data-search-key={`advanced:smartRouterTier:${tier.key}`}
            >
              <span style={{ minWidth: 72 }}>{tier.label}</span>
              <div
                className="flex gap-2"
                style={{ flex: 1, justifyContent: "flex-end" }}
              >
                <Select
                  disabled={!enabled}
                  value={m.providerId || undefined}
                  placeholder={t("advancedSettings.smartRouter.providerPlaceholder")}
                  style={{ width: 160 }}
                  showSearch
                  optionFilterProp="label"
                  allowClear
                  onChange={(v) => updateTier(tier.key, { providerId: v ?? "", modelId: "" })}
                  options={providers.map((p) => ({ value: p.id, label: p.name }))}
                />
                <Select
                  disabled={!enabled || !selectedProvider}
                  value={m.modelId || undefined}
                  placeholder={t("advancedSettings.smartRouter.modelPlaceholder")}
                  style={{ width: 180 }}
                  showSearch
                  optionFilterProp="label"
                  allowClear
                  onChange={(v) => updateTier(tier.key, { modelId: v ?? "" })}
                  options={(selectedProvider?.models ?? []).map((md) => ({
                    value: md.modelId,
                    label: md.name || md.modelId,
                  }))}
                />
              </div>
            </div>
          </div>
        );
      })}
    </SettingsGroup>
  );
}

// ---------------------------------------------------------------------------
// Agent 能力开关（后端真正消费的运行时能力，此前无 UI 入口）
// ---------------------------------------------------------------------------

function AgentBehaviorSection() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);

  return (
    <SettingsGroup title={t("advancedSettings.agentBehavior")}>
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
        data-search-key="advanced:thoughtChain"
      >
        <span>{t("advancedSettings.thoughtChain")}</span>
        <Switch
          checked={settings.thoughtChainEnabled ?? true}
          onChange={(v) => saveSettings({ thoughtChainEnabled: v })}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
        data-search-key="advanced:errorRecovery"
      >
        <span>{t("advancedSettings.errorRecovery")}</span>
        <Switch
          checked={settings.errorRecoveryEnabled ?? true}
          onChange={(v) => saveSettings({ errorRecoveryEnabled: v })}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
        data-search-key="advanced:screenPerception"
      >
        <span>{t("advancedSettings.screenPerception")}</span>
        <Switch
          checked={settings.screenPerceptionEnabled ?? false}
          onChange={(v) => saveSettings({ screenPerceptionEnabled: v })}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div style={{ padding: "4px 0" }} data-search-key="advanced:sandboxMode">
        <div className="flex items-center justify-between">
          <span>{t("advancedSettings.sandboxMode")}</span>
          <Select
            style={{ width: 180 }}
            value={settings.sandboxMode ?? "danger-full-access"}
            onChange={(v) =>
              saveSettings({
                sandboxMode: v as "read-only" | "workspace-write" | "danger-full-access",
              })}
            options={[
              { value: "read-only", label: t("advancedSettings.sandboxReadOnly") },
              { value: "workspace-write", label: t("advancedSettings.sandboxWorkspaceWrite") },
              {
                value: "danger-full-access",
                label: t("advancedSettings.sandboxDangerFullAccess"),
              },
            ]}
          />
        </div>
        <div style={{ marginTop: 2, fontSize: 12, opacity: 0.6 }}>
          {t("advancedSettings.sandboxModeHint")}
        </div>
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div style={{ padding: "4px 0" }} data-search-key="advanced:approvalPolicy">
        <div className="flex items-center justify-between">
          <span>{t("advancedSettings.approvalPolicy")}</span>
          <Select
            style={{ width: 180 }}
            value={settings.approvalPolicy ?? "on-request"}
            onChange={(v) =>
              saveSettings({
                approvalPolicy: v as "untrusted" | "on-failure" | "on-request" | "never",
              })}
            options={[
              { value: "untrusted", label: t("advancedSettings.approvalUntrusted") },
              { value: "on-failure", label: t("advancedSettings.approvalOnFailure") },
              { value: "on-request", label: t("advancedSettings.approvalOnRequest") },
              { value: "never", label: t("advancedSettings.approvalNever") },
            ]}
          />
        </div>
        <div style={{ marginTop: 2, fontSize: 12, opacity: 0.6 }}>
          {t("advancedSettings.approvalPolicyHint")}
        </div>
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div
        className="flex items-center justify-between"
        style={{ padding: "4px 0" }}
        data-search-key="advanced:tot"
      >
        <span>{t("advancedSettings.tot")}</span>
        <Switch
          checked={settings.totEnabled ?? false}
          onChange={(v) => saveSettings({ totEnabled: v })}
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
      <AgentBehaviorSection />
      <GreenContractSection />
      <DreamConsolidationSection />
      <LspDiagnosticsSection />
      <SmartRouterSection />
      <PrivacyControlSection />
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
