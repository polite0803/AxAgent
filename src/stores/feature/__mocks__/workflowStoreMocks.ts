/**
 * Workflow Store Mock Data
 *
 * DEV-ONLY: 仅在开发环境（非生产构建）中导入。
 * 生产构建通过 import.meta.env.PROD 守卫排除，这些函数不会被 bundle。
 *
 * 包含：
 * - generateMockParseResult: 模拟 NL→工作流解析
 * - generateUISchema: 模拟 NL→UI Schema 生成
 * - extractTriggers: 触发词提取辅助函数
 */

import type { UISchema } from "@/types/dynamicUI";
import type {
  NL2SkillResult,
  NL2UIRequest,
  NL2UIResult,
  NLParseResult,
  SkillDefinition,
  WorkflowDefinition,
} from "@/types/workflow";

// ── ID 生成 ──

function makeId(): string {
  return `wf_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}

// ── NL→工作流 模拟 ──

export function generateMockParseResult(prompt: string): NLParseResult {
  const workflow: WorkflowDefinition = {
    id: makeId(),
    name: prompt.length > 30 ? prompt.slice(0, 30) + "..." : prompt,
    description: `基于自然语言描述自动生成的工作流: ${prompt}`,
    version: 1,
    nodes: [
      { id: "trigger-1", type: "trigger", label: "手动触发", config: {}, position: { x: 100, y: 50 } },
      {
        id: "action-1",
        type: "action",
        label: "处理步骤 1",
        config: { text: prompt },
        position: { x: 100, y: 160 },
        inputs: ["trigger-1"],
        outputs: ["result_1"],
      },
      {
        id: "condition-1",
        type: "condition",
        label: "条件判断",
        config: { expression: "result_1 !== null" },
        position: { x: 100, y: 280 },
        inputs: ["action-1"],
      },
      {
        id: "action-2",
        type: "action",
        label: "处理步骤 2",
        config: {},
        position: { x: 300, y: 280 },
        inputs: ["condition-1"],
        outputs: ["result_2"],
      },
      {
        id: "output-1",
        type: "output",
        label: "输出结果",
        config: {},
        position: { x: 100, y: 400 },
        inputs: ["action-2"],
      },
    ],
    edges: [
      { id: "e1", source: "trigger-1", target: "action-1" },
      { id: "e2", source: "action-1", target: "condition-1" },
      { id: "e3", source: "condition-1", target: "action-2" },
      { id: "e4", source: "action-2", target: "output-1" },
    ],
    variables: {},
    createdAt: Date.now(),
    updatedAt: Date.now(),
    status: "draft",
  };

  return {
    workflow,
    confidence: 0.72 + Math.random() * 0.2,
    suggestions: [
      "建议为 HTTP 请求节点添加超时和重试配置",
      "建议添加异常处理分支以提高工作流鲁棒性",
      "可考虑添加执行结果通知节点",
    ],
    alternatives: undefined,
  };
}

// ── NL→Skill 模拟 ──

export function extractTriggers(prompt: string): string[] {
  if (prompt.includes("客服")) { return ["客服", "帮助", "咨询", "问题"]; }
  if (prompt.includes("报告")) { return ["生成报告", "日报", "周报", "总结"]; }
  if (prompt.includes("翻译")) { return ["翻译", "translate", "译"]; }
  return ["帮助", "help", "怎么", "如何"];
}

export async function mockParseSkill(
  prompt: string,
  skillType: string,
  setProgress: (p: string) => void,
): Promise<NL2SkillResult> {
  const phases = [
    {
      phase: "意图分析",
      status: "done" as const,
      detail: `识别为${
        prompt.includes("客服")
          ? "客服自动回复"
          : prompt.includes("报告")
          ? "报告生成"
          : prompt.includes("翻译")
          ? "多语言翻译"
          : "自定义"
      }技能`,
    },
    {
      phase: "技能匹配",
      status: "done" as const,
      detail: `匹配到 ${skillType || "chat"} 类型，${extractTriggers(prompt).length} 个触发词`,
    },
    { phase: "参数提取", status: "done" as const, detail: "提取 2 个参数：query、context" },
    { phase: "模板生成", status: "done" as const, detail: "生成提示词模板" },
    { phase: "校验优化", status: "done" as const, detail: "通过语义校验，置信度 92%" },
  ];

  for (const p of phases) {
    await new Promise((r) => setTimeout(r, 300));
    setProgress(p.phase);
  }

  const skillId = `skill_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
  const skill: SkillDefinition = {
    id: skillId,
    name: prompt.includes("客服")
      ? "智能客服回复"
      : prompt.includes("报告")
      ? "日报生成"
      : prompt.includes("翻译")
      ? "多语言翻译"
      : "自定义技能",
    description: prompt.slice(0, 100),
    type: (skillType || "chat") as "chat" | "automation" | "tool" | "workflow",
    triggers: extractTriggers(prompt),
    prompt_template: `基于以下上下文回答问题：\n{{context}}\n\n用户问题：{{query}}\n\n要求：${prompt}`,
    parameters: [
      { name: "query", type: "string", description: "用户输入的问题", required: true },
      { name: "context", type: "string", description: "对话上下文", required: false, default: "" },
    ],
    tools: ["web_search", "knowledge_retrieval"],
    icon: "MessageSquare",
    tags: ["auto-generated"],
  };

  return {
    skill,
    confidence: 0.92,
    phases,
    suggestions: [
      "建议添加错误处理分支：当用户输入无法识别时返回友好提示",
      "可增加多轮对话支持，在 prompt_template 中引入对话历史变量",
      "建议为高频问题添加缓存机制以提升响应速度",
    ],
  };
}

// ── NL→UI 模拟 ──

export function generateUISchema(request: NL2UIRequest): UISchema {
  const uiType = request.uiType || "custom";

  if (uiType === "dashboard") {
    return {
      version: "1.0",
      id: `dashboard_${Date.now()}`,
      type: "Container",
      props: { style: { padding: "16px", display: "flex", flexDirection: "column", gap: "16px" } },
      children: [
        {
          version: "1.0",
          id: "row_1",
          type: "Row",
          props: { gutter: 16 },
          children: [
            {
              version: "1.0",
              id: "stat_1",
              type: "Card",
              props: { title: "总请求量" },
              children: [{
                version: "1.0",
                id: "stat_1_inner",
                type: "Text",
                props: { content: "12,847 次", style: { fontSize: "24px", fontWeight: "bold", color: "#52c41a" } },
              }],
            },
            {
              version: "1.0",
              id: "stat_2",
              type: "Card",
              props: { title: "成功率" },
              children: [{
                version: "1.0",
                id: "stat_2_inner",
                type: "Text",
                props: { content: "98.5%", style: { fontSize: "24px", fontWeight: "bold", color: "#1677ff" } },
              }],
            },
            {
              version: "1.0",
              id: "stat_3",
              type: "Card",
              props: { title: "平均耗时" },
              children: [{
                version: "1.0",
                id: "stat_3_inner",
                type: "Text",
                props: { content: "234 ms", style: { fontSize: "24px", fontWeight: "bold", color: "#faad14" } },
              }],
            },
          ],
        },
        {
          version: "1.0",
          id: "chart_1",
          type: "Card",
          props: { title: "请求趋势（近 7 天）" },
          children: [{
            version: "1.0",
            id: "chart_1_inner",
            type: "Chart",
            props: {
              chartType: "line",
              data: {
                labels: ["周一", "周二", "周三", "周四", "周五", "周六", "周日"],
                values: [1200, 1900, 1500, 2100, 1800, 2400, 1700],
              },
            },
          }],
        },
      ],
    };
  }

  if (uiType === "form") {
    return {
      version: "1.0",
      id: `form_${Date.now()}`,
      type: "Form",
      props: { layout: "vertical", submitText: "提交" },
      children: [
        {
          version: "1.0",
          id: "input_1",
          type: "Input",
          props: { label: "名称", name: "name", required: true, placeholder: "请输入名称" },
        },
        {
          version: "1.0",
          id: "select_1",
          type: "Select",
          props: {
            label: "类型",
            name: "type",
            options: [{ label: "选项A", value: "a" }, { label: "选项B", value: "b" }],
          },
        },
        { version: "1.0", id: "textarea_1", type: "Textarea", props: { label: "描述", name: "description", rows: 4 } },
        { version: "1.0", id: "switch_1", type: "Switch", props: { label: "启用", name: "enabled", default: true } },
        { version: "1.0", id: "btn_1", type: "Button", props: { children: "提交", type: "primary", action: "submit" } },
      ],
    };
  }

  if (uiType === "settings") {
    return {
      version: "1.0",
      id: `settings_${Date.now()}`,
      type: "Tabs",
      props: { items: [{ key: "general", label: "常规" }, { key: "advanced", label: "高级" }] },
      children: [
        {
          version: "1.0",
          id: "tab_general",
          type: "Container",
          props: { tabKey: "general" },
          children: [
            {
              version: "1.0",
              id: "input_appName",
              type: "Input",
              props: { label: "应用名称", name: "appName", default: "AxAgent" },
            },
            {
              version: "1.0",
              id: "select_lang",
              type: "Select",
              props: {
                label: "语言",
                name: "lang",
                options: [{ label: "中文", value: "zh" }, { label: "English", value: "en" }],
              },
            },
          ],
        },
        {
          version: "1.0",
          id: "tab_advanced",
          type: "Container",
          props: { tabKey: "advanced" },
          children: [
            {
              version: "1.0",
              id: "switch_debug",
              type: "Switch",
              props: { label: "调试模式", name: "debug", default: false },
            },
            { version: "1.0", id: "input_api", type: "Input", props: { label: "API 端点", name: "apiEndpoint" } },
          ],
        },
      ],
    };
  }

  // custom / report 默认
  const truncatedTitle = request.prompt.length > 30 ? request.prompt.slice(0, 30) + "..." : request.prompt;
  return {
    version: "1.0",
    id: `custom_${Date.now()}`,
    type: "Card",
    props: { title: truncatedTitle },
    children: [
      {
        version: "1.0",
        id: "md_1",
        type: "Markdown",
        props: { content: `# 基于描述生成的 UI\n\n${request.prompt}\n\n*此 UI 由 NL2UI 自动生成*` },
      },
      { version: "1.0", id: "tag_1", type: "Tag", props: { children: "AI 生成", color: "blue" } },
    ],
  };
}

export async function mockParseUI(
  request: NL2UIRequest,
  setProgress: (p: string) => void,
): Promise<NL2UIResult> {
  const uiType = request.uiType || "custom";
  const phases = [
    { phase: "意图分析", status: "done" as const, detail: `识别为 ${uiType} 类型 UI` },
    { phase: "布局规划", status: "done" as const, detail: "规划布局，组件编排" },
    { phase: "组件选择", status: "done" as const, detail: "选择 Form + Table + Card + Statistic + Chart + Tag" },
    { phase: "Schema构建", status: "done" as const, detail: "生成 UISchema" },
    { phase: "校验优化", status: "done" as const, detail: "通过 JSON Schema 校验，无循环引用" },
  ];

  for (const p of phases) {
    await new Promise((r) => setTimeout(r, 300));
    setProgress(p.phase);
  }

  const schema = generateUISchema(request);

  return {
    schema,
    confidence: 0.88,
    phases,
    suggestions: [
      "建议将表格列宽设为响应式以适应不同屏幕",
      "可在 Card 外层添加 Tabs 组件以分组展示不同维度数据",
      "Chart 建议接入实际数据源后调整颜色映射",
    ],
  };
}
