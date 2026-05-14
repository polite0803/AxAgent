import {
  AboutPage,
  AcpSettings,
  AdvancedSettings,
  BackupCenter,
  CloudWorkspaceSettings,
  DashboardPluginsSettings,
  DataManager,
  DisplaySettings,
  EvolutionSettings,
  GeneralSettings,
  LocalToolSettings,
  McpServerSettings,
  MessageChannelSettings,
  PromptTemplatesSettings,
  ProviderSettings,
  ProxySettings,
  SchedulerSettings,
  SearchProviderSettings,
  SettingsPanel,
  SettingsSidebar,
  ShortcutSettings,
  StorageSpaceManager,
  ToolManager,
  UserProfileSettings,
  WebhookSettings,
  WorkflowSettings,
} from "@/components/settings";
import { ConversationSettings } from "@/components/settings/ConversationSettings";
import { DefaultModelSettings } from "@/components/settings/DefaultModelSettings";
import { ErrorBoundary } from "@/components/shared/ErrorBoundary";
import { SkillPageRenderer } from "@/components/skill/SkillPageRenderer";
import { WorkflowEditor } from "@/components/workflow";
import { SkillsPage } from "@/pages/SkillsPage";
import { useSkillExtensionStore, useUIStore } from "@/stores";
import type { SettingsSection } from "@/types";
import { Button, Result, theme } from "antd";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ReactFlowProvider } from "reactflow";

const SECTION_COMPONENTS: Record<SettingsSection, React.ComponentType<any>> = {
  providers: ProviderSettings,
  conversationSettings: ConversationSettings,
  cloudWorkspace: CloudWorkspaceSettings,
  defaultModel: DefaultModelSettings,
  general: GeneralSettings,
  display: DisplaySettings,
  proxy: ProxySettings,
  shortcuts: ShortcutSettings,
  data: DataManager,
  storage: StorageSpaceManager,
  scheduler: SchedulerSettings,
  about: AboutPage,
  searchProviders: SearchProviderSettings,
  localTools: LocalToolSettings,
  mcpServers: McpServerSettings,
  tools: ToolManager,
  backup: BackupCenter,
  workflow: WorkflowSettings,
  appConfig: SettingsPanel,
  userProfile: UserProfileSettings,
  skillsHub: SkillsPage,
  dashboardPlugins: DashboardPluginsSettings,
  webhooks: WebhookSettings,
  messageChannels: MessageChannelSettings,
  advanced: AdvancedSettings,
  promptTemplates: PromptTemplatesSettings,
  acp: AcpSettings,
  evolution: EvolutionSettings,
};

/** 单个设置 section 的错误边界，防止一个 section 崩溃导致整页白屏 */
function SectionErrorBoundary({ sectionKey, children }: { sectionKey: string; children: React.ReactNode }) {
  const { t } = useTranslation();
  const setSettingsSection = useUIStore((s) => s.setSettingsSection);

  return (
    <ErrorBoundary
      fallback={
        <div className="flex items-center justify-center" style={{ padding: 48, minHeight: 300 }}>
          <Result
            status="error"
            title={`${t("error.page")}: ${sectionKey}`}
            subTitle={t("nav.chat")}
            extra={
              <span className="flex gap-2">
                <Button onClick={() => window.location.reload()}>
                  {t("settingsPage.refreshPage")}
                </Button>
                <Button type="primary" onClick={() => setSettingsSection("general")}>
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
  const workflowEditorOpen = useUIStore((s) => s.workflowEditorOpen);
  const openWorkflowEditor = useUIStore((s) => s.openWorkflowEditor);
  const closeWorkflowEditor = useUIStore((s) => s.closeWorkflowEditor);
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
      if (!resizingRef.current) { return; }
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
    ? skillSections.find((sec) => `skill:${sec.skillName}:${sec.id}` === settingsSection)
    : null;

  const [editingTemplateId, setEditingTemplateId] = useState<string | undefined>(undefined);

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
        <ReactFlowProvider>
          <WorkflowEditor templateId={editingTemplateId} onClose={handleCloseEditor} />
        </ReactFlowProvider>
      );
    }
    return (
      <WorkflowSettings
        onOpenEditor={(templateId?: string) => handleOpenEditor(templateId)}
        onCreateNew={handleCreateNew}
      />
    );
  };

  return (
    <div className="flex h-full" data-testid="settings-panel">
      <div
        className="shrink-0 h-full"
        style={{ width: sidebarWidth, backgroundColor: token.colorBgContainer }}
      >
        <SettingsSidebar />
      </div>
      <div
        className="shrink-0 cursor-col-resize select-none"
        style={{
          width: 5,
          borderRight: "1px solid var(--border-color)",
          backgroundColor: "transparent",
          transition: "background-color 0.15s",
        }}
        onMouseDown={handleResizeStart}
        onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = token.colorPrimaryBg)}
        onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "transparent")}
      />
      <div
        className="min-w-0 flex-1 flex flex-col"
        style={{ backgroundColor: token.colorBgElevated, overflowY: "auto", overflowX: "hidden" }}
      >
        {settingsSection === "workflow"
          ? renderWorkflowContent()
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
            <div style={{ padding: 24, textAlign: "center", color: "var(--color-text-secondary)" }}>
              Unknown settings section: {settingsSection}
            </div>
          )}
      </div>
    </div>
  );
}
