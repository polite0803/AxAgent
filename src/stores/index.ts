// SPDX-License-Identifier: AGPL-3.0-only

export { useAgentPanelStore } from "./shared/agentPanelStore";
export type { AgentContext, AgentPanelTab, AgentSelection } from "./shared/agentPanelStore";
export { useArtifactStore } from "./shared/artifactStore";
export { type TabItem, useTabStore } from "./shared/tabStore";
export { useUIStore } from "./shared/uiStore";
export { useWorkspaceStore as useWorkspaceTabStore, type WorkspaceTab } from "./shared/workspaceStore";

export {
  type AnimationMode,
  initAnimationPreference,
  isAnimationEnabled,
  useAnimationStore,
} from "./shared/animationStore";
export { useBackendStatusStore } from "./shared/backendStatusStore";
export { useErrorNotificationStore } from "./shared/errorNotificationStore";

export { useDecompositionStore } from "./feature/decompositionStore";
export { useDynamicUIStore } from "./feature/dynamicUIStore";
export { useIndustryLearningStore } from "./feature/industryLearningStore";
export { useKnowledgeSourceStore } from "./feature/knowledgeSourceStore";
export { useLlmWikiStore } from "./feature/llmWikiStore";
export { useProactiveStore } from "./feature/proactiveStore";
export { useStyleStore } from "./feature/styleStore";
export { useThemeStore } from "./feature/themeStore";
export { useWikiStore } from "./feature/wikiStore";

export { useEvaluatorStore } from "./devtools/evaluatorStore";
export { useFineTuneStore } from "./devtools/fineTuneStore";
export { useRecommendationStore } from "./devtools/recommendationStore";
export { useTracerStore } from "./devtools/tracerStore";
export { useCompressStore } from "./domain/compressStore";
export { useConversationStore } from "./domain/conversationStore";
export { useMultiModelStore } from "./domain/multiModelStore";
export { usePreferenceStore } from "./domain/preferenceStore";
export { useStreamStore } from "./domain/streamStore";
export { useAgentRoleStore } from "./feature/agentRoleStore";
export { setupAgentEventListeners, useAgentStore } from "./feature/agentStore";
export { useAppConfigStore } from "./feature/appConfigStore";
export type { FeatureFlags, ModelTier as AppConfigModelTier } from "./feature/appConfigStore";
export { useApprovalStore } from "./feature/approvalStore";
export { useBackupStore } from "./feature/backupStore";
export { useBuddyStore } from "./feature/buddyStore";
export type { BuddyInfo, BuddyMessage, BuddyMood } from "./feature/buddyStore";
export { useCacheStore } from "./feature/cacheStore";
export { useCapabilityStore } from "./feature/capabilityStore";
export { useCategoryStore } from "./feature/categoryStore";
export { useCognitiveRouteStore } from "./feature/cognitiveRouteStore";
export { useContextSourceStore } from "./feature/contextSourceStore";
export { formatCny, useCurrencyStore, useFormatCny } from "./feature/currencyStore";
export { useDeviceSyncStore } from "./feature/deviceSyncStore";
export { setupDreamEventListeners, useDreamStore } from "./feature/dreamStore";
export { useEvolutionStore } from "./feature/evolutionStore";
export type { ABTestResult, EngineLog, EngineStatus, EvolutionEvent, SkillVersion } from "./feature/evolutionStore";
export { useExecutionStore } from "./feature/executionStore";
export { useExpertStore } from "./feature/expertStore";
export { useGatewayLinkStore } from "./feature/gatewayLinkStore";
export { initGatewayStatusListener, useGatewayStore } from "./feature/gatewayStore";
export { useKnowledgeGraphStore } from "./feature/knowledgeGraphStore";
export { useKnowledgeStore } from "./feature/knowledgeStore";
export { useLocalToolStore } from "./feature/localToolStore";
export { useMcpStore } from "./feature/mcpStore";
export type { DiscoveredMcpServer } from "./feature/mcpStore";
export { useMemoryStore } from "./feature/memoryStore";
export { useMultiAgentStore } from "./feature/multiAgentStore";
export { useNudgeStore } from "./feature/nudgeStore";
export { useOfficeStore } from "./feature/officeStore";
export { useOnboardingStore } from "./feature/onboardingStore";
export { usePaperStore } from "./feature/paperStore";
export { usePipelineStore } from "./feature/pipelineStore";
export { setupPlanEventListeners, usePlanStore } from "./feature/planStore";
export { usePlatformStore } from "./feature/platformStore";
export { usePluginStore } from "./feature/pluginStore";
export { usePromptTemplateStore } from "./feature/promptTemplateStore";
export { useProviderStore } from "./feature/providerStore";
export { useReadingListStore } from "./feature/readingListStore";
export { useRlTrainingStore } from "./feature/rlTrainingStore";
export type { CheckpointInfo, RLTrainingConfig, TrainingMetrics } from "./feature/rlTrainingStore";
export { useSchedulerStore } from "./feature/schedulerStore";
export { useSearchStore } from "./feature/searchStore";
export { useSecurityStore } from "./feature/securityStore";
export { useSettingsStore } from "./feature/settingsStore";
export type { GlobalShortcutDiagnostic, GlobalShortcutStatus } from "./feature/settingsStore";
export { useSkillExtensionStore } from "./feature/skillExtensionStore";
export { useSkillStore } from "./feature/skillStore";
export { useSourceStore } from "./feature/sourceStore";
export { setupTaskShapeApprovalListeners, useTaskShapeApprovalStore } from "./feature/taskShapeApprovalStore";
export type { TaskShapeApprovalEvent } from "./feature/taskShapeApprovalStore";
export { useTerminalStore } from "./feature/terminalStore";
export { useTopicGroupStore } from "./feature/topicGroupStore";
export { useTrajectoryStore } from "./feature/trajectoryStore";
export { useTTSChannelStore } from "./feature/ttsChannelStore";
export { useUserProfileStore } from "./feature/userProfileStore";
export type { AvatarType } from "./feature/userProfileStore";
export { useVoicePreferenceStore } from "./feature/voicePreferenceStore";
export { useWorkEngineStore } from "./feature/workEngineStore";
export type { PausedExecutionInfo } from "./feature/workEngineStore";
export { useWorkflowEditorStore } from "./feature/workflowEditorStore";
export type { SaveSkillWorkflowResponse, SimilarWorkflow } from "./feature/workflowEditorStore";
export { useWorkflowStore } from "./feature/workflowStore";

// ── AxInvest stock analysis ──
export { useStockAnalysisStore } from "./feature/stockAnalysisStore";
export type {
  ExperimentRecord,
  PortfolioDashboard,
  PortfolioStressBundle,
  PortfolioStressResult,
} from "./feature/stockAnalysisStore";
// ── G2 模拟观察组合（Paper Trading Portfolio） ──
export { usePaperPortfolioStore } from "./feature/paperPortfolioStore";

// ── G4 市场主线自动提炼（Market Mainline） ──
export { useMarketMainlineStore } from "./feature/marketMainlineStore";

// ── G6 截图持仓诊断（Screenshot Diagnosis） ──
export { useScreenshotDiagnosisStore } from "./feature/screenshotDiagnosisStore";

// ── G1 跨市场数据接入（美股/港股/外汇/基准指数） ──
export { useCrossMarketStore } from "./feature/crossMarketStore";
export type { CrossMarketState } from "./feature/crossMarketStore";

// ── AxInvest 股票工作区（激进重构：单股票全生命周期） ──
export { useWorkspaceStore } from "./feature/workspaceStore";
export type { RecentStock, UserMode, WorkspaceView } from "./feature/workspaceStore";

// ── AxInvest quant 模块（回测 + 策略管理）──
export { useBacktestStore } from "./feature/quant";
export { useStrategyStore } from "./feature/quant";

// ── OPC 一人公司（One-Person Company） ──
export { useOpcStore } from "./feature/opcStore";
export type {
  OpcActions,
  OpcCustomer,
  OpcDashboardSummary,
  OpcInvoice,
  OpcProject,
  OpcState,
} from "./feature/opcStore";

// ── AxInvest 交易执行桥接（Execution Bridge） ──
export { setupExecutionBridgeEventListeners, useExecutionBridgeStore } from "./feature/executionBridgeStore";
