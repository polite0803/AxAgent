import { useState } from 'react';
import { theme } from 'antd';
import { ReactFlowProvider } from 'reactflow';
import { useUIStore } from '@/stores';
import {
  SettingsSidebar,
  ProviderSettings,
  GeneralSettings,
  DisplaySettings,
  ProxySettings,
  ShortcutSettings,
  DataManager,
  AboutPage,
  SearchProviderSettings,
  LocalToolSettings,
  McpServerSettings,
  ToolManager,
  BackupCenter,
  StorageSpaceManager,
  SchedulerSettings,
  WorkflowSettings,
} from '@/components/settings';
import { WorkflowEditor } from '@/components/workflow';
import { DefaultModelSettings } from '@/components/settings/DefaultModelSettings';
import { ConversationSettings } from '@/components/settings/ConversationSettings';
import type { SettingsSection } from '@/types';

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
};

export function SettingsPage() {
  const { token } = theme.useToken();
  const settingsSection = useUIStore((s) => s.settingsSection);
  const workflowEditorOpen = useUIStore((s) => s.workflowEditorOpen);
  const openWorkflowEditor = useUIStore((s) => s.openWorkflowEditor);
  const closeWorkflowEditor = useUIStore((s) => s.closeWorkflowEditor);
  const ContentComponent = SECTION_COMPONENTS[settingsSection];

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
    <div className="flex h-full">
      <div
        className="w-56 shrink-0 h-full"
        style={{ borderRight: '1px solid var(--border-color)', backgroundColor: token.colorBgContainer }}
      >
        <SettingsSidebar />
      </div>
      <div className="min-w-0 flex-1 overflow-y-auto" style={{ backgroundColor: token.colorBgElevated }}>
        {settingsSection === 'workflow' ? (
          renderWorkflowContent()
        ) : (
          <ContentComponent />
        )}
      </div>
    </div>
  );
}
