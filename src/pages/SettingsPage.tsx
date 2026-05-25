import { SettingsSidebar } from "@/components/settings";
import { ErrorBoundary } from "@/components/shared/ErrorBoundary";
import { SkillPageRenderer } from "@/components/skill/SkillPageRenderer";
import { invoke } from "@/lib/invoke";
import { useSkillExtensionStore, useUIStore } from "@/stores";
import type { SettingsSection } from "@/types";
import { Button, message, Result, Spin, theme } from "antd";
import { lazy, Suspense, useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

/* 按需 lazy-load 各设置 section，避免一次性加载 30+ 组件阻塞首屏 */
const LazySkillsPage = lazy(() => import("@/pages/SkillsPage").then((m) => ({ default: m.SkillsPage })));
const LazyWorkflowEditor = lazy(() => import("@/components/workflow").then((m) => ({ default: m.WorkflowEditor })));
const LazyReactFlowProvider = lazy(() => import("reactflow").then((m) => ({ default: m.ReactFlowProvider })));
const LazyNotificationCenter = lazy(() =>
  import("@/components/notification/NotificationCenter").then((m) => ({ default: m.NotificationCenter }))
);
const LazyProviderSettings = lazy(() =>
  import("@/components/settings/ProviderSettings").then((m) => ({ default: m.ProviderSettings }))
);
const LazyConversationSettings = lazy(() =>
  import("@/components/settings/ConversationSettings").then((m) => ({ default: m.ConversationSettings }))
);
const LazyCloudWorkspaceSettings = lazy(() =>
  import("@/components/settings/CloudWorkspaceSelector").then((m) => ({ default: m.CloudWorkspaceSelector }))
);
const LazyDefaultModelSettings = lazy(() =>
  import("@/components/settings/DefaultModelSettings").then((m) => ({ default: m.DefaultModelSettings }))
);
const LazyGeneralSettings = lazy(() =>
  import("@/components/settings/GeneralSettings").then((m) => ({ default: m.GeneralSettings }))
);
const LazyDisplaySettings = lazy(() =>
  import("@/components/settings/DisplaySettings").then((m) => ({ default: m.DisplaySettings }))
);
const LazyProxySettings = lazy(() =>
  import("@/components/settings/ProxySettings").then((m) => ({ default: m.ProxySettings }))
);
const LazyShortcutSettings = lazy(() =>
  import("@/components/settings/ShortcutSettings").then((m) => ({ default: m.ShortcutSettings }))
);
const LazyDataManager = lazy(() =>
  import("@/components/settings/DataManager").then((m) => ({ default: m.DataManager }))
);
const LazyStorageSpaceManager = lazy(() =>
  import("@/components/settings/StorageSpaceManager").then((m) => ({ default: m.StorageSpaceManager }))
);
const LazySchedulerSettings = lazy(() =>
  import("@/components/settings/SchedulerSettings").then((m) => ({ default: m.SchedulerSettings }))
);
const LazyAboutPage = lazy(() => import("@/components/settings/AboutPage").then((m) => ({ default: m.AboutPage })));
const LazySearchProviderSettings = lazy(() =>
  import("@/components/settings/SearchProviderSettings").then((m) => ({ default: m.SearchProviderSettings }))
);
const LazyLocalToolSettings = lazy(() =>
  import("@/components/settings/LocalToolSettings").then((m) => ({ default: m.LocalToolSettings }))
);
const LazyMcpServerSettings = lazy(() =>
  import("@/components/settings/McpServerSettings").then((m) => ({ default: m.McpServerSettings }))
);
const LazyToolManager = lazy(() =>
  import("@/components/settings/ToolManager").then((m) => ({ default: m.ToolManager }))
);
const LazyBackupCenter = lazy(() =>
  import("@/components/settings/BackupCenter").then((m) => ({ default: m.BackupCenter }))
);
const LazyWorkflowSettings = lazy(() =>
  import("@/components/settings/WorkflowSettings").then((m) => ({ default: m.WorkflowSettings }))
);
const LazySettingsPanel = lazy(() =>
  import("@/components/settings/SettingsPanel").then((m) => ({ default: m.SettingsPanel }))
);
const LazyPluginMarketplace = lazy(() =>
  import("@/components/chat/PluginMarketplace").then((m) => ({ default: m.PluginMarketplace }))
);
const LazyKnowledgeSettings = lazy(() =>
  import("@/components/settings/KnowledgeSettings").then((m) => ({ default: m.KnowledgeSettings }))
);
const LazyDashboardPluginsSettings = lazy(() =>
  import("@/components/settings/DashboardPluginsSettings").then((m) => ({ default: m.DashboardPluginsSettings }))
);
const LazyWebhookSettings = lazy(() =>
  import("@/components/settings/WebhookSettings").then((m) => ({ default: m.WebhookSettings }))
);
const LazyMessageChannelSettings = lazy(() =>
  import("@/components/settings/MessageChannelSettings").then((m) => ({ default: m.MessageChannelSettings }))
);
const LazyAdvancedSettings = lazy(() =>
  import("@/components/settings/AdvancedSettings").then((m) => ({ default: m.AdvancedSettings }))
);
const LazyPromptTemplatesSettings = lazy(() =>
  import("@/components/settings/PromptTemplatesSettings").then((m) => ({ default: m.PromptTemplatesSettings }))
);
const LazyAcpSettings = lazy(() =>
  import("@/components/settings/AcpSettings").then((m) => ({ default: m.AcpSettings }))
);
const LazyEvolutionSettings = lazy(() =>
  import("@/components/settings/EvolutionSettings").then((m) => ({ default: m.EvolutionSettings }))
);
const LazyImageGenSettings = lazy(() =>
  import("@/components/settings/ImageGenSettings").then((m) => ({ default: m.ImageGenSettings }))
);
const LazyThemeManager = lazy(() =>
  import("@/components/settings/ThemeManager").then((m) => ({ default: m.ThemeManager }))
);
const LazyCronManager = lazy(() =>
  import("@/components/settings/CronManager").then((m) => ({ default: m.CronManager }))
);

function SectionFallback() {
  return (
    <div style={{ display: "flex", justifyContent: "center", padding: 48 }}>
      <Spin />
    </div>
  );
}

const SECTION_COMPONENTS: Record<SettingsSection, React.ComponentType<any>> = {
  providers: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyProviderSettings />
    </Suspense>
  ),
  conversationSettings: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyConversationSettings />
    </Suspense>
  ),
  cloudWorkspace: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyCloudWorkspaceSettings />
    </Suspense>
  ),
  defaultModel: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyDefaultModelSettings />
    </Suspense>
  ),
  general: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyGeneralSettings />
    </Suspense>
  ),
  display: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyDisplaySettings />
    </Suspense>
  ),
  proxy: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyProxySettings />
    </Suspense>
  ),
  shortcuts: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyShortcutSettings />
    </Suspense>
  ),
  data: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyDataManager />
    </Suspense>
  ),
  storage: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyStorageSpaceManager />
    </Suspense>
  ),
  scheduler: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazySchedulerSettings />
    </Suspense>
  ),
  about: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyAboutPage />
    </Suspense>
  ),
  searchProviders: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazySearchProviderSettings />
    </Suspense>
  ),
  localTools: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyLocalToolSettings />
    </Suspense>
  ),
  mcpServers: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyMcpServerSettings />
    </Suspense>
  ),
  tools: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyToolManager />
    </Suspense>
  ),
  backup: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyBackupCenter />
    </Suspense>
  ),
  workflow: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyWorkflowSettings />
    </Suspense>
  ),
  appConfig: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazySettingsPanel />
    </Suspense>
  ),
  skillsHub: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazySkillsPage />
    </Suspense>
  ),
  plugins: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyPluginMarketplace />
    </Suspense>
  ),
  knowledgeSettings: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyKnowledgeSettings />
    </Suspense>
  ),
  dashboardPlugins: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyDashboardPluginsSettings />
    </Suspense>
  ),
  notificationCenter: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyNotificationCenter trigger={<span />} />
    </Suspense>
  ),
  webhooks: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyWebhookSettings />
    </Suspense>
  ),
  messageChannels: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyMessageChannelSettings />
    </Suspense>
  ),
  advanced: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyAdvancedSettings />
    </Suspense>
  ),
  promptTemplates: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyPromptTemplatesSettings />
    </Suspense>
  ),
  acp: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyAcpSettings />
    </Suspense>
  ),
  evolution: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyEvolutionSettings />
    </Suspense>
  ),
  imageGen: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyImageGenSettings />
    </Suspense>
  ),
  theme: () => (
    <Suspense fallback={<SectionFallback />}>
      <LazyThemeManager />
    </Suspense>
  ),
  cron: () => (
    <Suspense fallback={<SectionFallback />}>
      <CronManagerWrapper />
    </Suspense>
  ),
};

/** CronManager 包装组件 — 通过 invoke 桥接后端定时任务 API */
function CronManagerWrapper() {
  const { t } = useTranslation();
  const [jobs, setJobs] = useState<
    Array<{
      id: string;
      name: string;
      schedule: string;
      prompt: string;
      platform: string | null;
      enabled_toolsets: string[] | null;
      enabled: boolean;
      last_run_at: number | null;
      next_run_at: number | null;
    }>
  >([]);

  const loadJobs = useCallback(async () => {
    try {
      const tasks: any[] = await invoke("list_scheduled_tasks");
      setJobs(
        tasks.map((task: any) => ({
          id: task.id,
          name: task.name,
          schedule: task.cron_expression ?? "",
          prompt: task.description,
          platform: task.task_type ?? null,
          enabled_toolsets: null,
          enabled: task.status === "active",
          last_run_at: task.last_run_at ? Date.parse(task.last_run_at) : null,
          next_run_at: task.next_run_at ? Date.parse(task.next_run_at) : null,
        })),
      );
    } catch {
      message.error(t("error.loadFailed"));
    }
  }, [t]);

  useEffect(() => {
    loadJobs();
  }, [loadJobs]);

  const handleAdd = useCallback(
    async (job: {
      name: string;
      schedule: string;
      prompt: string;
      platform?: string;
    }) => {
      try {
        await invoke("create_scheduled_task", {
          name: job.name,
          description: job.prompt,
          cronExpression: job.schedule,
          taskType: job.platform ?? "general",
        });
        message.success(t("common.success"));
        loadJobs();
      } catch {
        message.error(t("error.saveFailed"));
      }
    },
    [t, loadJobs],
  );

  const handleDelete = useCallback(
    async (id: string) => {
      try {
        await invoke("delete_scheduled_task", { id });
        message.success(t("common.success"));
        loadJobs();
      } catch {
        message.error(t("error.deleteFailed"));
      }
    },
    [t, loadJobs],
  );

  const handleToggle = useCallback(
    async (id: string, enabled: boolean) => {
      try {
        if (enabled) {
          await invoke("resume_scheduled_task", { id });
        } else {
          await invoke("pause_scheduled_task", { id });
        }
        loadJobs();
      } catch {
        message.error(t("error.saveFailed"));
      }
    },
    [t, loadJobs],
  );

  return (
    <Suspense fallback={<SectionFallback />}>
      <LazyCronManager
        jobs={jobs}
        onAdd={handleAdd}
        onDelete={handleDelete}
        onToggle={handleToggle}
      />
    </Suspense>
  );
}

/** 单个设置 section 的错误边界，防止一个 section 崩溃导致整页白屏 */
function SectionErrorBoundary({
  sectionKey,
  children,
}: {
  sectionKey: string;
  children: React.ReactNode;
}) {
  const { t } = useTranslation();
  const setSettingsSection = useUIStore((s) => s.setSettingsSection);

  return (
    <ErrorBoundary
      fallback={
        <div
          className="flex items-center justify-center"
          style={{ padding: 48, minHeight: 300 }}
        >
          <Result
            status="error"
            title={`${t("error.page")}: ${sectionKey}`}
            subTitle={t("nav.chat")}
            extra={
              <span className="flex gap-2">
                <Button onClick={() => window.location.reload()}>
                  {t("settingsPage.refreshPage")}
                </Button>
                <Button
                  type="primary"
                  onClick={() => setSettingsSection("general")}
                >
                  {t("settings.general.title")}
                </Button>
              </span>
            }
          />
        </div>
      }
    >
      {children}
    </ErrorBoundary>
  );
}

export function SettingsPage() {
  const { token } = theme.useToken();
  const settingsSection = useUIStore((s) => s.settingsSection);
  const deviceLayout = useUIStore((s) => s.deviceLayout);
  const workflowEditorOpen = useUIStore((s) => s.workflowEditorOpen);
  const openWorkflowEditor = useUIStore((s) => s.openWorkflowEditor);
  const closeWorkflowEditor = useUIStore((s) => s.closeWorkflowEditor);
  const isSmallScreen = deviceLayout === "mobile" || deviceLayout === "tablet";
  const ContentComponent = SECTION_COMPONENTS[settingsSection as keyof typeof SECTION_COMPONENTS];
  const skillSections = useSkillExtensionStore((s) => s.settingsSections);

  // 侧边栏可拖曳宽度
  const [sidebarWidth, setSidebarWidth] = useState(224);
  const resizingRef = useRef(false);

  const handleResizeStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    resizingRef.current = true;
  }, []);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!resizingRef.current) {
        return;
      }
      setSidebarWidth(Math.max(180, Math.min(500, e.clientX)));
    };
    const handleMouseUp = () => {
      resizingRef.current = false;
    };
    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, []);

  // 检查是否是技能设置段
  const isSkillSection = typeof settingsSection === "string" && settingsSection.startsWith("skill:");
  const skillSectionData = isSkillSection
    ? skillSections.find(
      (sec) => `skill:${sec.skillName}:${sec.id}` === settingsSection,
    )
    : null;

  const [editingTemplateId, setEditingTemplateId] = useState<
    string | undefined
  >(undefined);

  const handleOpenEditor = (templateId?: string) => {
    setEditingTemplateId(templateId);
    openWorkflowEditor();
  };

  const handleCreateNew = () => {
    setEditingTemplateId(undefined);
    openWorkflowEditor();
  };

  const handleCloseEditor = () => {
    closeWorkflowEditor();
    setEditingTemplateId(undefined);
  };

  const renderWorkflowContent = () => {
    if (workflowEditorOpen) {
      return (
        <Suspense fallback={<SectionFallback />}>
          <LazyReactFlowProvider>
            <LazyWorkflowEditor
              templateId={editingTemplateId}
              onClose={handleCloseEditor}
            />
          </LazyReactFlowProvider>
        </Suspense>
      );
    }
    return (
      <Suspense fallback={<SectionFallback />}>
        <LazyWorkflowSettings
          onOpenEditor={(templateId?: string) => handleOpenEditor(templateId)}
          onCreateNew={handleCreateNew}
        />
      </Suspense>
    );
  };

  return (
    <div className="settings-layout" data-testid="settings-panel">
      {!isSmallScreen && (
        <>
          <div className="settings-sidebar" style={{ width: sidebarWidth }}>
            <SettingsSidebar />
          </div>
          <div
            className="shrink-0 cursor-col-resize select-none"
            role="separator"
            tabIndex={0}
            style={{
              width: 5,
              borderRight: "1px solid var(--border)",
              backgroundColor: "transparent",
              transition: "background-color 0.15s",
            }}
            onMouseDown={handleResizeStart}
            onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = token.colorPrimaryBg)}
            onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "transparent")}
          />
        </>
      )}
      <div className="settings-content">
        {isSmallScreen && (
          <div className="settings-sidebar">
            <SettingsSidebar />
          </div>
        )}
        {settingsSection === "workflow"
          ? (
            renderWorkflowContent()
          )
          : isSkillSection && skillSectionData
          ? (
            <SectionErrorBoundary sectionKey={settingsSection}>
              <SkillPageRenderer
                componentType={skillSectionData.componentType}
                componentConfig={skillSectionData.componentConfig}
                skillName={skillSectionData.skillName}
              />
            </SectionErrorBoundary>
          )
          : ContentComponent
          ? (
            <SectionErrorBoundary sectionKey={settingsSection}>
              <ContentComponent />
            </SectionErrorBoundary>
          )
          : (
            <div
              style={{
                padding: 24,
                textAlign: "center",
                color: "var(--color-text-secondary)",
              }}
            >
              Unknown settings section: {settingsSection}
            </div>
          )}
      </div>
    </div>
  );
}
