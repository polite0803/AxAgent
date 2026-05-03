import {
  AboutPage,
  AcpSettings,
  BackupCenter,
  DashboardPluginsSettings,
  DataManager,
  DisplaySettings,
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
  SkillsHubSettings,
  StorageSpaceManager,
  ToolManager,
  UserProfileSettings,
  WebhookSettings,
  WorkflowSettings,
} from "@/components/settings";
import { AdvancedSettings } from "@/components/settings/AdvancedSettings";
import { ConversationSettings } from "@/components/settings/ConversationSettings";
import { DefaultModelSettings } from "@/components/settings/DefaultModelSettings";
import { SkillPageRenderer } from "@/components/skill/SkillPageRenderer";
import { WorkflowEditor } from "@/components/workflow";
import { useSkillExtensionStore, useUIStore } from "@/stores";
import type { SettingsSection } from "@/types";
import { theme } from "antd";
import { useState } from "react";
import { ReactFlowProvider } from "reactflow";

const SECTION_COMPONENTS: Record<SettingsSection, React.ComponentType<any>> = {
  providers: ProviderSettings,
  conversationSettings: ConversationSettings,
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
  skillsHub: SkillsHubSettings,
  dashboardPlugins: DashboardPluginsSettings,
  webhooks: WebhookSettings,
  messageChannels: MessageChannelSettings,
  advanced: AdvancedSettings,
  promptTemplates: PromptTemplatesSettings,
  acp: AcpSettings,
};

export function SettingsPage() {
  const { token } = theme.useToken();
  const settingsSection = useUIStore((s) => s.settingsSection);
  const workflowEditorOpen = useUIStore((s) => s.workflowEditorOpen);
  const openWorkflowEditor = useUIStore((s) => s.openWorkflowEditor);
  const closeWorkflowEditor = useUIStore((s) => s.closeWorkflowEditor);
  const ContentComponent = SECTION_COMPONENTS[settingsSection as keyof typeof SECTION_COMPONENTS];
  const skillSections = useSkillExtensionStore((s) => s.settingsSections);

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
        className="w-56 shrink-0 h-full"
        style={{ borderRight: "1px solid var(--border-color)", backgroundColor: token.colorBgContainer }}
      >
        <SettingsSidebar />
      </div>
      <div className="min-w-0 flex-1 overflow-y-auto" style={{ backgroundColor: token.colorBgElevated }}>
        {settingsSection === "workflow"
          ? renderWorkflowContent()
          : isSkillSection && skillSectionData
          ? (
            <SkillPageRenderer
              componentType={skillSectionData.componentType}
              componentConfig={skillSectionData.componentConfig}
              skillName={skillSectionData.skillName}
            />
          )
          : ContentComponent
          ? <ContentComponent />
          : (
            <div style={{ padding: 24, textAlign: "center", color: "var(--color-text-secondary)" }}>
              Unknown settings section: {settingsSection}
            </div>
          )}
      </div>
    </div>
  );
}
