// SPDX-License-Identifier: AGPL-3.0-only

import { Icon } from "@/components/common/Icon";
import { DropdownMenu } from "@/components/layout/DropdownMenu";
import { Tooltip } from "@/components/layout/Tooltip";
import { useResolvedDarkMode } from "@/hooks/useResolvedDarkMode";
import i18n from "@/i18n";
import { useConversationStore, useRightPanelStore, useSettingsStore } from "@/stores";
import { useCacheStore } from "@/stores/feature/cacheStore";
import {
  BarChart3,
  Camera,
  ChevronDown,
  ChevronUp,
  Eye,
  FileSearch,
  FileText,
  GitBranch,
  Globe,
  Image,
  Layers,
  LayoutList,
  Monitor,
  Search,
  Sparkles,
  User,
  Users,
  X,
  Zap,
} from "lucide-react";
import { Component, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

// 单个 tab 面板的错误边界，防止一个 tab 崩溃导致整个右侧栏白屏
class TabErrorBoundary extends Component<
  { children: React.ReactNode; tabKey: string },
  { hasError: boolean; error: Error | null }
> {
  constructor(props: { children: React.ReactNode; tabKey: string }) {
    super(props);
    this.state = { hasError: false, error: null };
  }
  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }
  render() {
    if (this.state.hasError) {
      return (
        <div style={{ padding: 16, color: "var(--muted)", fontSize: 12 }}>
          <p>{i18n.t("errorBoundary.panelLoadFailed", { tabKey: this.props.tabKey })}</p>
          <p style={{ color: "var(--danger)", fontSize: 11 }}>
            {this.state.error?.message || i18n.t("errorBoundary.unknownError")}
          </p>
        </div>
      );
    }
    return this.props.children;
  }
}

import { AgentExecutionPanel } from "./AgentExecutionPanel";
import { AgentHierarchyTree } from "./AgentHierarchyTree";
import { ArtifactPanel } from "./ArtifactPanel";
import { BrowserAutomationPanel } from "./BrowserAutomationPanel";
import { CacheIndicator } from "./CacheIndicator";
import { ChartInterpreter } from "./ChartInterpreter";
import { ChatInspector } from "./ChatInspector";
import { getChatCodeThemes } from "./ChatMarkdownNodes";
import { CitationManager, CitationStats } from "./CitationManager";
import { ComputerControlPanel } from "./ComputerControlPanel";
import { EvolutionSidebar } from "./EvolutionSidebar";
import { ImageAnalysisPanel } from "./ImageAnalysisPanel";
import { ImageGenPanel } from "./ImageGenPanel";
import { ReflectionPanel } from "./ReflectionPanel";
import { ReportViewer } from "./ReportViewer";
import { ResearchSources } from "./ResearchSources";
import { SteerInput } from "./SteerInput";
import { TaskPanel } from "./TaskPanel";
import { TeammatePanel } from "./TeammatePanel";
import { UISnapshotViewer } from "./UISnapshotViewer";
import { UserProfilePanel } from "./UserProfilePanel";

export interface RightPanelContainerProps {
  conversationId: string;
  compactMode: boolean;
  onToggleCompact: () => void;
  /** 抽屉模式（覆盖在内容上方，而非并排） */
  drawerOpen?: boolean;
  /** 抽屉关闭回调 */
  onCloseDrawer?: () => void;
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
  drawerOpen,
  onCloseDrawer,
}: RightPanelContainerProps) {
  const { t } = useTranslation();
  const [inspectorTab, setInspectorTab] = useState("overview");

  // 最小化 store selector 粒度，减少渲染触发
  const convMode = useConversationStore(
    (s) => s.conversations.find((c) => c.id === conversationId)?.mode,
  );
  const isAgent = convMode === "agent";
  const settings = useSettingsStore((s) => s.settings);
  const panelChartData = useRightPanelStore((s) => s.chartData);
  const panelChartRawAnalysis = useRightPanelStore((s) => s.chartRawAnalysis);
  const panelSnapshotElements = useRightPanelStore((s) => s.snapshotElements);
  const panelSnapshotDescription = useRightPanelStore((s) => s.snapshotDescription);
  const panelResearchSources = useRightPanelStore((s) => s.researchSources);
  const panelReport = useRightPanelStore((s) => s.report);
  const panelSetChartResult = useRightPanelStore((s) => s.setChartResult);
  const panelSetReport = useRightPanelStore((s) => s.setReport);
  const isDarkMode = useResolvedDarkMode(settings.theme_mode);

  const cacheValid = useCacheStore((s) => s.cacheValid);
  const hasPendingChanges = useCacheStore((s) => s.hasPendingChanges);
  const tokensSaved = useCacheStore((s) => s.tokensSaved);
  const cacheHits = useCacheStore((s) => s.cacheHits);

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
        shouldRender: !!panelReport,
        render: () => (
          <ReportViewer
            report={panelReport}
            onReset={() => panelSetReport(null)}
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
        key: "chart",
        icon: <BarChart3 size={ICON} />,
        labelKey: "chatRightPanel.chart",
        category: "extra",
        shouldRender: !!panelChartData,
        render: () => (
          <ChartInterpreter
            chartData={panelChartData}
            rawAnalysis={panelChartRawAnalysis}
          />
        ),
      },
      {
        key: "snapshot",
        icon: <Camera size={ICON} />,
        labelKey: "chatRightPanel.snapshot",
        category: "extra",
        shouldRender: panelSnapshotElements.length > 0,
        render: () => (
          <UISnapshotViewer
            elements={panelSnapshotElements}
            rawDescription={panelSnapshotDescription}
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
        render: () => <SteerInput conversationId={conversationId} />,
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
        key: "researchSources",
        icon: <Search size={ICON} />,
        labelKey: "chatRightPanel.researchSources",
        category: "extra",
        shouldRender: panelResearchSources.length > 0,
        render: () => <ResearchSources sources={panelResearchSources} />,
      },
    ];

    return entries;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    conversationId,
    compactMode,
    onToggleCompact,
    isAgent,
    isDarkMode,
    codeThemes,
    panelReport,
    panelChartData,
    panelChartRawAnalysis,
    panelSnapshotElements,
    panelSnapshotDescription,
    panelResearchSources,
    panelSetReport,
    panelSetChartResult,
  ]);

  // 过滤出可见面板
  const visiblePanels = useMemo(
    () =>
      panels.filter((p) => {
        // agent 面板仅在 agent 模式下可见
        if (p.category === "agent" && !isAgent) { return false; }
        return p.shouldRender;
      }),
    [panels, isAgent],
  );

  const tabsRef = useRef<HTMLDivElement>(null);
  const [visibleTabs, setVisibleTabs] = useState<PanelEntry[]>([]);
  const [overflowPanels, setOverflowPanels] = useState<PanelEntry[]>([]);

  // 计算可见 tab 和溢出 tab
  /* eslint-disable react-hooks/set-state-in-effect */
  useEffect(() => {
    const container = tabsRef.current;
    if (!container) { return; }
    const maxWidth = container.clientWidth;
    let usedWidth = 0;
    const visible: PanelEntry[] = [];
    const overflow: PanelEntry[] = [];
    // 为每个 tab 预留约 40px 宽度（icon + padding）
    const tabWidth = 40;
    for (const p of visiblePanels) {
      if (usedWidth + tabWidth + 30 <= maxWidth) { // 30px 为 ... 按钮预留
        visible.push(p);
        usedWidth += tabWidth;
      } else {
        overflow.push(p);
      }
    }
    if (overflow.length === 0) {
      setVisibleTabs(visiblePanels);
      setOverflowPanels([]);
    } else {
      setVisibleTabs(visible);
      setOverflowPanels(overflow);
    }
  }, [visiblePanels]);
  /* eslint-enable react-hooks/set-state-in-effect */

  const [activeTab, setActiveTab] = useState(() => visiblePanels[0]?.key ?? "");

  // 仅在 isAgent 切换时验证 activeTab 有效性
  /* eslint-disable react-hooks/set-state-in-effect, react-hooks/exhaustive-deps */
  useEffect(() => {
    if (!visiblePanels.some((p) => p.key === activeTab)) {
      setActiveTab(visiblePanels[0]?.key ?? "");
    }
  }, [isAgent]);
  /* eslint-enable react-hooks/set-state-in-effect, react-hooks/exhaustive-deps */

  const activePanel = visiblePanels.find((p) => p.key === activeTab);

  // ── 抽屉模式渲染 ──
  if (drawerOpen) {
    return (
      <>
        <div className="rp-drawer-backdrop" onClick={onCloseDrawer} />
        <div className="rp-drawer">
          <div className="rp-header">
            <span className="rp-header-title">
              {t("chatRightPanel.title")}
            </span>
            <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
              <button className="titlebar-btn" onClick={onCloseDrawer} title={t("chatRightPanel.close")}>
                <X size={14} />
              </button>
            </div>
          </div>
          <div className="rp-tabs" ref={tabsRef}>
            {visibleTabs.map((p) => (
              <Tooltip key={p.key} title={t(p.labelKey)} placement="bottom">
                <button
                  className={`rp-tab ${activeTab === p.key ? "active" : ""}`}
                  onClick={() => setActiveTab(p.key)}
                >
                  {p.icon}
                </button>
              </Tooltip>
            ))}
            {overflowPanels.length > 0 && (
              <DropdownMenu
                items={overflowPanels.map((p) => ({
                  key: p.key,
                  label: t(p.labelKey),
                  icon: p.icon,
                  onClick: () => setActiveTab(p.key),
                }))}
              >
                <button
                  className={`rp-tabs-overflow ${overflowPanels.some((p) => p.key === activeTab) ? "active" : ""}`}
                >
                  <span>···</span>
                </button>
              </DropdownMenu>
            )}
          </div>
          <div className="rp-body">
            <TabErrorBoundary tabKey={activeTab}>
              {activePanel?.render()}
            </TabErrorBoundary>
          </div>
        </div>
      </>
    );
  }

  return (
    <div className="right-panel open" style={{ width: "100%", minWidth: 0 }}>
      <div className="rp-header">
        <span className="rp-header-title">
          {t("chatRightPanel.title")}
        </span>
        <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
          <button
            className="titlebar-btn"
            onClick={onToggleCompact}
            title={t(compactMode ? "chatRightPanel.expand" : "chatRightPanel.collapse")}
          >
            {compactMode ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
          </button>
          <button
            className="titlebar-btn"
            onClick={() => {
              panelSetChartResult(null, "");
              panelSetReport(null);
              onToggleCompact();
            }}
            title={t("chatRightPanel.close")}
          >
            <X size={14} />
          </button>
        </div>
      </div>
      <div className="rp-tabs" ref={tabsRef}>
        {visibleTabs.map((p) => (
          <Tooltip key={p.key} title={t(p.labelKey)} placement="bottom">
            <button
              className={`rp-tab ${activeTab === p.key ? "active" : ""}`}
              onClick={() => setActiveTab(p.key)}
            >
              {p.icon}
            </button>
          </Tooltip>
        ))}
        {overflowPanels.length > 0 && (
          <DropdownMenu
            items={overflowPanels.map((p) => ({
              key: p.key,
              label: t(p.labelKey),
              icon: p.icon,
              onClick: () => setActiveTab(p.key),
            }))}
          >
            <button className={`rp-tabs-overflow ${overflowPanels.some((p) => p.key === activeTab) ? "active" : ""}`}>
              <span>···</span>
            </button>
          </DropdownMenu>
        )}
      </div>
      <div className="rp-body" style={{ flex: 1, overflow: "auto", paddingBottom: 16 }}>
        <TabErrorBoundary tabKey={activeTab}>
          {activePanel?.render()}
        </TabErrorBoundary>
      </div>
    </div>
  );
}
