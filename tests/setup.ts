// SPDX-License-Identifier: AGPL-3.0-only
// 集成测试全局 Setup

import "@testing-library/jest-dom/vitest";
import { vi } from "vitest";

// ── Mock Tauri API ──
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

// ── Mock react-router-dom (仅覆盖 useNavigate，useLocation 走 MemoryRouter 真实实现) ──
vi.mock("react-router-dom", async () => {
  const actual = await vi.importActual("react-router-dom");
  return {
    ...(actual as object),
    useNavigate: vi.fn(() => vi.fn()),
  };
});

// ── Mock react-i18next ──
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "zh", changeLanguage: vi.fn() },
  }),
}));

// ── Mock lucide-react 图标（避免 SVG 渲染问题） ──
vi.mock("lucide-react", async () => {
  const actual = await vi.importActual("lucide-react");
  const mockIcon = (name: string) => {
    const MockIcon = (props: Record<string, unknown>) => ({
      $$typeof: Symbol.for("react.element"),
      type: name,
      props,
    });
    MockIcon.displayName = name;
    return MockIcon;
  };
  const exports: Record<string, unknown> = {};
  if (actual && typeof actual === "object") {
    for (const key of Object.keys(actual as object)) {
      exports[key] = mockIcon(key);
    }
  }
  return exports;
});

// ── Mock @xyflow/react ──
vi.mock("@xyflow/react", () => ({
  ReactFlowProvider: ({ children }: { children: React.ReactNode }) => children,
  ReactFlow: () => null,
  useReactFlow: () => ({
    fitView: vi.fn(),
    setCenter: vi.fn(),
    getNodes: vi.fn(() => []),
    getEdges: vi.fn(() => []),
  }),
  useNodesState: () => [[], vi.fn(), vi.fn()],
  useEdgesState: () => [[], vi.fn(), vi.fn()],
  addEdge: vi.fn(),
  MarkerType: { ArrowClosed: "arrowclosed" },
  Position: { Left: "left", Right: "right", Top: "top", Bottom: "bottom" },
}));

// ── Mock DOMPurify ──
vi.mock("dompurify", () => ({
  default: {
    sanitize: (html: string) => html,
  },
}));

// ── Mock dndState (拖拽状态) ──
vi.mock("@/components/workflow/dndState", () => ({
  setDragPayload: vi.fn(),
}));

// ── Mock window.matchMedia (Ant Design 依赖) ──
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// ── Mock ResizeObserver ──
class MockResizeObserver {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
Object.defineProperty(window, "ResizeObserver", { value: MockResizeObserver });

// ── Mock scrollIntoView ──
Element.prototype.scrollIntoView = vi.fn();

// ── 重置所有 store 的 helper ──
import { useWorkflowEditorStore } from "@/stores/feature/workflowEditorStore";

beforeEach(() => {
  // 重置 workflowEditorStore
  useWorkflowEditorStore.setState({
    nodes: [],
    edges: [],
    currentTemplate: null,
    pendingAiChatActions: null,
    chatHistory: [],
    chatStreaming: false,
  });

  // NOTE: evolutionStore 不在此处重置。它的 mock 数据在 create() 时构建，
  // 各测试如需干净状态，应在各自测试文件中通过 describe 级别的 beforeEach 处理。
});
