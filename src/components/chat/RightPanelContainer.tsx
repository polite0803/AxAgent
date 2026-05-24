import { Icon } from "@/components/common/Icon";
import { useResolvedDarkMode } from "@/hooks/useResolvedDarkMode";
import { useConversationStore, useRightPanelStore, useSettingsStore } from "@/stores";
import { useCacheStore } from "@/stores/feature/cacheStore";
import { Button, Tabs, Tooltip } from "antd";
import {
  BarChart3,
  Bug,
  Camera,
  ChevronDown,
  ChevronUp,
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
  Search,
  Share2,
  Sparkles,
  User,
  Users,
  X,
  Zap,
} from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { AgentExecutionPanel } from "./AgentExecutionPanel";
import { AgentHierarchyTree } from "./AgentHierarchyTree";
import { ArtifactPanel } from "./ArtifactPanel";
import { BenchmarkPanel } from "./BenchmarkPanel";
import { BranchComparePanel } from "./BranchComparePanel";
import { BrowserAutomationPanel } from "./BrowserAutomationPanel";
import { CacheIndicator } from "./CacheIndicator";
import { ChartInterpreter } from "./ChartInterpreter";
import { ChatInspector } from "./ChatInspector";
import { getChatCodeThemes } from "./ChatMarkdownNodes";
import { CitationManager, CitationStats } from "./CitationManager";
import { CodeExecutorPanel } from "./CodeExecutorPanel";
import { CollaborationPanel } from "./CollaborationPanel";
import { ComputerControlPanel } from "./ComputerControlPanel";
import { ContextClassificationBar } from "./ContextClassificationBar";
import { ErrorRecoveryPanel } from "./ErrorRecoveryPanel";
import { EvolutionSidebar } from "./EvolutionSidebar";
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

/** 面板分类层级 */
type PanelCategory = "core" | "agent" | "extra";

interface PanelEntry {
  key: string;
  icon: React.ReactNode;
  labelKey: string;
  category: PanelCategory;
  /** 为 false 时完全跳过渲染 */
  shouldRender: boolean;
  render: () => React.ReactNode;
}

export function RightPanelContainer({
  conversationId,
  compactMode,
  onToggleCompact,
}: RightPanelContainerProps) {
  const { t } = useTranslation();
  const [inspectorTab, setInspectorTab] = useState("overview");
  const [extrasExpanded, setExtrasExpanded] = useState(false);

  // 最小化 store selector 粒度，减少渲染触发
  const convMode = useConversationStore(
    (s) => s.conversations.find((c) => c.id === conversationId)?.mode,
  );
  const isAgent = convMode === "agent";
  const settings = useSettingsStore((s) => s.settings);
  const panelData = useRightPanelStore();
  const isDarkMode = useResolvedDarkMode(settings.theme_mode);

  // 缓存状态 — 只取所需字段
  const cacheState = useCacheStore();
  const cacheValid = cacheState.cacheValid;
  const hasPendingChanges = cacheState.hasPendingChanges;
  const tokensSaved = cacheState.tokensSaved;
  const cacheHits = cacheState.cacheHits;

  const codeThemes = useMemo(
    () => getChatCodeThemes(settings.code_theme, settings.code_theme_light),
    [settings.code_theme, settings.code_theme_light],
  );

  // ── 面板定义（静态配置 + 运行时条件） ──────────────────────────────
  const panels = useMemo<PanelEntry[]>(() => {
    const entries: PanelEntry[] = [
      // ═══ 核心面板（始终显示） ═══
      {
        key: "agent",
        icon: <Icon icon="fluent:bot-20-filled" size={ICON} />,
        labelKey: "chatRightPanel.agent",
        category: "core",
        shouldRender: true,
        render: () => (
          <AgentExecutionPanel
            conversationId={conversationId}
            compactMode={compactMode}
            onToggleCompact={onToggleCompact}
          />
        ),
      },
      {
        key: "code",
        icon: <Icon icon="fluent:code-20-filled" size={ICON} />,
        labelKey: "chatRightPanel.code",
        category: "core",
        shouldRender: true,
        render: () => <CodeExecutorPanel />,
      },
      {
        key: "artifact",
        icon: <Icon icon="fluent:color-20-filled" size={ICON} />,
        labelKey: "chatRightPanel.artifact",
        category: "core",
        shouldRender: true,
        render: () => <ArtifactPanel />,
      },
      {
        key: "citation",
        icon: <Search size={ICON} />,
        labelKey: "chatRightPanel.citation",
        category: "core",
        shouldRender: true,
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
        category: "core",
        shouldRender: true,
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
        key: "cache",
        icon: <Layers size={ICON} />,
        labelKey: "chatRightPanel.cache",
        category: "core",
        shouldRender: true,
        render: () => (
          <CacheIndicator
            cacheValid={cacheValid}
            hasPendingChanges={hasPendingChanges}
            tokensSaved={tokensSaved}
            cacheHits={cacheHits}
          />
        ),
      },

      // ═══ 智能体模式面板（仅 agent 模式下显示） ═══
      {
        key: "hierarchy",
        icon: <GitBranch size={ICON} />,
        labelKey: "chatRightPanel.hierarchy",
        category: "agent",
        shouldRender: isAgent,
        render: () => <AgentHierarchyTree conversationId={conversationId} />,
      },
      {
        key: "research",
        icon: <Microscope size={ICON} />,
        labelKey: "chatRightPanel.research",
        category: "agent",
        shouldRender: isAgent,
        render: () => <ResearchPanel />,
      },
      {
        key: "git",
        icon: <FolderGit2 size={ICON} />,
        labelKey: "chatRightPanel.git",
        category: "agent",
        shouldRender: isAgent,
        render: () => <GitCommitPanel />,
      },
      {
        key: "task",
        icon: <LayoutList size={ICON} />,
        labelKey: "chatRightPanel.task",
        category: "agent",
        shouldRender: isAgent,
        render: () => <TaskPanel />,
      },
      {
        key: "teammate",
        icon: <Users size={ICON} />,
        labelKey: "chatRightPanel.teammate",
        category: "agent",
        shouldRender: isAgent,
        render: () => <TeammatePanel conversationId={conversationId} />,
      },

      // ═══ 扩展面板（默认折叠） ═══
      {
        key: "imageGen",
        icon: <Image size={ICON} />,
        labelKey: "chatRightPanel.imageGen",
        category: "extra",
        shouldRender: true,
        render: () => <ImageGenPanel />,
      },
      {
        key: "visionAnalysis",
        icon: <Camera size={ICON} />,
        labelKey: "chatRightPanel.visionAnalysis",
        category: "extra",
        shouldRender: true,
        render: () => <ImageAnalysisPanel />,
      },
      {
        key: "report",
        icon: <FileText size={ICON} />,
        labelKey: "chatRightPanel.report",
        category: "extra",
        shouldRender: !!panelData.report,
        render: () => (
          <ReportViewer
            report={panelData.report}
            onReset={() => panelData.setReport(null)}
          />
        ),
      },
      {
        key: "browser",
        icon: <Globe size={ICON} />,
        labelKey: "chatRightPanel.browser",
        category: "extra",
        shouldRender: true,
        render: () => <BrowserAutomationPanel />,
      },
      {
        key: "computer",
        icon: <Monitor size={ICON} />,
        labelKey: "chatRightPanel.computer",
        category: "extra",
        shouldRender: true,
        render: () => <ComputerControlPanel />,
      },
      {
        key: "benchmark",
        icon: <Gauge size={ICON} />,
        labelKey: "chatRightPanel.benchmark",
        category: "extra",
        shouldRender: true,
        render: () => <BenchmarkPanel />,
      },
      {
        key: "chart",
        icon: <BarChart3 size={ICON} />,
        labelKey: "chatRightPanel.chart",
        category: "extra",
        shouldRender: !!panelData.chartData,
        render: () => (
          <ChartInterpreter
            chartData={panelData.chartData}
            rawAnalysis={panelData.chartRawAnalysis}
          />
        ),
      },
      {
        key: "snapshot",
        icon: <Camera size={ICON} />,
        labelKey: "chatRightPanel.snapshot",
        category: "extra",
        shouldRender: panelData.snapshotElements.length > 0,
        render: () => (
          <UISnapshotViewer
            elements={panelData.snapshotElements}
            rawDescription={panelData.snapshotDescription}
          />
        ),
      },
      {
        key: "profile",
        icon: <User size={ICON} />,
        labelKey: "chatRightPanel.profile",
        category: "extra",
        shouldRender: true,
        render: () => <UserProfilePanel />,
      },
      {
        key: "errorRecovery",
        icon: <Bug size={ICON} />,
        labelKey: "chatRightPanel.errorRecovery",
        category: "extra",
        shouldRender: true,
        render: () => <ErrorRecoveryPanel />,
      },
      {
        key: "collaboration",
        icon: <Share2 size={ICON} />,
        labelKey: "chatRightPanel.collaboration",
        category: "extra",
        shouldRender: true,
        render: () => <CollaborationPanel conversationId={conversationId} />,
      },
      {
        key: "evolution",
        icon: <Sparkles size={ICON} />,
        labelKey: "chatRightPanel.evolution",
        category: "extra",
        shouldRender: true,
        render: () => <EvolutionSidebar />,
      },
      {
        key: "steer",
        icon: <Zap size={ICON} />,
        labelKey: "chatRightPanel.steer",
        category: "extra",
        shouldRender: true,
        render: () => <SteerInput />,
      },
      {
        key: "gateway",
        icon: <Share2 size={ICON} />,
        labelKey: "chatRightPanel.gateway",
        category: "extra",
        shouldRender: false,
        render: () => <GatewaySessionBadge platform="" />,
      },
      {
        key: "contextClass",
        icon: <ListFilter size={ICON} />,
        labelKey: "chatRightPanel.contextClass",
        category: "extra",
        shouldRender: false,
        render: () => <ContextClassificationBar segments={[]} maxTokens={0} />,
      },
      {
        key: "reflection",
        icon: <Eye size={ICON} />,
        labelKey: "chatRightPanel.reflection",
        category: "extra",
        shouldRender: true,
        render: () => <ReflectionPanel />,
      },
      {
        key: "branchCompare",
        icon: <GitBranch size={ICON} />,
        labelKey: "chatRightPanel.branchCompare",
        category: "extra",
        shouldRender: true,
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
        category: "extra",
        shouldRender: panelData.researchSources.length > 0,
        render: () => <ResearchSources sources={panelData.researchSources} />,
      },
      {
        key: "sessionShare",
        icon: <Share2 size={ICON} />,
        labelKey: "chatRightPanel.sessionShare",
        category: "extra",
        shouldRender: false,
        render: () => (
          <SessionShareDialog
            open={false}
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
    ];

    return entries;
    // 精简依赖：isAgent 和面板特定数据变化时才重算
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    conversationId,
    compactMode,
    onToggleCompact,
    isAgent,
    inspectorTab,
    isDarkMode,
    codeThemes,
    cacheValid,
    hasPendingChanges,
    tokensSaved,
    cacheHits,
    panelData.report,
    panelData.chartData,
    panelData.snapshotElements,
    panelData.researchSources,
  ]);

  // 过滤出可见面板
  const visiblePanels = useMemo(
    () =>
      panels.filter((p) => {
        // agent 面板仅在 agent 模式下可见
        if (p.category === "agent" && !isAgent) { return false; }
        // 扩展面板仅在展开时可见
        if (p.category === "extra" && !extrasExpanded) { return false; }
        return p.shouldRender;
      }),
    [panels, isAgent, extrasExpanded],
  );

  const tabItems = visiblePanels.map((p) => ({
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
    <div className="right-panel">
      <div className="rp-header">
        <span className="rp-header-title">
          {t("chatRightPanel.title")}
        </span>
        <button
          className="rp-toggle"
          onClick={() => {
            panelData.setChartResult(null, "");
            panelData.setReport(null);
          }}
          title={t("chatRightPanel.close")}
        >
          <X size={14} />
        </button>
      </div>
      <Tabs
        size="small"
        tabPosition="top"
        items={tabItems}
        className="rp-tabs-container"
        style={{ height: "100%", flex: 1, overflow: "hidden" }}
        tabBarStyle={{ padding: "0 8px", margin: 0 }}
      />
      {!compactMode && (
        <div
          className="rp-header"
          style={{ justifyContent: "center", borderTop: "1px solid var(--border)", borderBottom: "none" }}
        >
          <Button
            type="text"
            size="small"
            block
            icon={extrasExpanded ? <ChevronDown size={14} /> : <ChevronUp size={14} />}
            onClick={() => setExtrasExpanded((v) => !v)}
          >
            {t(extrasExpanded ? "chatRightPanel.hideExtras" : "chatRightPanel.showExtras")}
          </Button>
        </div>
      )}
    </div>
  );
}
