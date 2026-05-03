import appLogo from "@/assets/image/logo.png";
import { AtomicSkillEditor } from "@/components/atomicSkill/AtomicSkillEditor";
import { AtomicSkillList } from "@/components/atomicSkill/AtomicSkillList";
import { SkillCreateModal } from "@/components/chat/SkillCreateEditModal";
import { SkillProposalPanel } from "@/components/chat/SkillProposalPanel";
import { CopyButton } from "@/components/common/CopyButton";
import { DecompositionPreview } from "@/components/decomposition/DecompositionPreview";
import { FrontendEditorModal } from "@/components/skill/FrontendEditorModal";
import type { WorkflowEdge, WorkflowNode } from "@/components/workflow/types";
import { CHAT_ICON_COLORS } from "@/lib/iconColors";
import { invoke } from "@/lib/invoke";
import { useSkillStore, useUIStore, useWorkflowEditorStore } from "@/stores";
import { useDecompositionStore } from "@/stores/feature/decompositionStore";
import type { MarketplaceSkill, Skill } from "@/types";
import { Claude } from "@lobehub/icons";
import {
  Button,
  Card,
  Collapse,
  Dropdown,
  Empty,
  Input,
  message,
  Modal,
  Popconfirm,
  Select,
  Space,
  Spin,
  Switch,
  Tabs,
  Tag,
  theme,
  Typography,
} from "antd";
import {
  ChevronRight,
  Code,
  Cpu,
  Download,
  FolderGit2,
  FolderOpen,
  GitFork,
  Layers,
  LayoutPanelTop,
  Lightbulb,
  Package,
  Radio,
  RefreshCw,
  Sparkles,
  Star,
  Store,
  Trash2,
  Workflow,
  Wrench,
  Zap,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const INSTALL_TARGETS = [
  {
    key: "axagent",
    label: "~/.axagent/skills/",
    desc: "AxAgent",
    icon: <Sparkles size={14} color={CHAT_ICON_COLORS.Sparkles} />,
  },
  {
    key: "claude",
    label: "~/.claude/skills/",
    desc: "Claude",
    icon: <FolderOpen size={14} color={CHAT_ICON_COLORS.FolderOpen} />,
  },
  {
    key: "trae",
    label: "~/.trae/skills/",
    desc: "Trae",
    icon: <FolderOpen size={14} color={CHAT_ICON_COLORS.FolderOpen} />,
  },
  {
    key: "codebuddy",
    label: "~/.codebuddy/skills/",
    desc: "CodeBuddy",
    icon: <FolderOpen size={14} color={CHAT_ICON_COLORS.FolderOpen} />,
  },
  {
    key: "workbuddy",
    label: "~/.workbuddy/skills/",
    desc: "WorkBuddy",
    icon: <FolderOpen size={14} color={CHAT_ICON_COLORS.FolderOpen} />,
  },
  {
    key: "agents",
    label: "~/.agents/skills/",
    desc: "Agents",
    icon: <FolderOpen size={14} color={CHAT_ICON_COLORS.FolderOpen} />,
  },
] as const;

const openExternalUrl = (url: string) => {
  import("@tauri-apps/plugin-opener")
    .then(({ openUrl }) => openUrl(url))
    .catch(() => window.open(url, "_blank", "noopener,noreferrer"));
};

const { Text, Paragraph } = Typography;

const SOURCE_ICONS: Record<string, React.ReactNode> = {
  axagent: <img src={appLogo} alt="" style={{ width: 14, height: 14, verticalAlign: "middle" }} />,
  claude: <Claude.Color size={14} />,
  agents: <Radio size={14} color={CHAT_ICON_COLORS.Route} />,
  builtin: <Cpu size={14} />,
  bundled: <Package size={14} />,
  codebuddy: <Code size={14} />,
  trae: <Zap size={14} />,
  workbuddy: <Wrench size={14} />,
  project: <FolderGit2 size={14} />,
};

const SOURCE_LABELS: Record<string, string> = {
  axagent: "AxAgent",
  claude: "Claude",
  agents: "Agents",
  builtin: "内置",
  bundled: "捆绑",
  codebuddy: "CodeBuddy",
  trae: "Trae",
  workbuddy: "WorkBuddy",
  project: "项目",
};

const ALL_SOURCE_ICON = <Layers size={14} color={CHAT_ICON_COLORS.Layers} />;

function SkillCard({
  skill,
  onToggle,
  onDetail,
  onUninstall,
  onOpenDir,
  onEditFrontend,
  t,
}: {
  skill: Skill;
  onToggle: (name: string, enabled: boolean) => void;
  onDetail: (name: string) => void;
  onUninstall: (name: string) => void;
  onOpenDir: (path: string) => void;
  onEditFrontend: (name: string) => void;
  t: (key: string, opts?: Record<string, unknown>) => string;
}) {
  const hasFrontend = !!skill.frontend;
  return (
    <Card
      size="small"
      className="skill-card-hover"
      style={{ marginBottom: 8 }}
      styles={{ body: { padding: "12px 16px" } }}
    >
      <div style={{ display: "flex", alignItems: "flex-start", gap: 12 }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
            <Text
              strong
              className="skill-card-title"
              style={{ cursor: "pointer" }}
              onClick={() => onDetail(skill.name)}
            >
              {skill.name}
            </Text>
            <CopyButton text={skill.name} size={12} />
            <Tag>
              <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                {SOURCE_ICONS[skill.source] ?? <Radio size={14} />}
                {SOURCE_LABELS[skill.source] ?? (skill.source)}
              </span>
            </Tag>
            {skill.version && <Text type="secondary" style={{ fontSize: 12 }}>v{skill.version}</Text>}
            {hasFrontend && <Tag color="blue" style={{ margin: 0 }}>含 UI 扩展</Tag>}
          </div>
          <Paragraph
            type="secondary"
            ellipsis={{ rows: 2 }}
            style={{ marginBottom: 0, fontSize: 13, cursor: "pointer" }}
            onClick={() => onDetail(skill.name)}
          >
            {skill.description}
          </Paragraph>
          {skill.author && <Text type="secondary" style={{ fontSize: 12 }}>{skill.author}</Text>}
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 8, flexShrink: 0 }}>
          <Switch
            size="small"
            checked={skill.enabled}
            onChange={(checked) => onToggle(skill.name, checked)}
          />
          <Button
            type="text"
            size="small"
            icon={<FolderOpen size={14} color={CHAT_ICON_COLORS.FolderOpen} />}
            onClick={() => onOpenDir(skill.sourcePath)}
          />
          <Button
            type="text"
            size="small"
            icon={<LayoutPanelTop size={14} color={CHAT_ICON_COLORS.Trash2} />}
            onClick={() => onEditFrontend(skill.name)}
            title={hasFrontend ? "编辑前端扩展" : "添加前端扩展"}
          />
          {skill.source !== "builtin" && (
            <Popconfirm
              title={t("skills.uninstallConfirm", { name: skill.name })}
              onConfirm={() => onUninstall(skill.name)}
              okText={t("skills.uninstall")}
              cancelText={t("common.cancel")}
            >
              <Button
                type="text"
                size="small"
                danger
                icon={<Trash2 size={14} color={CHAT_ICON_COLORS.Trash2} />}
              />
            </Popconfirm>
          )}
        </div>
      </div>
    </Card>
  );
}

function MarketplaceCard({
  skill,
  onInstall,
  onDetail,
  onExtract,
  onConvert,
  installing,
  t,
  source,
}: {
  skill: MarketplaceSkill;
  onInstall: (repo: string, target: string) => void;
  onDetail: (repo: string) => void;
  onExtract: (repo: string) => void;
  onConvert: (repo: string) => void;
  installing: string | null;
  t: (key: string) => string;
  source: string;
}) {
  const githubUrl = `https://github.com/${skill.repo}`;

  return (
    <Card
      size="small"
      className="skill-card-hover"
      style={{ marginBottom: 8 }}
      styles={{ body: { padding: "12px 16px" } }}
    >
      <div style={{ display: "flex", alignItems: "flex-start", gap: 12 }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
            <Text
              strong
              className="skill-card-title"
              style={{ cursor: "pointer" }}
              onClick={() => onDetail(skill.repo)}
            >
              {skill.name}
            </Text>
            <CopyButton text={skill.name} size={12} />
            {source === "github"
              ? (
                <Text type="secondary" style={{ fontSize: 12, display: "inline-flex", alignItems: "center", gap: 2 }}>
                  <Star size={12} style={{ color: "#faad14" }} /> {skill.stars.toLocaleString()}
                </Text>
              )
              : (
                <Text type="secondary" style={{ fontSize: 12, display: "inline-flex", alignItems: "center", gap: 2 }}>
                  <Download size={12} /> {skill.installs.toLocaleString()}
                </Text>
              )}
          </div>
          {skill.description
            ? (
              <Text
                type="secondary"
                style={{ fontSize: 12, display: "block", marginBottom: 2, cursor: "pointer" }}
                onClick={() => onDetail(skill.repo)}
              >
                {skill.description}
              </Text>
            )
            : null}
          <Text type="secondary" style={{ fontSize: 12 }}>{skill.repo}</Text>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
          <Button
            size="small"
            type="text"
            icon={<GitFork size={14} color={CHAT_ICON_COLORS.GitFork} />}
            onClick={() => openExternalUrl(githubUrl)}
          >
            GitHub
          </Button>
          {skill.installed
            ? (
              skill.hasUpdate
                ? (
                  <Button
                    size="small"
                    type="primary"
                    loading={installing === skill.repo}
                    icon={<RefreshCw size={14} />}
                    onClick={() => onInstall(skill.repo, "axagent")}
                  >
                    {t("skills.update")}
                  </Button>
                )
                : (
                  <Button size="small" disabled>
                    {t("skills.installed")}
                  </Button>
                )
            )
            : (
              <Space size={4}>
                <Button
                  size="small"
                  icon={<Layers size={14} />}
                  onClick={() => onExtract(skill.repo)}
                >
                  {t("skills.extractAtomicSkills")}
                </Button>
                <Button
                  size="small"
                  type="primary"
                  icon={<Workflow size={14} />}
                  onClick={() => onConvert(skill.repo)}
                >
                  {t("skills.marketplace.convertToWorkflow")}
                </Button>
              </Space>
            )}
        </div>
      </div>
    </Card>
  );
}

export function SkillsPage() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [messageApi, contextHolder] = message.useMessage();
  const {
    skills,
    marketplaceSkills,
    loading,
    marketplaceLoading,
    marketplaceHasMore,
    selectedSkill,
    loadSkills,
    getSkill,
    toggleSkill,
    installSkill,
    uninstallSkill,
    uninstallSkillGroup,
    openSkillDir,
    searchMarketplace,
    loadMoreMarketplace,
    clearSelectedSkill,
  } = useSkillStore();

  const [installUrl, setInstallUrl] = useState("");
  const [installing, setInstalling] = useState<string | null>(null);
  const [detailOpen, setDetailOpen] = useState(false);
  const [marketplaceSource, setMarketplaceSource] = useState<"skillhub" | "github">("skillhub");
  const [marketplaceQuery, setMarketplaceQuery] = useState("");
  const marketplaceLoaded = useRef(false);
  const [marketplaceDetailOpen, setMarketplaceDetailOpen] = useState(false);
  const [marketplaceDetailContent, setMarketplaceDetailContent] = useState<
    { name: string; repo: string; content: string } | null
  >(null);
  const [marketplaceDetailLoading, setMarketplaceDetailLoading] = useState(false);
  const [sourceFilter, setSourceFilter] = useState<"all" | "axagent" | "claude" | "agents">("all");
  const [sortOrder, setSortOrder] = useState<"popular" | "latest" | "stars">("popular");
  const [decomposePreviewOpen, setDecomposePreviewOpen] = useState(false);
  const [decomposeRequest, setDecomposeRequest] = useState<
    {
      name: string;
      description: string;
      content: string;
      source: string;
      version?: string;
      repo?: string;
    } | null
  >(null);
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [proposalPanelOpen, setProposalPanelOpen] = useState(false);
  const [atomicSkillEditVisible, setAtomicSkillEditVisible] = useState(false);
  const [editingAtomicSkill, setEditingAtomicSkill] = useState<import("@/types").AtomicSkill | null>(null);
  const [editingFrontendSkill, setEditingFrontendSkill] = useState<Skill | null>(null);

  const { previewDecomposition } = useDecompositionStore();
  const { setImportedWorkflowData } = useWorkflowEditorStore();
  const { openWorkflowEditor } = useUIStore();

  useEffect(() => {
    loadSkills();
  }, [loadSkills]);

  // Re-search when source changes (if marketplace was already searched)
  useEffect(() => {
    if (marketplaceLoaded.current) {
      searchMarketplace(marketplaceQuery, marketplaceSource, sortOrder);
    }
  }, [marketplaceSource]);

  // Re-search when sort order changes
  useEffect(() => {
    if (marketplaceLoaded.current) {
      searchMarketplace(marketplaceQuery, marketplaceSource, sortOrder);
    }
  }, [sortOrder]);

  const handleTabChange = useCallback((key: string) => {
    if (key === "marketplace" && !marketplaceLoaded.current) {
      marketplaceLoaded.current = true;
      searchMarketplace("", marketplaceSource, sortOrder);
    }
  }, [searchMarketplace, marketplaceSource, sortOrder]);

  const handleInstallFromUrl = useCallback(async (target: string) => {
    if (!installUrl.trim()) { return; }
    setInstalling(installUrl);
    try {
      const name = await installSkill(installUrl.trim(), target);
      messageApi.success(t("skills.installSuccess", { name }));
      setInstallUrl("");
    } catch (e) {
      messageApi.error(String(e));
    } finally {
      setInstalling(null);
    }
  }, [installUrl, installSkill, messageApi, t]);

  const handleInstallFromMarketplace = useCallback(async (repo: string, target: string) => {
    setInstalling(repo);
    try {
      const name = await installSkill(repo, target);
      messageApi.success(t("skills.installSuccess", { name }));
    } catch (e) {
      messageApi.error(String(e));
    } finally {
      setInstalling(null);
    }
  }, [installSkill, messageApi, t]);

  const handleToggle = useCallback((name: string, enabled: boolean) => {
    toggleSkill(name, enabled);
  }, [toggleSkill]);

  const handleDetail = useCallback(async (name: string) => {
    await getSkill(name);
    setDetailOpen(true);
  }, [getSkill]);

  const handleEditFrontend = useCallback((name: string) => {
    const skill = skills.find((s) => s.name === name);
    if (skill) {
      setEditingFrontendSkill(skill);
    }
  }, [skills]);

  const handleMarketplaceDetail = useCallback(async (repo: string) => {
    const skill = marketplaceSkills.find(s => s.repo === repo);
    if (!skill) { return; }
    setMarketplaceDetailOpen(true);
    setMarketplaceDetailLoading(true);
    setMarketplaceDetailContent({ name: skill.name, repo: skill.repo, content: "" });
    try {
      const result = await invoke<{ content: string; file_name: string; found: boolean; error: string | null }>(
        "get_marketplace_skill_content",
        { repo },
      );
      if (result.found && result.content.trim()) {
        setMarketplaceDetailContent({ name: skill.name, repo: skill.repo, content: result.content });
      } else {
        setMarketplaceDetailContent({
          name: skill.name,
          repo: skill.repo,
          content: `(${result.error || t("skills.marketplace.skillsMdNotFound") || "Skill definition file not found"})`,
        });
      }
    } catch {
      setMarketplaceDetailContent({
        name: skill.name,
        repo: skill.repo,
        content: `(${t("skills.marketplace.skillsMdFetchFailed") || "Failed to fetch skill content"})`,
      });
    } finally {
      setMarketplaceDetailLoading(false);
    }
  }, [marketplaceSkills, t]);

  const handleMarketplaceExtract = useCallback(async (repo: string) => {
    const skill = marketplaceSkills.find(s => s.repo === repo);
    if (!skill) { return; }
    try {
      const result = await invoke<{ content: string; file_name: string; found: boolean; error: string | null }>(
        "get_marketplace_skill_content",
        { repo },
      );
      if (!result.found || !result.content.trim()) {
        messageApi.error(result.error || t("skills.marketplace.skillsMdNotFound") || "Skill definition file not found");
        return;
      }
      setDecomposeRequest({
        name: skill.name,
        description: skill.description,
        content: result.content,
        source: marketplaceSource,
        repo: repo,
        version: skill.currentVersion || skill.latestVersion,
      });
      setDecomposePreviewOpen(true);
    } catch {
      messageApi.error(t("skills.marketplace.skillsMdFetchFailed") || "Failed to fetch skill content");
    }
  }, [marketplaceSkills, marketplaceSource, messageApi, t]);

  const handleMarketplaceConvert = useCallback(async (repo: string) => {
    const skill = marketplaceSkills.find(s => s.repo === repo);
    if (!skill) { return; }
    try {
      const result = await invoke<{ content: string; file_name: string; found: boolean; error: string | null }>(
        "get_marketplace_skill_content",
        { repo },
      );
      if (!result.found || !result.content.trim()) {
        messageApi.error(result.error || t("skills.marketplace.skillsMdNotFound") || "Skill definition file not found");
        return;
      }
      await previewDecomposition({
        name: skill.name,
        description: skill.description,
        content: result.content,
        source: marketplaceSource,
        repo,
      });
      const { preview } = useDecompositionStore.getState();
      if (preview?.workflow_nodes && preview?.workflow_edges) {
        setImportedWorkflowData({
          nodes: preview.workflow_nodes as WorkflowNode[],
          edges: preview.workflow_edges as WorkflowEdge[],
          name: skill.name,
          description: skill.description,
          isDecompositionWorkflow: true,
          decompositionSource: {
            market: marketplaceSource,
            repo: repo,
            version: skill.currentVersion || skill.latestVersion,
            content: result.content,
          },
        });
        openWorkflowEditor();
      }
    } catch {
      messageApi.error(t("skills.marketplace.skillsMdFetchFailed") || "Failed to fetch skill content");
    }
  }, [
    marketplaceSkills,
    marketplaceSource,
    previewDecomposition,
    setImportedWorkflowData,
    openWorkflowEditor,
    messageApi,
    t,
  ]);

  const handleConvertToWorkflow = useCallback(async (repo: string) => {
    const skill = marketplaceSkills.find(s => s.repo === repo);
    if (!skill || !marketplaceDetailContent?.content) { return; }
    try {
      await previewDecomposition({
        name: skill.name,
        description: skill.description,
        content: marketplaceDetailContent.content,
        source: marketplaceSource,
        repo,
      });
      const { preview } = useDecompositionStore.getState();
      if (preview?.workflow_nodes && preview?.workflow_edges) {
        setImportedWorkflowData({
          nodes: preview.workflow_nodes as WorkflowNode[],
          edges: preview.workflow_edges as WorkflowEdge[],
          name: skill.name,
          description: skill.description,
          isDecompositionWorkflow: true,
          decompositionSource: {
            market: marketplaceSource,
            repo: repo,
            version: skill.currentVersion || skill.latestVersion,
            content: marketplaceDetailContent.content,
          },
        });
        openWorkflowEditor();
      }
    } catch (e) {
      console.error("Failed to convert skill to workflow:", e);
    }
  }, [
    marketplaceSkills,
    marketplaceDetailContent,
    marketplaceSource,
    previewDecomposition,
    setImportedWorkflowData,
    openWorkflowEditor,
  ]);

  const handleMarketplaceExtractAtomicSkills = useCallback(async () => {
    const skill = marketplaceSkills.find(s => s.repo === marketplaceDetailContent?.repo);
    if (!skill || !marketplaceDetailContent?.content) { return; }
    setDecomposeRequest({
      name: skill.name,
      description: skill.description,
      content: marketplaceDetailContent.content,
      source: marketplaceSource,
      repo: marketplaceDetailContent.repo,
      version: skill.currentVersion || skill.latestVersion,
    });
    setDecomposePreviewOpen(true);
    setMarketplaceDetailOpen(false);
  }, [marketplaceSkills, marketplaceDetailContent, marketplaceSource, setMarketplaceDetailOpen]);

  const handleConvertMySkillToWorkflow = useCallback(async () => {
    if (!selectedSkill?.content) { return; }
    try {
      await previewDecomposition({
        name: selectedSkill.info.name,
        description: selectedSkill.info.description,
        content: selectedSkill.content,
        source: selectedSkill.info.source,
        repo: selectedSkill.info.name,
      });
      const { preview } = useDecompositionStore.getState();
      if (preview?.workflow_nodes && preview?.workflow_edges) {
        setImportedWorkflowData({
          nodes: preview.workflow_nodes as WorkflowNode[],
          edges: preview.workflow_edges as WorkflowEdge[],
          name: selectedSkill.info.name,
          description: selectedSkill.info.description,
          isDecompositionWorkflow: true,
          decompositionSource: {
            market: selectedSkill.info.source,
            repo: selectedSkill.info.name,
            version: selectedSkill.info.version,
            content: selectedSkill.content,
          },
        });
        openWorkflowEditor();
        setDetailOpen(false);
      }
    } catch (e) {
      console.error("Failed to convert my skill to workflow:", e);
    }
  }, [selectedSkill, previewDecomposition, setImportedWorkflowData, openWorkflowEditor]);

  const handleExtractAtomicSkills = useCallback(async () => {
    if (!selectedSkill?.content) { return; }
    setDecomposeRequest({
      name: selectedSkill.info.name,
      description: selectedSkill.info.description,
      content: selectedSkill.content,
      source: selectedSkill.info.source,
      repo: selectedSkill.info.name,
      version: selectedSkill.info.version,
    });
    setDecomposePreviewOpen(true);
    setDetailOpen(false);
  }, [selectedSkill, setDetailOpen]);

  const handleDecomposeComplete = useCallback(() => {
    setDecomposePreviewOpen(false);
    setDecomposeRequest(null);
    loadSkills();
    messageApi.success(t("skills.decomposeSuccess", "Atomic skills extracted successfully"));
  }, [loadSkills, messageApi, t]);

  const handleUninstall = useCallback(async (name: string) => {
    try {
      await uninstallSkill(name);
      messageApi.success(t("skills.uninstallSuccess", { name }));
    } catch (e) {
      messageApi.error(String(e));
    }
  }, [uninstallSkill, messageApi, t]);

  const handleOpenSkillDir = useCallback(async (path: string) => {
    try {
      await openSkillDir(path);
    } catch (e) {
      messageApi.error(String(e));
    }
  }, [openSkillDir, messageApi]);

  const handleGroupToggle = useCallback((groupSkills: Skill[], enabled: boolean) => {
    for (const skill of groupSkills) {
      toggleSkill(skill.name, enabled);
    }
  }, [toggleSkill]);

  const handleUninstallGroup = useCallback(async (group: string) => {
    try {
      await uninstallSkillGroup(group);
      messageApi.success(t("skills.uninstallSuccess", { name: group }));
    } catch (e) {
      messageApi.error(String(e));
    }
  }, [uninstallSkillGroup, messageApi, t]);

  const filteredSkills = useMemo(() => {
    if (sourceFilter === "all") { return skills; }
    return skills.filter(s => s.source === sourceFilter);
  }, [skills, sourceFilter]);

  const groupedSkills = useMemo(() => {
    const groups = new Map<string, Skill[]>();
    const ungrouped: Skill[] = [];
    for (const skill of filteredSkills) {
      if (skill.group) {
        const arr = groups.get(skill.group) || [];
        arr.push(skill);
        groups.set(skill.group, arr);
      } else {
        ungrouped.push(skill);
      }
    }
    return { groups, ungrouped };
  }, [filteredSkills]);

  const handleOpenGroupDir = useCallback(async (group: string) => {
    // Find the first skill in the group and open its parent's parent dir
    const groupSkills = groupedSkills.groups.get(group);
    if (groupSkills && groupSkills.length > 0) {
      const firstSkillPath = groupSkills[0].sourcePath;
      // sourcePath points to SKILL.md; go up two levels to the group dir
      const parts = firstSkillPath.split("/");
      const groupDir = parts.slice(0, -2).join("/");
      try {
        await openSkillDir(groupDir || firstSkillPath);
      } catch (e) {
        messageApi.error(String(e));
      }
    }
  }, [groupedSkills.groups, openSkillDir, messageApi]);

  const mySkillsContent = (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div style={{ padding: "0 4px", flexShrink: 0 }}>
        <Space.Compact style={{ width: "100%", marginBottom: 8 }}>
          <Input
            placeholder={t("skills.installUrlPlaceholder")}
            value={installUrl}
            onChange={(e) => setInstallUrl(e.target.value)}
            onPressEnter={() => handleInstallFromUrl(sourceFilter === "all" ? "axagent" : sourceFilter)}
          />
          <Dropdown
            menu={{
              items: INSTALL_TARGETS.map((target) => ({
                key: target.key,
                icon: target.icon,
                label: `${target.desc} (${target.label})`,
              })),
              onClick: ({ key }) => handleInstallFromUrl(key),
            }}
            trigger={["click"]}
            disabled={!installUrl.trim()}
          >
            <Button
              type="primary"
              loading={installing === installUrl && !!installUrl}
              disabled={!installUrl.trim()}
            >
              {t("skills.installFromUrl")}
            </Button>
          </Dropdown>
          <Button
            icon={<RefreshCw size={14} color={CHAT_ICON_COLORS.RefreshCw} />}
            onClick={() => loadSkills()}
          />
          <Button
            onClick={() => setCreateModalOpen(true)}
          >
            {t("skill.create", "Create Skill")}
          </Button>
          <Button
            icon={<Lightbulb size={14} color={CHAT_ICON_COLORS.Lightbulb} />}
            onClick={() => setProposalPanelOpen(true)}
            title={t("skill.proposal.title", "Skill Proposals")}
          />
        </Space.Compact>
        <Tabs
          size="small"
          activeKey={sourceFilter}
          onChange={(k) => setSourceFilter(k as any)}
          items={(() => {
            // 统计各来源的技能数量
            const sourceCounts = new Map<string, number>();
            for (const s of skills) {
              sourceCounts.set(s.source, (sourceCounts.get(s.source) ?? 0) + 1);
            }
            // 标准来源 Tab，始终显示（即使 count=0）
            const standardSources = ["axagent", "claude", "agents"];
            const tabs: any[] = [
              {
                key: "all",
                label: (
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                    {ALL_SOURCE_ICON}
                    {t("skills.sourceAll")}
                    <span style={{ color: "var(--color-text-quaternary)", fontSize: 11 }}>
                      ({skills.length})
                    </span>
                  </span>
                ),
              },
            ];
            for (const src of standardSources) {
              const count = sourceCounts.get(src) ?? 0;
              tabs.push({
                key: src,
                label: (
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                    {SOURCE_ICONS[src] ?? <Radio size={14} />}
                    {SOURCE_LABELS[src] ?? src}
                    <span style={{ color: "var(--color-text-quaternary)", fontSize: 11 }}>
                      ({count})
                    </span>
                  </span>
                ),
              });
            }
            // 动态添加其他来源 Tab
            for (const [src, count] of sourceCounts) {
              if (standardSources.includes(src)) { continue; }
              tabs.push({
                key: src,
                label: (
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                    {SOURCE_ICONS[src] ?? <Radio size={14} />}
                    {SOURCE_LABELS[src] ?? src}
                    <span style={{ color: "var(--color-text-quaternary)", fontSize: 11 }}>
                      ({count})
                    </span>
                  </span>
                ),
              });
            }
            return tabs;
          })()}
          style={{ marginBottom: 8 }}
        />
      </div>
      <div style={{ flex: 1, overflow: "auto", padding: "0 4px" }}>
        {sourceFilter !== "all" && (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              padding: "4px 8px",
              marginBottom: 8,
              backgroundColor: token.colorBgContainer,
              borderRadius: 6,
            }}
          >
            <FolderOpen size={14} style={{ color: token.colorTextSecondary, flexShrink: 0 }} />
            <Text type="secondary" style={{ fontSize: 12, flex: 1 }} ellipsis>
              {INSTALL_TARGETS.find(t => t.key === sourceFilter)?.label}
            </Text>
            <Button
              type="text"
              size="small"
              icon={<FolderOpen size={14} />}
              onClick={async () => {
                try {
                  const target = INSTALL_TARGETS.find(t => t.key === sourceFilter);
                  if (!target) { return; }
                  const { homeDir } = await import("@tauri-apps/api/path");
                  const home = await homeDir();
                  const fullPath = target.label.replace("~/", home.endsWith("/") ? home : home + "/");
                  const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
                  await revealItemInDir(fullPath);
                } catch { /* ignore */ }
              }}
            >
              {t("skills.openDir")}
            </Button>
          </div>
        )}
        {loading
          ? (
            <div style={{ textAlign: "center", padding: 48 }}>
              <Spin />
            </div>
          )
          : filteredSkills.length === 0
          ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={
                <div>
                  <div>{t("skills.empty")}</div>
                  <Text type="secondary" style={{ fontSize: 12 }}>{t("skills.emptyDesc")}</Text>
                </div>
              }
            />
          )
          : (
            <>
              {groupedSkills.ungrouped.map((skill) => (
                <SkillCard
                  key={skill.name}
                  skill={skill}
                  onToggle={handleToggle}
                  onDetail={handleDetail}
                  onUninstall={handleUninstall}
                  onOpenDir={handleOpenSkillDir}
                  onEditFrontend={handleEditFrontend}
                  t={t}
                />
              ))}
              {Array.from(groupedSkills.groups.entries()).map(([group, groupSkills]) => {
                const allEnabled = groupSkills.every((s) => s.enabled);
                const someEnabled = groupSkills.some((s) => s.enabled);
                return (
                  <Collapse
                    key={group}
                    defaultActiveKey={[]}
                    style={{ marginTop: 8 }}
                    expandIcon={({ isActive }) => (
                      <ChevronRight
                        size={14}
                        style={{
                          transform: isActive ? "rotate(90deg)" : "rotate(0deg)",
                          transition: "transform 0.2s",
                        }}
                      />
                    )}
                    items={[{
                      key: group,
                      label: (
                        <div style={{ display: "flex", alignItems: "center", gap: 8, flex: 1, lineHeight: 1 }}>
                          <Text strong style={{ lineHeight: "22px" }}>{group}</Text>
                          <Tag style={{ margin: 0 }}>
                            <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                              {SOURCE_ICONS[groupSkills[0]?.source]}
                              {t(`skills.source.${groupSkills[0]?.source}`)}
                            </span>
                          </Tag>
                          <Tag style={{ margin: 0 }}>{t("skills.groupSkillCount", { count: groupSkills.length })}</Tag>
                        </div>
                      ),
                      extra: (
                        <Space size={4} onClick={(e) => e.stopPropagation()}>
                          <Switch
                            size="small"
                            checked={allEnabled}
                            style={someEnabled && !allEnabled ? { backgroundColor: "#faad14" } : undefined}
                            onChange={(checked) => {
                              handleGroupToggle(groupSkills, checked);
                            }}
                          />
                          <Button
                            type="text"
                            size="small"
                            icon={<FolderOpen size={14} />}
                            onClick={(e) => {
                              e.stopPropagation();
                              handleOpenGroupDir(group);
                            }}
                          />
                          <Popconfirm
                            title={t("skills.uninstallGroupConfirm", { name: group })}
                            onConfirm={() => handleUninstallGroup(group)}
                            okText={t("skills.uninstall")}
                            cancelText={t("common.cancel")}
                          >
                            <Button
                              type="text"
                              size="small"
                              danger
                              icon={<Trash2 size={14} />}
                              onClick={(e) => e.stopPropagation()}
                            />
                          </Popconfirm>
                        </Space>
                      ),
                      children: (
                        <div style={{ padding: "4px 0" }}>
                          {groupSkills.map((skill) => (
                            <SkillCard
                              key={skill.name}
                              skill={skill}
                              onToggle={handleToggle}
                              onDetail={handleDetail}
                              onUninstall={handleUninstall}
                              onOpenDir={handleOpenSkillDir}
                              onEditFrontend={handleEditFrontend}
                              t={t}
                            />
                          ))}
                        </div>
                      ),
                    }]}
                  />
                );
              })}
            </>
          )}
      </div>
    </div>
  );

  const marketplaceContent = (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div style={{ padding: "0 4px", flexShrink: 0 }}>
        <Space.Compact style={{ width: "100%", marginBottom: 12 }}>
          <Select
            value={marketplaceSource}
            onChange={(v) => setMarketplaceSource(v)}
            style={{ width: 120, flexShrink: 0 }}
            options={[
              { value: "skillhub", label: "skillhub" },
              { value: "github", label: "GitHub" },
            ]}
          />
          <Select
            value={sortOrder}
            onChange={(v) => setSortOrder(v)}
            style={{ width: 100, flexShrink: 0 }}
            options={[
              { value: "popular", label: t("skills.sortPopular") || "Popular" },
              { value: "latest", label: t("skills.sortLatest") || "Latest" },
              { value: "stars", label: t("skills.sortStars") || "Stars" },
            ]}
          />
          <Input.Search
            placeholder={t("skills.searchMarketplace")}
            loading={marketplaceLoading}
            onSearch={(q) => {
              setMarketplaceQuery(q);
              marketplaceLoaded.current = true;
              searchMarketplace(q, marketplaceSource, sortOrder);
            }}
            enterButton
          />
        </Space.Compact>
      </div>
      <div style={{ flex: 1, overflow: "auto", padding: "0 4px" }}>
        {marketplaceLoading
          ? (
            <div style={{ textAlign: "center", padding: 48 }}>
              <Spin />
            </div>
          )
          : marketplaceSkills.length === 0
          ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={t("skills.noResults")}
            />
          )
          : (
            <>
              {marketplaceSkills.map((skill) => (
                <MarketplaceCard
                  key={skill.repo}
                  skill={skill}
                  onInstall={handleInstallFromMarketplace}
                  onDetail={handleMarketplaceDetail}
                  onExtract={handleMarketplaceExtract}
                  onConvert={handleMarketplaceConvert}
                  installing={installing}
                  t={t}
                  source={marketplaceSource}
                />
              ))}
              {marketplaceHasMore && (
                <div style={{ textAlign: "center", padding: "16px 0" }}>
                  <Button onClick={loadMoreMarketplace} loading={marketplaceLoading}>
                    {t("skills.loadMore")}
                  </Button>
                </div>
              )}
            </>
          )}
      </div>
    </div>
  );

  return (
    <>
      {contextHolder}
      <div className="h-full flex flex-col" style={{ overflow: "hidden", backgroundColor: token.colorBgElevated }}>
        <Tabs
          className="skills-page-tabs"
          defaultActiveKey="my"
          onChange={handleTabChange}
          style={{ flex: 1, display: "flex", flexDirection: "column", minHeight: 0 }}
          tabBarStyle={{ padding: "0 16px", flexShrink: 0 }}
          items={[
            {
              key: "my",
              label: (
                <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                  <Sparkles size={14} color={CHAT_ICON_COLORS.Sparkles} />
                  {t("skills.mySkills")}
                </span>
              ),
              children: mySkillsContent,
            },
            {
              key: "atomic",
              label: (
                <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                  ⚛️{t("skills.atomicSkills", "原子Skill")}
                </span>
              ),
              children: (
                <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
                  <AtomicSkillList
                    onEdit={(skill) => {
                      setEditingAtomicSkill(skill);
                      setAtomicSkillEditVisible(true);
                    }}
                    onCreate={() => {
                      setEditingAtomicSkill(null);
                      setAtomicSkillEditVisible(true);
                    }}
                  />
                </div>
              ),
            },
            {
              key: "marketplace",
              label: (
                <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                  <Store size={14} color={CHAT_ICON_COLORS.Cloud} />
                  {t("skills.marketplace.title")}
                </span>
              ),
              children: marketplaceContent,
            },
          ]}
        />

        <style>
          {`
          .skills-page-tabs > .ant-tabs-content-holder {
            flex: 1;
            min-height: 0;
            display: flex;
            flex-direction: column;
            overflow: hidden;
            padding: 0 12px;
          }
          .skills-page-tabs > .ant-tabs-content-holder > .ant-tabs-content {
            flex: 1;
            min-height: 0;
          }
          .skills-page-tabs > .ant-tabs-content-holder > .ant-tabs-content > .ant-tabs-tabpane-active {
            height: 100%;
            display: flex;
            flex-direction: column;
          }
          .skill-card-hover {
            transition: border-color 0.2s;
          }
          .skill-card-hover:hover {
            border-color: ${token.colorPrimary} !important;
          }
          .skill-card-hover:hover .skill-card-title {
            color: ${token.colorPrimary} !important;
          }
        `}
        </style>
      </div>

      <Modal
        title={t("skills.detail")}
        open={detailOpen}
        onCancel={() => {
          setDetailOpen(false);
          clearSelectedSkill();
        }}
        footer={selectedSkill && selectedSkill.content && !selectedSkill.content.startsWith("(")
          ? (
            <Space>
              <Button
                icon={<Layers size={14} />}
                onClick={handleExtractAtomicSkills}
              >
                {t("skills.extractAtomicSkills", "Extract Atomic Skills")}
              </Button>
              <Button
                icon={<Workflow size={14} />}
                onClick={handleConvertMySkillToWorkflow}
              >
                {t("skills.marketplace.convertToWorkflow")}
              </Button>
            </Space>
          )
          : null}
        width={640}
      >
        {selectedSkill && (
          <div style={{ userSelect: "text" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
              <Typography.Title level={4} style={{ margin: 0 }}>{selectedSkill.info.name}</Typography.Title>
              <CopyButton text={selectedSkill.info.name} size={14} />
              <Tag>
                <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                  {SOURCE_ICONS[selectedSkill.info.source]}
                  {t(`skills.source.${selectedSkill.info.source}`)}
                </span>
              </Tag>
            </div>
            <Paragraph type="secondary">{selectedSkill.info.description}</Paragraph>
            {selectedSkill.manifest && (
              <div style={{ marginBottom: 12 }}>
                {selectedSkill.manifest.sourceRef && (
                  <Text type="secondary" style={{ fontSize: 12, display: "block" }}>
                    Source: {selectedSkill.manifest.sourceRef}
                  </Text>
                )}
                <Text type="secondary" style={{ fontSize: 12, display: "block" }}>
                  Installed: {selectedSkill.manifest.installedAt}
                </Text>
              </div>
            )}
            <div style={{ position: "relative" }}>
              <CopyButton
                text={selectedSkill.content}
                size={14}
                style={{ position: "absolute", top: 8, right: 8, zIndex: 1 }}
              />
              <div
                style={{
                  background: token.colorBgContainer,
                  borderRadius: token.borderRadius,
                  padding: 16,
                  whiteSpace: "pre-wrap",
                  fontFamily: "monospace",
                  fontSize: 13,
                  maxHeight: 400,
                  overflow: "auto",
                  userSelect: "text",
                }}
              >
                {selectedSkill.content}
              </div>
            </div>
            {selectedSkill.files.length > 0 && (
              <div style={{ marginTop: 12 }}>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  Files: {selectedSkill.files.join(", ")}
                </Text>
              </div>
            )}
          </div>
        )}
      </Modal>

      <Modal
        title={t("skills.detail")}
        open={marketplaceDetailOpen}
        onCancel={() => {
          setMarketplaceDetailOpen(false);
          setMarketplaceDetailContent(null);
        }}
        footer={marketplaceDetailContent && !marketplaceDetailContent.content.startsWith("(")
          ? (
            <Space>
              <Button
                icon={<Layers size={14} />}
                onClick={handleMarketplaceExtractAtomicSkills}
              >
                {t("skills.extractAtomicSkills", "Extract Atomic Skills")}
              </Button>
              <Button
                icon={<Workflow size={14} />}
                onClick={() => handleConvertToWorkflow(marketplaceDetailContent.repo)}
              >
                {t("skills.marketplace.convertToWorkflow")}
              </Button>
            </Space>
          )
          : null}
        width={640}
      >
        {marketplaceDetailContent && (
          <div style={{ userSelect: "text" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
              <Typography.Title level={4} style={{ margin: 0 }}>{marketplaceDetailContent.name}</Typography.Title>
              <CopyButton text={marketplaceDetailContent.name} size={14} />
            </div>
            <Text type="secondary" style={{ fontSize: 12, display: "block", marginBottom: 12 }}>
              {marketplaceDetailContent.repo}
            </Text>
            {marketplaceDetailLoading
              ? (
                <div style={{ textAlign: "center", padding: 32 }}>
                  <Spin />
                </div>
              )
              : (
                <div style={{ position: "relative" }}>
                  <CopyButton
                    text={marketplaceDetailContent.content}
                    size={14}
                    style={{ position: "absolute", top: 8, right: 8, zIndex: 1 }}
                  />
                  <div
                    style={{
                      background: token.colorBgContainer,
                      borderRadius: token.borderRadius,
                      padding: 16,
                      whiteSpace: "pre-wrap",
                      fontFamily: "monospace",
                      fontSize: 13,
                      maxHeight: 400,
                      overflow: "auto",
                      userSelect: "text",
                    }}
                  >
                    {marketplaceDetailContent.content}
                  </div>
                </div>
              )}
          </div>
        )}
      </Modal>

      <SkillCreateModal
        open={createModalOpen}
        onClose={() => setCreateModalOpen(false)}
      />

      <SkillProposalPanel
        open={proposalPanelOpen}
        onClose={() => setProposalPanelOpen(false)}
      />

      <AtomicSkillEditor
        visible={atomicSkillEditVisible}
        skill={editingAtomicSkill}
        onClose={() => {
          setAtomicSkillEditVisible(false);
          setEditingAtomicSkill(null);
        }}
      />

      {decomposeRequest && (
        <DecompositionPreview
          visible={decomposePreviewOpen}
          request={decomposeRequest}
          onClose={() => {
            setDecomposePreviewOpen(false);
            setDecomposeRequest(null);
          }}
          onComplete={handleDecomposeComplete}
        />
      )}

      <FrontendEditorModal
        open={!!editingFrontendSkill}
        skillName={editingFrontendSkill?.name || ""}
        currentFrontend={editingFrontendSkill?.frontend}
        onClose={() => setEditingFrontendSkill(null)}
        onSaved={() => {
          setEditingFrontendSkill(null);
          loadSkills();
        }}
      />
    </>
  );
}
