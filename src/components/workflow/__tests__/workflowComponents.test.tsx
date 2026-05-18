import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();

vi.mock("@/lib/invoke", () => ({
  invoke: invokeMock,
  listen: () => vi.fn(),
  isTauri: () => false,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: string | Record<string, unknown>) => {
      const translations: Record<string, string> = {
        "workflow.aiPanel.enterWorkflowDesc": "请输入工作流描述",
        "workflow.aiPanel.enterPromptToOptimize": "请输入要优化的 Prompt",
        "workflow.aiPanel.enterContext": "请输入上下文描述",
        "workflow.aiPanel.enterAgentPrompt": "输入 Agent Prompt",
        "workflow.aiPanel.describeContext": "描述上下文",
        "workflow.aiPanel.describeWorkflow": "描述工作流",
        "workflow.aiPanel.generateBtn": "生成工作流",
        "workflow.aiPanel.generatePlaceholder": "创建一个代码审查工作流",
        "workflow.aiPanel.aiAssistant": "AI 助手",
        "workflow.aiPanel.currentCanvasState": "当前画布状态",
        "workflow.aiPanel.replaceCanvasWarning": "生成新工作流将替换当前画布上的所有内容",
        "workflow.aiPanel.tabGenerateWorkflow": "生成工作流",
        "workflow.aiPanel.tabOptimizePrompt": "优化 Prompt",
        "workflow.aiPanel.tabRecommend": "推荐节点",
        "workflow.aiPanel.canvasStatus": "节点: 0, 连线: 0",
        "workflow.aiPanel.getRecommendation": "获取推荐",
        "workflow.aiPanel.noRecommendations": "暂无推荐节点",
        "workflow.aiPanel.dragHint": "拖拽节点到画布上",
        "workflow.templateList.noTemplates": "暂无模板",
        "workflow.templateList.noMatchFound": "未找到匹配的模板",
        "workflow.templateList.searchPlaceholder": "搜索模板",
        "workflow.templateList.tagPlaceholder": "标签筛选",
        "workflow.templateList.typePlaceholder": "类型筛选",
        "workflow.templateList.newTemplate": "新建模板",
        "workflow.templateList.preset": "预设",
        "workflow.templateList.custom": "自定义",
        "workflow.templateList.noDescription": "暂无描述",
        "workflow.templateList.readonly": "只读",
        "workflow.templateList.view": "查看",
        "workflow.templateList.edit": "编辑",
        "workflow.templateList.versionHistory": "版本历史",
        "workflow.templateList.duplicate": "复制",
        "workflow.templateList.delete": "删除",
        "workflow.templateList.confirmDelete": "确认删除",
        "workflow.templateList.confirmDeleteMessage": "确定要删除模板 {{name}} 吗？",
        "workflow.templateList.irreversible": "此操作不可撤销",
        "workflow.templateList.deleted": "已删除",
        "workflow.templateList.deleteFailed": "删除失败",
        "workflow.templateList.copied": "已复制",
        "workflow.templateList.copyFailed": "复制失败",
        "workflow.importExport.title": "导入/导出模板",
        "workflow.importExport.export": "导出",
        "workflow.importExport.import": "导入",
        "workflow.importExport.templateId": "模板 ID",
        "workflow.importExport.enterTemplateId": "输入要导出的模板 ID",
        "workflow.importExport.exportTemplate": "导出模板",
        "workflow.importExport.importTemplate": "导入模板",
        "workflow.importExport.pasteJsonPlaceholder": "粘贴模板 JSON 数据...",
        "workflow.importExport.pasteJsonData": "粘贴 JSON 数据",
        "workflow.importExport.uploadJsonFile": "上传 JSON 文件",
        "workflow.importExport.dragOrClickUpload": "拖拽或点击上传",
        "workflow.importExport.or": "或",
        "workflow.importExport.preview": "预览",
        "workflow.importExport.workflowName": "工作流名称",
        "workflow.importExport.format": "格式",
        "workflow.importExport.formatN8n": "n8n",
        "workflow.importExport.formatAxAgent": "AxAgent",
        "workflow.importExport.nodeCount": "节点数",
        "workflow.importExport.edgeCount": "连线数",
        "workflow.importExport.exportResultJson": "导出结果 JSON",
        "workflow.importExport.copied": "已复制",
        "workflow.importExport.copy": "复制",
        "workflow.importExport.pleaseEnterId": "请输入模板 ID",
        "workflow.importExport.pleaseEnterJson": "请输入 JSON 数据",
        "workflow.importExport.invalidJson": "无效的 JSON",
        "workflow.importExport.exportSuccess": "导出成功",
        "workflow.importExport.exportNotFound": "未找到模板",
        "workflow.importExport.exportFailed": "导出失败",
        "workflow.importExport.importSuccess": "成功导入 {{count}} 个模板",
        "workflow.importExport.templateImportSuccess": "模板导入成功",
        "workflow.importExport.importFailed": "导入失败",
        "workflow.importExport.importFailedWithError": "导入失败: {{error}}",
        "workflow.importExport.copiedToClipboard": "已复制到剪贴板",
        "workflow.importExport.fileReadFailed": "文件读取失败",
        "workflow.importExport.importHint": "导入提示",
        "workflow.importExport.batchImport": "批量导入",
        "workflow.importExport.n8nBatchImport": "n8n 批量导入",
        "workflow.importExport.selectFolder": "选择文件夹",
        "workflow.importExport.selectN8nDir": "选择 n8n 目录",
        "workflow.importExport.batchImportSuccess": "成功导入 {{count}} 个模板",
        "workflow.importExport.batchResult": "导入 {{count}} 个模板",
        "workflow.importExport.noJsonFound": "未找到 JSON 文件",
        "workflow.importExport.n8nResult": "导入 {{imported}} 个，跳过 {{skipped}} 个",
        "workflow.importExport.errorCount": "{{count}} 个错误",
        "workflow.importExport.viewAllErrors": "查看全部 {{count}} 个错误",
        "workflow.importExport.moreErrors": "还有 {{count}} 个错误",
      };
      const fallbackStr = typeof options === "object" && options !== null
        ? ((options as Record<string, unknown>).defaultValue as string)
        : (options as string | undefined);
      return translations[key] ?? fallbackStr ?? key;
    },
  }),
}));

// ─── Shared mutable mock state for useWorkflowEditorStore ───
const {
  mockStoreState,
  subscribeMock: _subscribeMock,
  setStateMock: _setStateMock,
  storeMockRef,
} = vi.hoisted(() => {
  const state: Record<string, any> = {
    nodes: [],
    edges: [],
    templates: [],
    isLoading: false,
    loadTemplates: vi.fn(),
    deleteTemplate: vi.fn(),
    duplicateTemplate: vi.fn(),
  };
  const sub = vi.fn();
  const ss = vi.fn();

  function createStoreMock() {
    const fn = vi.fn(() => ({ ...state })) as any;
    fn.getState = vi.fn(() => ({ ...state }));
    fn.setState = ss;
    fn.subscribe = sub;
    return fn;
  }

  const ref = createStoreMock();
  return {
    mockStoreState: state,
    subscribeMock: sub,
    setStateMock: ss,
    storeMockRef: ref,
  };
});

vi.mock("@/stores", () => ({
  useWorkflowEditorStore: storeMockRef,
}));

describe("AIPanel Component", () => {
  const mockOnGenerateWorkflow = vi.fn();
  const mockOnOptimizePrompt = vi.fn();
  const mockOnRecommendNodes = vi.fn();
  const mockOnClose = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should render AI panel with three tabs", async () => {
    const { AIPanel } = await import("@/components/workflow/AIPanel");

    render(
      <AIPanel
        onGenerateWorkflow={mockOnGenerateWorkflow}
        onOptimizePrompt={mockOnOptimizePrompt}
        onRecommendNodes={mockOnRecommendNodes}
        onClose={mockOnClose}
      />,
    );

    expect(screen.getByText("AI 助手")).toBeTruthy();
    expect(screen.getAllByText("生成工作流").length).toBeGreaterThan(0);
    expect(screen.getByText("优化 Prompt")).toBeTruthy();
    expect(screen.getByText("推荐节点")).toBeTruthy();
  });

  it("should have generate workflow tab active by default", async () => {
    const { AIPanel } = await import("@/components/workflow/AIPanel");

    render(
      <AIPanel
        onGenerateWorkflow={mockOnGenerateWorkflow}
        onOptimizePrompt={mockOnOptimizePrompt}
        onRecommendNodes={mockOnRecommendNodes}
        onClose={mockOnClose}
      />,
    );

    const generateTextarea = screen.getByPlaceholderText(/创建一个代码审查工作流/);
    expect(generateTextarea).toBeTruthy();
  });

  it("should call onGenerateWorkflow when generate button is clicked", async () => {
    mockOnGenerateWorkflow.mockResolvedValue({
      nodes: [{ id: "node-1", type: "trigger", data: { label: "Test" } }],
      edges: [],
    });

    const { AIPanel } = await import("@/components/workflow/AIPanel");

    render(
      <AIPanel
        onGenerateWorkflow={mockOnGenerateWorkflow}
        onOptimizePrompt={mockOnOptimizePrompt}
        onRecommendNodes={mockOnRecommendNodes}
        onClose={mockOnClose}
      />,
    );

    const textarea = screen.getByPlaceholderText(/创建一个代码审查工作流/);
    fireEvent.change(textarea, { target: { value: "Create a test workflow" } });

    const generateButton = screen.getByRole("button", { name: /生成工作流/ });
    fireEvent.click(generateButton);

    await waitFor(() => {
      expect(mockOnGenerateWorkflow).toHaveBeenCalledWith(
        "Create a test workflow",
      );
    });
  });

  it("should show warning when trying to generate with empty prompt", async () => {
    const { AIPanel } = await import("@/components/workflow/AIPanel");

    render(
      <AIPanel
        onGenerateWorkflow={mockOnGenerateWorkflow}
        onOptimizePrompt={mockOnOptimizePrompt}
        onRecommendNodes={mockOnRecommendNodes}
        onClose={mockOnClose}
      />,
    );

    const generateButton = screen.getByRole("button", { name: /生成工作流/ });
    fireEvent.click(generateButton);

    await waitFor(() => {
      expect(screen.getByText("请输入工作流描述")).toBeTruthy();
    });
  });
});

describe("TemplateList Component", () => {
  const mockOnSelectTemplate = vi.fn();
  const mockOnCreateNew = vi.fn();
  const mockOnEditTemplate = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    // Reset mutable mock state
    Object.assign(mockStoreState, {
      templates: [],
      isLoading: false,
      loadTemplates: vi.fn(),
      deleteTemplate: vi.fn(),
      duplicateTemplate: vi.fn(),
    });
  });

  it("should render loading state", async () => {
    mockStoreState.isLoading = true;

    const { TemplateList } = await import("@/components/workflow/Templates");

    const { container } = render(
      <TemplateList
        onSelectTemplate={mockOnSelectTemplate}
        onCreateNew={mockOnCreateNew}
        onEditTemplate={mockOnEditTemplate}
      />,
    );

    // The Ant Design Spin component renders with aria-busy="true" when spinning
    const spinner = container.querySelector(".ant-spin-spinning");
    expect(spinner).toBeTruthy();
  });

  it("should render empty state when no templates", async () => {
    mockStoreState.isLoading = false;
    mockStoreState.templates = [];

    const { TemplateList } = await import("@/components/workflow/Templates");

    render(
      <TemplateList
        onSelectTemplate={mockOnSelectTemplate}
        onCreateNew={mockOnCreateNew}
        onEditTemplate={mockOnEditTemplate}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText("暂无模板")).toBeTruthy();
    });
  });

  it("should render template cards when templates exist", async () => {
    const mockTemplates = [
      {
        id: "template-1",
        name: "Test Template",
        description: "A test template",
        icon: "📋",
        tags: ["test"],
        version: 1,
        is_preset: false,
        is_editable: true,
        is_public: false,
        trigger_config: null,
        nodes: [],
        edges: [],
        input_schema: null,
        output_schema: null,
        variables: null,
        error_config: null,
        created_at: Date.now(),
        updated_at: Date.now(),
      },
    ];

    mockStoreState.templates = mockTemplates;
    mockStoreState.isLoading = false;

    const { TemplateList } = await import("@/components/workflow/Templates");

    render(
      <TemplateList
        onSelectTemplate={mockOnSelectTemplate}
        onCreateNew={mockOnCreateNew}
        onEditTemplate={mockOnEditTemplate}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText("Test Template")).toBeTruthy();
      expect(screen.getByText("A test template")).toBeTruthy();
    });
  });

  it("should call onSelectTemplate when template card is clicked", async () => {
    const mockTemplate = {
      id: "template-1",
      name: "Test Template",
      description: "A test template",
      icon: "📋",
      tags: ["test"],
      version: 1,
      is_preset: false,
      is_editable: true,
      is_public: false,
      trigger_config: null,
      nodes: [],
      edges: [],
      input_schema: null,
      output_schema: null,
      variables: null,
      error_config: null,
      created_at: Date.now(),
      updated_at: Date.now(),
    };

    mockStoreState.templates = [mockTemplate];
    mockStoreState.isLoading = false;

    const { TemplateList } = await import("@/components/workflow/Templates");

    render(
      <TemplateList
        onSelectTemplate={mockOnSelectTemplate}
        onCreateNew={mockOnCreateNew}
        onEditTemplate={mockOnEditTemplate}
      />,
    );

    await waitFor(() => {
      const card = screen.getByText("Test Template");
      fireEvent.click(card);
    });

    expect(mockOnSelectTemplate).toHaveBeenCalledWith(mockTemplate);
  });
});

describe("ImportExportModal Component", () => {
  const mockOnClose = vi.fn();
  const mockOnExport = vi.fn();
  const mockOnImport = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should render modal with export and import tabs", async () => {
    const { ImportExportModal } = await import("@/components/workflow/Templates");

    render(
      <ImportExportModal
        open={true}
        onClose={mockOnClose}
        onExport={mockOnExport}
        onImport={mockOnImport}
        templates={[]}
      />,
    );

    expect(screen.getByText("导入/导出模板")).toBeTruthy();
    expect(screen.getAllByText("导出")).toBeTruthy();
    expect(screen.getByText("导出模板")).toBeTruthy();
    expect(screen.getByText("导入")).toBeTruthy();
  });

  it("should show export tab by default", async () => {
    const { ImportExportModal } = await import("@/components/workflow/Templates");

    render(
      <ImportExportModal
        open={true}
        onClose={mockOnClose}
        onExport={mockOnExport}
        onImport={mockOnImport}
        templates={[]}
      />,
    );

    expect(screen.getByText("导出模板")).toBeTruthy();
  });

  it("should switch to import tab when clicked", async () => {
    mockOnExport.mockResolvedValue(null);
    const { ImportExportModal } = await import("@/components/workflow/Templates");

    render(
      <ImportExportModal
        open={true}
        onClose={mockOnClose}
        onExport={mockOnExport}
        onImport={mockOnImport}
        templates={[]}
      />,
    );

    const importTab = screen.getByText("导入");
    fireEvent.click(importTab);

    await waitFor(() => {
      expect(screen.getByPlaceholderText("粘贴模板 JSON 数据...")).toBeTruthy();
      expect(screen.getByText("导入模板")).toBeTruthy();
    });
  });
});
