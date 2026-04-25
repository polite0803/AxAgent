import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';

const invokeMock = vi.fn();

vi.mock('@/lib/invoke', () => ({
  invoke: invokeMock,
  listen: () => vi.fn(),
  isTauri: () => false,
}));

// ─── Shared mutable mock state for useWorkflowEditorStore ───
const mockStoreState: Record<string, any> = {
  nodes: [],
  edges: [],
  templates: [],
  isLoading: false,
  loadTemplates: vi.fn(),
  deleteTemplate: vi.fn(),
  duplicateTemplate: vi.fn(),
};
const subscribeMock = vi.fn();
const setStateMock = vi.fn();

// Create a mock that delegates to the mutable `mockStoreState` object.
// `vi.fn()` returns a function — calling `useWorkflowEditorStore()` returns the current state.
function createStoreMock() {
  const fn = vi.fn(() => ({ ...mockStoreState })) as any;
  fn.getState = vi.fn(() => ({ ...mockStoreState }));
  fn.setState = setStateMock;
  fn.subscribe = subscribeMock;
  return fn;
}

// We need to hold a stable reference for 'subscribe' etc.
const storeMockRef = createStoreMock();

vi.mock('@/stores', () => ({
  useWorkflowEditorStore: storeMockRef,
}));

describe('AIPanel Component', () => {
  const mockOnGenerateWorkflow = vi.fn();
  const mockOnOptimizePrompt = vi.fn();
  const mockOnRecommendNodes = vi.fn();
  const mockOnClose = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render AI panel with three tabs', async () => {
    const { AIPanel } = await import('@/components/workflow/AIPanel');

    render(
      <AIPanel
        onGenerateWorkflow={mockOnGenerateWorkflow}
        onOptimizePrompt={mockOnOptimizePrompt}
        onRecommendNodes={mockOnRecommendNodes}
        onClose={mockOnClose}
      />
    );

    expect(screen.getByText('AI 助手')).toBeTruthy();
    expect(screen.getAllByText('生成工作流').length).toBeGreaterThan(0);
    expect(screen.getByText('优化 Prompt')).toBeTruthy();
    expect(screen.getByText('推荐节点')).toBeTruthy();
  });

  it('should have generate workflow tab active by default', async () => {
    const { AIPanel } = await import('@/components/workflow/AIPanel');

    render(
      <AIPanel
        onGenerateWorkflow={mockOnGenerateWorkflow}
        onOptimizePrompt={mockOnOptimizePrompt}
        onRecommendNodes={mockOnRecommendNodes}
        onClose={mockOnClose}
      />
    );

    const generateTextarea = screen.getByPlaceholderText(/创建一个代码审查工作流/);
    expect(generateTextarea).toBeTruthy();
  });

  it('should call onGenerateWorkflow when generate button is clicked', async () => {
    mockOnGenerateWorkflow.mockResolvedValue({
      nodes: [{ id: 'node-1', type: 'trigger', data: { label: 'Test' } }],
      edges: [],
    });

    const { AIPanel } = await import('@/components/workflow/AIPanel');

    render(
      <AIPanel
        onGenerateWorkflow={mockOnGenerateWorkflow}
        onOptimizePrompt={mockOnOptimizePrompt}
        onRecommendNodes={mockOnRecommendNodes}
        onClose={mockOnClose}
      />
    );

    const textarea = screen.getByPlaceholderText(/创建一个代码审查工作流/);
    fireEvent.change(textarea, { target: { value: 'Create a test workflow' } });

    const generateButton = screen.getByRole('button', { name: /生成工作流/ });
    fireEvent.click(generateButton);

    await waitFor(() => {
      expect(mockOnGenerateWorkflow).toHaveBeenCalledWith('Create a test workflow');
    });
  });

  it('should show warning when trying to generate with empty prompt', async () => {
    const { AIPanel } = await import('@/components/workflow/AIPanel');

    render(
      <AIPanel
        onGenerateWorkflow={mockOnGenerateWorkflow}
        onOptimizePrompt={mockOnOptimizePrompt}
        onRecommendNodes={mockOnRecommendNodes}
        onClose={mockOnClose}
      />
    );

    const generateButton = screen.getByRole('button', { name: /生成工作流/ });
    fireEvent.click(generateButton);

    await waitFor(() => {
      expect(screen.getByText('请输入工作流描述')).toBeTruthy();
    });
  });
});

describe('TemplateList Component', () => {
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

  it('should render loading state', async () => {
    mockStoreState.isLoading = true;

    const { TemplateList } = await import('@/components/workflow/Templates');

    const { container } = render(
      <TemplateList
        onSelectTemplate={mockOnSelectTemplate}
        onCreateNew={mockOnCreateNew}
        onEditTemplate={mockOnEditTemplate}
      />
    );

    // The Ant Design Spin component renders with aria-busy="true" when spinning
    const spinner = container.querySelector('.ant-spin-spinning');
    expect(spinner).toBeTruthy();
  });

  it('should render empty state when no templates', async () => {
    mockStoreState.isLoading = false;
    mockStoreState.templates = [];

    const { TemplateList } = await import('@/components/workflow/Templates');

    render(
      <TemplateList
        onSelectTemplate={mockOnSelectTemplate}
        onCreateNew={mockOnCreateNew}
        onEditTemplate={mockOnEditTemplate}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('暂无模板')).toBeTruthy();
    });
  });

  it('should render template cards when templates exist', async () => {
    const mockTemplates = [
      {
        id: 'template-1',
        name: 'Test Template',
        description: 'A test template',
        icon: '📋',
        tags: ['test'],
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

    const { TemplateList } = await import('@/components/workflow/Templates');

    render(
      <TemplateList
        onSelectTemplate={mockOnSelectTemplate}
        onCreateNew={mockOnCreateNew}
        onEditTemplate={mockOnEditTemplate}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('Test Template')).toBeTruthy();
      expect(screen.getByText('A test template')).toBeTruthy();
    });
  });

  it('should call onSelectTemplate when template card is clicked', async () => {
    const mockTemplate = {
      id: 'template-1',
      name: 'Test Template',
      description: 'A test template',
      icon: '📋',
      tags: ['test'],
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

    const { TemplateList } = await import('@/components/workflow/Templates');

    render(
      <TemplateList
        onSelectTemplate={mockOnSelectTemplate}
        onCreateNew={mockOnCreateNew}
        onEditTemplate={mockOnEditTemplate}
      />
    );

    await waitFor(() => {
      const card = screen.getByText('Test Template');
      fireEvent.click(card);
    });

    expect(mockOnSelectTemplate).toHaveBeenCalledWith(mockTemplate);
  });
});

describe('ImportExportModal Component', () => {
  const mockOnClose = vi.fn();
  const mockOnExport = vi.fn();
  const mockOnImport = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render modal with export and import tabs', async () => {
    const { ImportExportModal } = await import('@/components/workflow/Templates');

    render(
      <ImportExportModal
        open={true}
        onClose={mockOnClose}
        onExport={mockOnExport}
        onImport={mockOnImport}
      />
    );

    expect(screen.getByText('导入/导出模板')).toBeTruthy();
    // Export tab is active by default — its label and content should be visible
    expect(screen.getAllByText('导出')).toBeTruthy();
    expect(screen.getByText('导出模板')).toBeTruthy();
    // Import tab label should be visible (tabs render all labels, but not all content)
    expect(screen.getByText('导入')).toBeTruthy();
  });

  it('should show export tab by default', async () => {
    const { ImportExportModal } = await import('@/components/workflow/Templates');

    render(
      <ImportExportModal
        open={true}
        onClose={mockOnClose}
        onExport={mockOnExport}
        onImport={mockOnImport}
      />
    );

    expect(screen.getByPlaceholderText('输入要导出的模板 ID')).toBeTruthy();
    expect(screen.getByText('导出模板')).toBeTruthy();
  });

  it('should switch to import tab when clicked', async () => {
    mockOnExport.mockResolvedValue(null);
    const { ImportExportModal } = await import('@/components/workflow/Templates');

    render(
      <ImportExportModal
        open={true}
        onClose={mockOnClose}
        onExport={mockOnExport}
        onImport={mockOnImport}
      />
    );

    // Click the import tab
    const importTab = screen.getByText('导入');
    fireEvent.click(importTab);

    await waitFor(() => {
      // After switching to import tab, import-specific content should appear
      expect(screen.getByPlaceholderText('粘贴模板 JSON 数据...')).toBeTruthy();
      expect(screen.getByText('导入模板')).toBeTruthy();
    });
  });
});
