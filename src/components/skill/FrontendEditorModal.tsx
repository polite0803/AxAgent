import { invoke } from "@/lib/invoke";
import { useSkillExtensionStore } from "@/stores";
import type {
  SkillChatCommand,
  SkillCommandAction,
  SkillFrontendExtension,
  SkillNavItem,
  SkillPage,
  SkillSettingsSection,
  SkillStatusBarItem,
  SkillToolbarButton,
  SkillUICommand,
  SkillUIPanel,
} from "@/types";
import {
  Button,
  Collapse,
  Empty,
  Form,
  Input,
  InputNumber,
  message,
  Modal,
  Popconfirm,
  Select,
  Space,
  Switch,
  Tabs,
  Typography,
} from "antd";
import {
  Copy,
  Edit3,
  Eye,
  FileCode,
  LayoutPanelTop,
  Lightbulb,
  MessageSquare,
  PanelBottom,
  Plus,
  Puzzle,
  Route,
  Settings,
  Trash2,
  Zap,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { ActionChainEditor } from "./ActionChainEditor";

const { Text } = Typography;

interface FrontendEditorModalProps {
  open: boolean;
  skillName: string;
  currentFrontend?: SkillFrontendExtension;
  onClose: () => void;
  onSaved: () => void;
}

type EditorTab = "visual" | "json" | "preview" | "manifest";
type SectionTab =
  | "navigation"
  | "pages"
  | "commands"
  | "panels"
  | "settingsSections"
  | "toolbar"
  | "chatCommand"
  | "statusBar";

const COMPONENT_TYPE_OPTIONS = [
  { value: "Html", label: "HTML" },
  { value: "Iframe", label: "Iframe" },
  { value: "Markdown", label: "Markdown" },
  { value: "React", label: "React (内置/捆绑)" },
  { value: "WebComponent", label: "Web Component" },
];

const PANEL_POSITION_OPTIONS = [
  { value: "Main", label: "主区域" },
  { value: "Sidebar", label: "侧边栏" },
  { value: "Header", label: "顶部" },
  { value: "Footer", label: "底部" },
];

const PANEL_SIZE_OPTIONS = [
  { value: "Small", label: "小" },
  { value: "Medium", label: "中" },
  { value: "Large", label: "大" },
  { value: "FullWidth", label: "全宽" },
];

const EMPTY_EXTENSION: SkillFrontendExtension = {
  navigation: [],
  pages: [],
  commands: [],
  panels: [],
  settingsSections: [],
  toolbar: [],
  chatCommand: [],
  statusBar: [],
};

function formatJson(obj: unknown): string {
  return JSON.stringify(obj, null, 2);
}

export function FrontendEditorModal({ open, skillName, currentFrontend, onClose, onSaved }: FrontendEditorModalProps) {
  const [editorTab, setEditorTab] = useState<EditorTab>("visual");
  const [sectionTab, setSectionTab] = useState<SectionTab>("navigation");

  // 工作副本
  const [data, setData] = useState<SkillFrontendExtension>(
    currentFrontend ? structuredClone(currentFrontend) : structuredClone(EMPTY_EXTENSION),
  );
  // JSON 文本
  const [jsonText, setJsonText] = useState(formatJson(data));
  const [jsonError, setJsonError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [analyzing, setAnalyzing] = useState(false);

  const setSkillFrontend = useSkillExtensionStore((s) => s.setSkillFrontend);

  const handleAnalyze = useCallback(async () => {
    setAnalyzing(true);
    try {
      const result = await invoke<SkillFrontendExtension>("skill_analyze_frontend", { name: skillName });
      setData(result);
      setJsonText(formatJson(result));
      setEditorTab("visual");
      message.success("智能分析完成，已生成前端扩展配置");
    } catch (e) {
      message.error(`智能分析失败: ${String(e)}`);
    } finally {
      setAnalyzing(false);
    }
  }, [skillName]);

  // 当 Modal 关闭且仍在分析时取消 loading 状态
  useEffect(() => {
    if (!open) {
      setAnalyzing(false);
    }
  }, [open]);

  useEffect(() => {
    if (open) {
      const d = currentFrontend ? structuredClone(currentFrontend) : structuredClone(EMPTY_EXTENSION);
      setData(d);
      setJsonText(formatJson(d));
      setJsonError(null);
      setEditorTab("visual");
    }
  }, [open, currentFrontend]);

  const handleSave = useCallback(async () => {
    try {
      setSaving(true);
      let finalData: SkillFrontendExtension;
      if (editorTab === "json") {
        finalData = JSON.parse(jsonText);
      } else {
        finalData = data;
      }
      await setSkillFrontend(skillName, finalData);
      message.success("前端扩展配置已保存");
      onClose();
      onSaved();
    } catch (e) {
      message.error(`保存失败: ${String(e)}`);
      setSaving(false);
    }
  }, [editorTab, jsonText, data, skillName, setSkillFrontend, onClose, onSaved]);

  const handleJsonChange = useCallback((value: string) => {
    setJsonText(value);
    try {
      JSON.parse(value);
      setJsonError(null);
    } catch (e) {
      setJsonError(String(e));
    }
  }, []);

  const syncVisualToJson = useCallback(() => {
    setJsonText(formatJson(data));
    setEditorTab("json");
  }, [data]);

  const syncJsonToVisual = useCallback(() => {
    try {
      const parsed = JSON.parse(jsonText);
      setData(parsed);
      setEditorTab("visual");
      setJsonError(null);
    } catch {
      message.error("JSON 格式错误，无法切换到可视化编辑");
    }
  }, [jsonText]);

  // ─── 列表编辑辅助 ───
  const addItem = useCallback((key: SectionTab, factory: () => unknown) => {
    setData((prev) => {
      const items = prev[key] as unknown[];
      return { ...prev, [key]: [...items, factory()] };
    });
  }, []);

  const removeItem = useCallback((key: SectionTab, index: number) => {
    setData((prev) => {
      const items = prev[key] as unknown[];
      return { ...prev, [key]: items.filter((_, i) => i !== index) };
    });
  }, []);

  const editItem = useCallback((key: SectionTab, index: number, item: unknown) => {
    setData((prev) => {
      const items = prev[key] as unknown[];
      return { ...prev, [key]: items.map((it, i) => (i === index ? item : it)) };
    });
  }, []);

  // ─── 各类型的工厂函数 ───
  const newNavItem = (): SkillNavItem => ({
    id: `nav-${Date.now()}`,
    label: "",
    icon: "lucide:Puzzle",
    pageId: "",
    position: 1,
  });

  const newPage = (): SkillPage => ({
    id: `page-${Date.now()}`,
    title: "",
    componentType: "Html",
    componentConfig: { file: "index.html" },
  });

  const newCommand = (): SkillUICommand => ({
    id: `cmd-${Date.now()}`,
    label: "",
    category: skillName,
    actions: [],
  });

  const newPanel = (): SkillUIPanel => ({
    id: `panel-${Date.now()}`,
    title: "",
    componentType: "Markdown",
    componentConfig: { file: "status.md" },
    position: "Sidebar",
    size: "Medium",
    collapsible: true,
    defaultCollapsed: false,
  });

  const newSettingsSection = (): SkillSettingsSection => ({
    id: `settings-${Date.now()}`,
    title: "",
    settingsGroup: "extensions",
    componentType: "Html",
    componentConfig: { file: "settings.html" },
  });

  const newToolbarButton = (): SkillToolbarButton => ({
    id: `toolbar-${Date.now()}`,
    icon: "lucide:Paperclip",
    tooltip: "",
    position: "left",
    priority: 0,
    onClick: [],
  });

  const newChatCommand = (): SkillChatCommand => ({
    name: "",
    description: "",
    mode: "agentic",
    promptTemplate: "",
  });

  const newStatusBarItem = (): SkillStatusBarItem => ({
    id: `status-${Date.now()}`,
    alignment: "left",
    priority: 0,
    text: "",
  });

  // ─── 计数标签 ───
  const countBadge = (n: number) => (n > 0 ? ` (${n})` : "");

  const sectionItems = useMemo((): { key: SectionTab; icon: React.ReactNode; label: string }[] => [
    { key: "navigation", icon: <Route size={14} />, label: `导航${countBadge(data.navigation.length)}` },
    { key: "pages", icon: <FileCode size={14} />, label: `页面${countBadge(data.pages.length)}` },
    { key: "commands", icon: <Zap size={14} />, label: `命令${countBadge(data.commands.length)}` },
    { key: "panels", icon: <LayoutPanelTop size={14} />, label: `面板${countBadge(data.panels.length)}` },
    { key: "settingsSections", icon: <Settings size={14} />, label: `设置${countBadge(data.settingsSections.length)}` },
    { key: "toolbar", icon: <Puzzle size={14} />, label: `工具栏${countBadge(data.toolbar.length)}` },
    { key: "chatCommand", icon: <MessageSquare size={14} />, label: `聊天命令${countBadge(data.chatCommand.length)}` },
    { key: "statusBar", icon: <PanelBottom size={14} />, label: `状态栏${countBadge(data.statusBar.length)}` },
  ], [data]);

  const visualEditor = (
    <div style={{ display: "flex", height: 420 }}>
      {/* 左侧：分区列表 */}
      <div style={{ width: 140, borderRight: "1px solid var(--border-color)", flexShrink: 0 }}>
        {sectionItems.map((item) => (
          <div
            key={item.key}
            onClick={() => setSectionTab(item.key)}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              padding: "8px 12px",
              cursor: "pointer",
              fontSize: 13,
              borderRadius: 0,
              backgroundColor: sectionTab === item.key ? "var(--color-primary-bg)" : "transparent",
              color: sectionTab === item.key ? "var(--color-primary)" : "var(--color-text)",
              fontWeight: sectionTab === item.key ? 500 : 400,
            }}
          >
            {item.icon}
            <span>{item.label}</span>
          </div>
        ))}
      </div>
      {/* 右侧：编辑区 */}
      <div style={{ flex: 1, overflow: "auto", padding: "8px 12px" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
          <Text strong style={{ fontSize: 13 }}>
            {sectionItems.find((s) => s.key === sectionTab)?.label}
          </Text>
          <Button
            type="dashed"
            size="small"
            icon={<Plus size={12} />}
            onClick={() => {
              const factories: Record<SectionTab, () => unknown> = {
                navigation: newNavItem,
                pages: newPage,
                commands: newCommand,
                panels: newPanel,
                settingsSections: newSettingsSection,
                toolbar: newToolbarButton,
                chatCommand: newChatCommand,
                statusBar: newStatusBarItem,
              };
              addItem(sectionTab, factories[sectionTab] as () => never);
            }}
          >
            添加
          </Button>
        </div>

        {(() => {
          const items = data[sectionTab];
          if (items.length === 0) {
            return <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description='暂无数据，点击"添加" 创建' />;
          }

          return (
            <Collapse
              size="small"
              items={(items as unknown as Record<string, unknown>[]).map((
                item: Record<string, unknown>,
                index: number,
              ) => ({
                key: String(index),
                label: getItemLabel(sectionTab, item),
                extra: (
                  <Popconfirm
                    title="确定删除？"
                    onConfirm={() => removeItem(sectionTab, index)}
                    okText="删除"
                    cancelText="取消"
                  >
                    <Button type="text" size="small" danger icon={<Trash2 size={12} />} />
                  </Popconfirm>
                ),
                children: renderItemEditor(sectionTab, item, (updated) => editItem(sectionTab, index, updated)),
              }))}
            />
          );
        })()}
      </div>
    </div>
  );

  const jsonEditor = (
    <div>
      <div style={{ marginBottom: 8 }}>
        <Space>
          <Button size="small" onClick={syncVisualToJson} icon={<Edit3 size={12} />}>
            从可视化生成
          </Button>
          <Button size="small" onClick={syncJsonToVisual} icon={<Eye size={12} />} disabled={!!jsonError}>
            切换到可视化
          </Button>
        </Space>
      </div>
      <Input.TextArea
        value={jsonText}
        onChange={(e) => handleJsonChange(e.target.value)}
        rows={22}
        style={{ fontFamily: "monospace", fontSize: 12 }}
      />
      {jsonError && (
        <Text type="danger" style={{ fontSize: 12, marginTop: 4, display: "block" }}>
          JSON 错误: {jsonError}
        </Text>
      )}
    </div>
  );

  const previewTab = (
    <div>
      <Button
        size="small"
        icon={<Copy size={12} />}
        onClick={() => {
          navigator.clipboard.writeText(formatJson(data));
          message.success("已复制到剪贴板");
        }}
        style={{ marginBottom: 8 }}
      >
        复制
      </Button>
      <pre
        style={{
          fontSize: 12,
          fontFamily: "monospace",
          backgroundColor: "var(--color-fill-alter)",
          padding: 12,
          borderRadius: 6,
          overflow: "auto",
          maxHeight: 420,
        }}
      >
        {formatJson(data)}
      </pre>
    </div>
  );

  return (
    <Modal
      title={`编辑前端扩展 — ${skillName}`}
      open={open}
      onCancel={onClose}
      onOk={handleSave}
      confirmLoading={saving}
      width={720}
      okText="保存"
      cancelText="取消"
      destroyOnClose
    >
      <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: 4 }}>
        <Button
          type="dashed"
          size="small"
          icon={analyzing ? undefined : <Lightbulb size={14} />}
          loading={analyzing}
          onClick={handleAnalyze}
        >
          {analyzing ? "分析中..." : "智能分析（AI 自动生成）"}
        </Button>
      </div>
      <Tabs
        activeKey={editorTab}
        onChange={(k) => setEditorTab(k as EditorTab)}
        size="small"
        style={{ marginBottom: 0 }}
        items={[
          {
            key: "visual",
            label: (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                <Puzzle size={12} />可视化编辑
              </span>
            ),
            children: visualEditor,
          },
          {
            key: "json",
            label: (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                <FileCode size={12} />JSON 编辑
              </span>
            ),
            children: jsonEditor,
          },
          {
            key: "preview",
            label: (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                <Eye size={12} />预览
              </span>
            ),
            children: previewTab,
          },
        ]}
      />
    </Modal>
  );
}

// ─── 辅助函数 ───

function getItemLabel(section: SectionTab, item: Record<string, unknown>): string {
  const id = String(item.id || "");
  switch (section) {
    case "navigation":
      return (item.label as string) || id;
    case "pages":
      return (item.title as string) || id;
    case "commands":
      return (item.label as string) || id;
    case "panels":
      return (item.title as string) || id;
    case "settingsSections":
      return (item.title as string) || id;
    case "toolbar":
      return `${item.tooltip || id}`;
    case "chatCommand":
      return `/${item.name || id}`;
    case "statusBar":
      return (item.text as string) || id;
  }
}

function renderItemEditor(
  section: SectionTab,
  item: Record<string, unknown>,
  onChange: (updated: Record<string, unknown>) => void,
) {
  const field = (key: string, value: unknown) => ({ ...item, [key]: value });

  switch (section) {
    case "navigation":
      return (
        <Form layout="vertical" size="small">
          <Form.Item label="ID">
            <Input value={item.id as string} onChange={(e) => onChange(field("id", e.target.value))} />
          </Form.Item>
          <Form.Item label="标签">
            <Input value={item.label as string} onChange={(e) => onChange(field("label", e.target.value))} />
          </Form.Item>
          <Form.Item label="图标">
            <Input
              value={item.icon as string}
              onChange={(e) => onChange(field("icon", e.target.value))}
              placeholder="lucide:Puzzle"
            />
          </Form.Item>
          <Form.Item label="页面 ID">
            <Input value={item.pageId as string} onChange={(e) => onChange(field("pageId", e.target.value))} />
          </Form.Item>
          <Form.Item label="排序">
            <InputNumber
              value={item.position as number}
              onChange={(v) => onChange(field("position", v ?? 0))}
              style={{ width: 100 }}
            />
          </Form.Item>
        </Form>
      );

    case "pages":
      return (
        <Form layout="vertical" size="small">
          <Form.Item label="ID">
            <Input value={item.id as string} onChange={(e) => onChange(field("id", e.target.value))} />
          </Form.Item>
          <Form.Item label="标题">
            <Input value={item.title as string} onChange={(e) => onChange(field("title", e.target.value))} />
          </Form.Item>
          <Form.Item label="组件类型">
            <Select
              value={item.componentType as string}
              onChange={(v) => onChange(field("componentType", v))}
              options={COMPONENT_TYPE_OPTIONS}
            />
          </Form.Item>
          <Form.Item label="配置文件">
            <Input
              value={(item.componentConfig as Record<string, unknown>)?.file as string || ""}
              onChange={(e) => onChange(field("componentConfig", { file: e.target.value }))}
              placeholder="index.html"
            />
          </Form.Item>
        </Form>
      );

    case "commands":
      return (
        <Form layout="vertical" size="small">
          <Form.Item label="ID">
            <Input value={item.id as string} onChange={(e) => onChange(field("id", e.target.value))} />
          </Form.Item>
          <Form.Item label="标签">
            <Input value={item.label as string} onChange={(e) => onChange(field("label", e.target.value))} />
          </Form.Item>
          <Form.Item label="分类">
            <Input value={item.category as string} onChange={(e) => onChange(field("category", e.target.value))} />
          </Form.Item>
          <Form.Item label="图标">
            <Input
              value={item.icon as string || ""}
              onChange={(e) => onChange(field("icon", e.target.value))}
              placeholder="lucide:Play"
            />
          </Form.Item>
          <Form.Item label="Actions (JSON)">
            <Input.TextArea
              rows={4}
              value={JSON.stringify(item.actions, null, 2)}
              onChange={(e) => {
                try {
                  onChange(field("actions", JSON.parse(e.target.value)));
                } catch { /* ignore */ }
              }}
              style={{ fontFamily: "monospace", fontSize: 11 }}
            />
          </Form.Item>
        </Form>
      );

    case "panels":
      return (
        <Form layout="vertical" size="small">
          <Form.Item label="ID">
            <Input value={item.id as string} onChange={(e) => onChange(field("id", e.target.value))} />
          </Form.Item>
          <Form.Item label="标题">
            <Input value={item.title as string} onChange={(e) => onChange(field("title", e.target.value))} />
          </Form.Item>
          <Form.Item label="组件类型">
            <Select
              value={item.componentType as string}
              onChange={(v) => onChange(field("componentType", v))}
              options={COMPONENT_TYPE_OPTIONS}
            />
          </Form.Item>
          <Form.Item label="配置文件">
            <Input
              value={(item.componentConfig as Record<string, unknown>)?.file as string || ""}
              onChange={(e) => onChange(field("componentConfig", { file: e.target.value }))}
            />
          </Form.Item>
          <Form.Item label="位置">
            <Select
              value={item.position as string}
              onChange={(v) => onChange(field("position", v))}
              options={PANEL_POSITION_OPTIONS}
            />
          </Form.Item>
          <Form.Item label="大小">
            <Select
              value={item.size as string}
              onChange={(v) => onChange(field("size", v))}
              options={PANEL_SIZE_OPTIONS}
            />
          </Form.Item>
        </Form>
      );

    case "settingsSections":
      return (
        <Form layout="vertical" size="small">
          <Form.Item label="ID">
            <Input value={item.id as string} onChange={(e) => onChange(field("id", e.target.value))} />
          </Form.Item>
          <Form.Item label="标题">
            <Input value={item.title as string} onChange={(e) => onChange(field("title", e.target.value))} />
          </Form.Item>
          <Form.Item label="图标">
            <Input
              value={item.icon as string || ""}
              onChange={(e) => onChange(field("icon", e.target.value))}
              placeholder="lucide:Settings"
            />
          </Form.Item>
          <Form.Item label="设置组">
            <Select
              value={item.settingsGroup as string || "extensions"}
              onChange={(v) => onChange(field("settingsGroup", v))}
              options={[
                { value: "extensions", label: "扩展" },
                { value: "appearance", label: "外观" },
                { value: "network", label: "网络" },
                { value: "data", label: "数据" },
                { value: "system", label: "系统" },
              ]}
            />
          </Form.Item>
          <Form.Item label="组件类型">
            <Select
              value={item.componentType as string}
              onChange={(v) => onChange(field("componentType", v))}
              options={COMPONENT_TYPE_OPTIONS}
            />
          </Form.Item>
          <Form.Item label="配置文件">
            <Input
              value={(item.componentConfig as Record<string, unknown>)?.file as string || ""}
              onChange={(e) => onChange(field("componentConfig", { file: e.target.value }))}
            />
          </Form.Item>
        </Form>
      );

    case "toolbar":
      return (
        <Form layout="vertical" size="small">
          <Form.Item label="ID">
            <Input value={item.id as string} onChange={(e) => onChange(field("id", e.target.value))} />
          </Form.Item>
          <Form.Item label="图标">
            <Input
              value={item.icon as string}
              onChange={(e) => onChange(field("icon", e.target.value))}
              placeholder="lucide:Paperclip"
            />
          </Form.Item>
          <Form.Item label="提示文本">
            <Input value={item.tooltip as string} onChange={(e) => onChange(field("tooltip", e.target.value))} />
          </Form.Item>
          <Form.Item label="位置">
            <Select
              value={item.position as string}
              onChange={(v) => onChange(field("position", v))}
              options={[{ value: "left", label: "左侧" }, { value: "right", label: "右侧" }]}
              style={{ width: 120 }}
            />
          </Form.Item>
          <Form.Item label="优先级">
            <InputNumber
              value={item.priority as number}
              onChange={(v) => onChange(field("priority", v ?? 0))}
              style={{ width: 80 }}
            />
          </Form.Item>
          <Form.Item label="点击 Actions">
            <ActionChainEditor
              actions={(item.onClick as SkillCommandAction[]) || []}
              availableHandlers={[]}
              onChange={(actions) => onChange(field("onClick", actions))}
            />
          </Form.Item>
        </Form>
      );

    case "chatCommand":
      return (
        <Form layout="vertical" size="small">
          <Form.Item label="命令名 (/xxx)">
            <Input
              value={item.name as string}
              onChange={(e) => onChange(field("name", e.target.value))}
              placeholder="review"
            />
          </Form.Item>
          <Form.Item label="描述">
            <Input
              value={item.description as string}
              onChange={(e) => onChange(field("description", e.target.value))}
            />
          </Form.Item>
          <Form.Item label="图标">
            <Input
              value={item.icon as string || ""}
              onChange={(e) => onChange(field("icon", e.target.value))}
              placeholder="lucide:Search"
            />
          </Form.Item>
          <Form.Item label="执行模式">
            <Select
              value={item.mode as string}
              onChange={(v) => onChange(field("mode", v))}
              options={[{ value: "declarative", label: "声明式" }, { value: "agentic", label: "Agent 智能" }]}
              style={{ width: 140 }}
            />
          </Form.Item>
          {(item.mode as string) === "declarative" && (
            <Form.Item label="Actions">
              <ActionChainEditor
                actions={(item.actions as SkillCommandAction[]) || []}
                availableHandlers={[]}
                onChange={(actions) => onChange(field("actions", actions))}
              />
            </Form.Item>
          )}
          {(item.mode as string) === "agentic" && (
            <>
              <Form.Item label="Prompt 模板">
                <Input.TextArea
                  size="small"
                  rows={3}
                  value={item.promptTemplate as string || ""}
                  onChange={(e) => onChange(field("promptTemplate", e.target.value))}
                  placeholder="可用变量: {{input}} {{conversation}} {{files}}"
                />
              </Form.Item>
              <Form.Item label="附加上下文">
                <span style={{ fontSize: 12 }}>
                  <Switch size="small" /> 包含对话
                </span>
              </Form.Item>
            </>
          )}
        </Form>
      );

    case "statusBar":
      return (
        <Form layout="vertical" size="small">
          <Form.Item label="ID">
            <Input value={item.id as string} onChange={(e) => onChange(field("id", e.target.value))} />
          </Form.Item>
          <Form.Item label="对齐">
            <Select
              value={item.alignment as string}
              onChange={(v) => onChange(field("alignment", v))}
              options={[{ value: "left", label: "左侧" }, { value: "right", label: "右侧" }]}
              style={{ width: 120 }}
            />
          </Form.Item>
          <Form.Item label="优先级">
            <InputNumber
              value={item.priority as number}
              onChange={(v) => onChange(field("priority", v ?? 0))}
              style={{ width: 80 }}
            />
          </Form.Item>
          <Form.Item label="图标">
            <Input value={item.icon as string || ""} onChange={(e) => onChange(field("icon", e.target.value))} />
          </Form.Item>
          <Form.Item label="文本">
            <Input value={item.text as string || ""} onChange={(e) => onChange(field("text", e.target.value))} />
          </Form.Item>
        </Form>
      );
  }
}
