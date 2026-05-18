import { useResolvedDarkMode } from "@/hooks/useResolvedDarkMode";
import { useConversationStore, useSettingsStore } from "@/stores";
import { Tabs, theme, Tooltip } from "antd";
import {
  Activity,
  BarChart3,
  Bell,
  Bug,
  Camera,
  Clock,
  Code2,
  Eye,
  FileSearch,
  FileText,
  FolderGit2,
  Gauge,
  GitBranch,
  Globe,
  Image,
  Layers,
  LayoutList,
  ListFilter,
  Microscope,
  Monitor,
  Palette,
  Search,
  Share2,
  Shield,
  Sparkles,
  User,
  Users,
  Zap,
} from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ContextPredictionPanel } from "../proactive/ContextPredictionPanel";
import { PrefetchIndicator } from "../proactive/PrefetchIndicator";
import { ReminderList } from "../proactive/ReminderList";
import { AgentExecutionPanel } from "./AgentExecutionPanel";
import { AgentHierarchyTree } from "./AgentHierarchyTree";
import { ArtifactPanel } from "./ArtifactPanel";
import { BenchmarkPanel } from "./BenchmarkPanel";
import { BranchComparePanel } from "./BranchComparePanel";
import { BrowserAutomationPanel } from "./BrowserAutomationPanel";
import { CacheIndicator } from "./CacheIndicator";
import { CategoryEditModal } from "./CategoryEditModal";
import { ChartInterpreter } from "./ChartInterpreter";
import { ChatInspector } from "./ChatInspector";
import { getChatCodeThemes } from "./ChatMarkdownNodes";
import { CitationManager, CitationStats } from "./CitationManager";
import { CodeExecutorPanel } from "./CodeExecutorPanel";
import { CollaborationPanel } from "./CollaborationPanel";
import { ComputerControlPanel } from "./ComputerControlPanel";
import { ContextClassificationBar } from "./ContextClassificationBar";
import { CronResultMessage } from "./CronResultMessage";
import { ErrorRecoveryPanel } from "./ErrorRecoveryPanel";
import { EvolutionSidebar } from "./EvolutionSidebar";
import { FilePermissionDialog } from "./FilePermissionDialog";
import { GatewaySessionBadge } from "./GatewaySessionBadge";
import { GitCommitPanel } from "./GitCommitPanel";
import { ImageAnalysisPanel } from "./ImageAnalysisPanel";
import { ImageGenPanel } from "./ImageGenPanel";
import { ReflectionPanel } from "./ReflectionPanel";
import { ReportViewer } from "./ReportViewer";
import { ResearchPanel } from "./ResearchPanel";
import { ResearchSources } from "./ResearchSources";
import { SessionShareDialog } from "./SessionShareDialog";
import { SteerInput } from "./SteerInput";
import { TaskPanel } from "./TaskPanel";
import { TeammatePanel } from "./TeammatePanel";
import { UISnapshotViewer } from "./UISnapshotViewer";
import { UserProfilePanel } from "./UserProfilePanel";

export interface RightPanelContainerProps {
  conversationId: string;
  compactMode: boolean;
  onToggleCompact: () => void;
}

const ICON = 14;

interface PanelEntry {
  key: string;
  icon: React.ReactNode;
  labelKey: string;
  render: () => React.ReactNode;
}

export function RightPanelContainer({
  conversationId,
  compactMode,
  onToggleCompact,
}: RightPanelContainerProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [inspectorTab, setInspectorTab] = useState("overview");

  const convMode = useConversationStore(
    (s) => s.conversations.find((c) => c.id === conversationId)?.mode,
  );
  const isAgent = convMode === "agent";
  const settings = useSettingsStore((s) => s.settings);
  const isDarkMode = useResolvedDarkMode(settings.theme_mode);

  const codeThemes = useMemo(
    () => getChatCodeThemes(settings.code_theme, settings.code_theme_light),
    [settings.code_theme, settings.code_theme_light],
  );

  const panels = useMemo<PanelEntry[]>(() => {
    const entries: PanelEntry[] = [
      {
        key: "agent",
        icon: <Activity size={ICON} />,
        labelKey: "chatRightPanel.agent",
        render: () => (
          <AgentExecutionPanel
            conversationId={conversationId}
            compactMode={compactMode}
            onToggleCompact={onToggleCompact}
          />
        ),
      },
    ];

    if (isAgent) {
      entries.push(
        {
          key: "hierarchy",
          icon: <GitBranch size={ICON} />,
          labelKey: "chatRightPanel.hierarchy",
          render: () => <AgentHierarchyTree conversationId={conversationId} />,
        },
        {
          key: "research",
          icon: <Microscope size={ICON} />,
          labelKey: "chatRightPanel.research",
          render: () => <ResearchPanel />,
        },
        {
          key: "git",
          icon: <FolderGit2 size={ICON} />,
          labelKey: "chatRightPanel.git",
          render: () => <GitCommitPanel />,
        },
        {
          key: "task",
          icon: <LayoutList size={ICON} />,
          labelKey: "chatRightPanel.task",
          render: () => <TaskPanel />,
        },
        {
          key: "teammate",
          icon: <Users size={ICON} />,
          labelKey: "chatRightPanel.teammate",
          render: () => <TeammatePanel conversationId={conversationId} />,
        },
      );
    }

    entries.push(
      {
        key: "code",
        icon: <Code2 size={ICON} />,
        labelKey: "chatRightPanel.code",
        render: () => <CodeExecutorPanel />,
      },
      {
        key: "artifact",
        icon: <Palette size={ICON} />,
        labelKey: "chatRightPanel.artifact",
        render: () => <ArtifactPanel />,
      },
      {
        key: "imageGen",
        icon: <Image size={ICON} />,
        labelKey: "chatRightPanel.imageGen",
        render: () => <ImageGenPanel />,
      },
      {
        key: "visionAnalysis",
        icon: <Camera size={ICON} />,
        labelKey: "chatRightPanel.visionAnalysis",
        render: () => <ImageAnalysisPanel />,
      },
      {
        key: "report",
        icon: <FileText size={ICON} />,
        labelKey: "chatRightPanel.report",
        render: () => <ReportViewer report={null} />,
      },
      {
        key: "citation",
        icon: <Search size={ICON} />,
        labelKey: "chatRightPanel.citation",
        render: () => (
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            <CitationStats />
            <CitationManager />
          </div>
        ),
      },
      {
        key: "inspector",
        icon: <FileSearch size={ICON} />,
        labelKey: "chatRightPanel.inspector",
        render: () => (
          <ChatInspector
            visible={true}
            activeTab={inspectorTab}
            onTabChange={setInspectorTab}
            conversationId={conversationId}
          />
        ),
      },
      {
        key: "browser",
        icon: <Globe size={ICON} />,
        labelKey: "chatRightPanel.browser",
        render: () => <BrowserAutomationPanel />,
      },
      {
        key: "computer",
        icon: <Monitor size={ICON} />,
        labelKey: "chatRightPanel.computer",
        render: () => <ComputerControlPanel />,
      },
      {
        key: "benchmark",
        icon: <Gauge size={ICON} />,
        labelKey: "chatRightPanel.benchmark",
        render: () => <BenchmarkPanel />,
      },
      {
        key: "chart",
        icon: <BarChart3 size={ICON} />,
        labelKey: "chatRightPanel.chart",
        render: () => <ChartInterpreter chartData={null} rawAnalysis="" />,
      },
      {
        key: "snapshot",
        icon: <Camera size={ICON} />,
        labelKey: "chatRightPanel.snapshot",
        render: () => <UISnapshotViewer elements={[]} rawDescription="" />,
      },
      {
        key: "profile",
        icon: <User size={ICON} />,
        labelKey: "chatRightPanel.profile",
        render: () => <UserProfilePanel />,
      },
      {
        key: "errorRecovery",
        icon: <Bug size={ICON} />,
        labelKey: "chatRightPanel.errorRecovery",
        render: () => <ErrorRecoveryPanel />,
      },
      {
        key: "collaboration",
        icon: <Share2 size={ICON} />,
        labelKey: "chatRightPanel.collaboration",
        render: () => <CollaborationPanel conversationId={conversationId} />,
      },
      {
        key: "evolution",
        icon: <Sparkles size={ICON} />,
        labelKey: "chatRightPanel.evolution",
        render: () => <EvolutionSidebar />,
      },
      // ── 数据面板 ──
      {
        key: "steer",
        icon: <Zap size={ICON} />,
        labelKey: "chatRightPanel.steer",
        render: () => <SteerInput />,
      },
      {
        key: "cache",
        icon: <Layers size={ICON} />,
        labelKey: "chatRightPanel.cache",
        render: () => (
          <CacheIndicator
            cacheValid={false}
            hasPendingChanges={false}
            tokensSaved={0}
            cacheHits={0}
          />
        ),
      },
      {
        key: "gateway",
        icon: <Share2 size={ICON} />,
        labelKey: "chatRightPanel.gateway",
        render: () => <GatewaySessionBadge platform="" />,
      },
      {
        key: "contextClass",
        icon: <ListFilter size={ICON} />,
        labelKey: "chatRightPanel.contextClass",
        render: () => <ContextClassificationBar segments={[]} maxTokens={0} />,
      },
      {
        key: "cronResult",
        icon: <Clock size={ICON} />,
        labelKey: "chatRightPanel.cronResult",
        render: () => (
          <CronResultMessage
            jobName=""
            schedule=""
            result=""
            success={false}
            timestamp={0}
          />
        ),
      },
      {
        key: "reflection",
        icon: <Eye size={ICON} />,
        labelKey: "chatRightPanel.reflection",
        render: () => <ReflectionPanel />,
      },
      {
        key: "branchCompare",
        icon: <GitBranch size={ICON} />,
        labelKey: "chatRightPanel.branchCompare",
        render: () => (
          <BranchComparePanel
            isDarkMode={isDarkMode}
            codeBlockDarkTheme={codeThemes.darkTheme}
            codeBlockLightTheme={codeThemes.lightTheme}
            codeBlockThemes={codeThemes.themes}
          />
        ),
      },
      {
        key: "researchSources",
        icon: <Search size={ICON} />,
        labelKey: "chatRightPanel.researchSources",
        render: () => <ResearchSources sources={[]} />,
      },
      // ── 对话框（作为面板内容直接展示）──
      {
        key: "categoryEdit",
        icon: <Layers size={ICON} />,
        labelKey: "chatRightPanel.categoryEdit",
        render: () => (
          <CategoryEditModal
            open={true}
            onClose={() => {}}
            onOk={(_data) => {}}
          />
        ),
      },
      {
        key: "filePermission",
        icon: <Shield size={ICON} />,
        labelKey: "chatRightPanel.filePermission",
        render: () => <FilePermissionDialog open={true} onClose={() => {}} path="" />,
      },
      {
        key: "sessionShare",
        icon: <Share2 size={ICON} />,
        labelKey: "chatRightPanel.sessionShare",
        render: () => (
          <SessionShareDialog
            open={true}
            sessionId={conversationId}
            onClose={() => {}}
            permissions={{
              allow_terminal_access: false,
              allow_file_access: false,
              allow_model_access: false,
              require_approval_for_actions: true,
              max_participants: 5,
            }}
          />
        ),
      },
      // ── Proactive ──
      {
        key: "prefetch",
        icon: <Zap size={ICON} />,
        labelKey: "chatRightPanel.prefetch",
        render: () => <PrefetchIndicator />,
      },
      {
        key: "reminders",
        icon: <Bell size={ICON} />,
        labelKey: "chatRightPanel.reminders",
        render: () => <ReminderList />,
      },
      {
        key: "contextPrediction",
        icon: <Eye size={ICON} />,
        labelKey: "chatRightPanel.contextPrediction",
        render: () => <ContextPredictionPanel context={{}} />,
      },
    );

    return entries;
  }, [
    conversationId,
    compactMode,
    onToggleCompact,
    isAgent,
    inspectorTab,
    isDarkMode,
    codeThemes,
  ]);

  const tabItems = panels.map((p) => ({
    key: p.key,
    label: (
      <Tooltip title={t(p.labelKey)} placement="left">
        <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
          {p.icon}
          {!compactMode && <span style={{ fontSize: 12 }}>{t(p.labelKey)}</span>}
        </span>
      </Tooltip>
    ),
    children: (
      <div style={{ height: "100%", overflow: "auto", paddingBottom: 16 }}>
        {p.render()}
      </div>
    ),
  }));

  return (
    <div
      className="flex flex-col h-full"
      style={{
        backgroundColor: token.colorBgContainer,
        borderLeft: `1px solid ${token.colorBorderSecondary}`,
      }}
    >
      <Tabs
        size="small"
        tabPosition={compactMode ? "top" : "left"}
        items={tabItems}
        style={{ height: "100%", flex: 1 }}
        tabBarStyle={compactMode
          ? { padding: "4px 8px 0" }
          : { width: 44, padding: "8px 0" }}
      />
    </div>
  );
}
