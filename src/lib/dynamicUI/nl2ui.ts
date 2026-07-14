// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicComponentType, UISchema } from "@/types";

interface FieldPattern {
  keywords: string[];
  type: DynamicComponentType;
  label: string;
  name: string;
  placeholder?: string;
  required?: boolean;
  options?: Array<{ label: string; value: string }>;
}

const FIELD_PATTERNS: FieldPattern[] = [
  {
    keywords: ["姓名", "name", "名字", "用户名", "username", "昵称"],
    type: "Input",
    label: "姓名",
    name: "name",
    placeholder: "请输入姓名",
  },
  {
    keywords: ["邮箱", "email", "e-mail", "邮件"],
    type: "Input",
    label: "邮箱",
    name: "email",
    placeholder: "请输入邮箱",
  },
  {
    keywords: ["电话", "phone", "手机", "mobile", "tel"],
    type: "Input",
    label: "电话",
    name: "phone",
    placeholder: "请输入电话号码",
  },
  {
    keywords: ["地址", "address", "住址"],
    type: "Textarea",
    label: "地址",
    name: "address",
    placeholder: "请输入地址",
  },
  {
    keywords: ["备注", "remark", "note", "comment", "描述", "description"],
    type: "Textarea",
    label: "备注",
    name: "remark",
    placeholder: "请输入备注",
  },
  { keywords: ["年龄", "age"], type: "Number", label: "年龄", name: "age", placeholder: "请输入年龄" },
  {
    keywords: ["价格", "price", "金额", "amount"],
    type: "Number",
    label: "金额",
    name: "amount",
    placeholder: "请输入金额",
  },
  {
    keywords: ["数量", "quantity", "count"],
    type: "Number",
    label: "数量",
    name: "quantity",
    placeholder: "请输入数量",
  },
  { keywords: ["日期", "date", "时间", "time", "生日", "birthday"], type: "DatePicker", label: "日期", name: "date" },
  {
    keywords: ["性别", "gender", "sex"],
    type: "Radio",
    label: "性别",
    name: "gender",
    options: [{ label: "男", value: "male" }, { label: "女", value: "female" }],
  },
  { keywords: ["是否", "enable", "disabled", "开关", "同意", "agree"], type: "Switch", label: "启用", name: "enabled" },
  {
    keywords: ["分类", "category", "类型", "type", "状态", "status"],
    type: "Select",
    label: "分类",
    name: "category",
    options: [{ label: "选项1", value: "opt1" }, { label: "选项2", value: "opt2" }],
  },
  {
    keywords: ["标题", "title", "主题", "subject"],
    type: "Input",
    label: "标题",
    name: "title",
    placeholder: "请输入标题",
    required: true,
  },
  {
    keywords: ["内容", "content", "正文", "body", "message", "消息"],
    type: "Textarea",
    label: "内容",
    name: "content",
    placeholder: "请输入内容",
  },
];

function detectFields(prompt: string, genId: (p: string) => string): UISchema[] {
  const fields: UISchema[] = [];
  const usedNames = new Set<string>();

  const lowerPrompt = prompt.toLowerCase();

  for (const pattern of FIELD_PATTERNS) {
    const matched = pattern.keywords.some((kw) => {
      if (/^[\0-\x7f]+$/.test(kw)) {
        return lowerPrompt.includes(kw.toLowerCase());
      }
      return prompt.includes(kw);
    });

    if (matched && !usedNames.has(pattern.name)) {
      usedNames.add(pattern.name);
      const props: Record<string, unknown> = {
        name: pattern.name,
        label: pattern.label,
      };
      if (pattern.placeholder) {
        props.placeholder = pattern.placeholder;
      }
      if (pattern.required) {
        props.required = true;
      }
      if (pattern.options) {
        props.options = pattern.options;
      }
      fields.push({
        version: "1.0",
        id: genId("field"),
        type: pattern.type,
        props,
      });
    }
  }

  if (fields.length === 0) {
    const lower = prompt.toLowerCase();
    if (
      lower.includes("dashboard") || lower.includes("仪表盘")
      || lower.includes("report") || lower.includes("报表")
      || lower.includes("chart") || lower.includes("图表")
    ) {
      fields.push({
        version: "1.0",
        id: genId("field"),
        type: "Chart",
        props: {
          name: "chart",
          label: "图表",
          chartType: "line",
        },
      });
    } else {
      fields.push({
        version: "1.0",
        id: genId("field"),
        type: "Input",
        props: {
          name: "title",
          label: "标题",
          placeholder: "请输入标题",
          required: true,
        },
      });
      fields.push({
        version: "1.0",
        id: genId("field"),
        type: "Textarea",
        props: {
          name: "content",
          label: "内容",
          placeholder: "请输入内容",
        },
      });
    }
  }

  return fields;
}

function detectTitle(prompt: string): string {
  const titlePatterns = [
    /(?:叫做|称为|命名为|名为|title[:：是为叫])["""]?([^"""\n，。,.]{2,20})["""]?/,
    /(?:form|表单|page|页面|dashboard|仪表盘|report|报表)[:\s]*([^\n，。,.]{2,20})/i,
  ];
  for (const pat of titlePatterns) {
    const m = prompt.match(pat);
    if (m && m[1]) {
      return m[1].trim();
    }
  }

  if (prompt.includes("表单") || prompt.includes("form")) {
    return "表单";
  }
  if (prompt.includes("报表") || prompt.includes("report")) {
    return "报表";
  }
  if (prompt.includes("仪表盘") || prompt.includes("dashboard")) {
    return "仪表盘";
  }
  return "动态UI";
}

export function generateUIFromNaturalLanguage(prompt: string): {
  schema: UISchema;
  title: string;
  description: string;
} {
  let counter = 0;
  const genId = (prefix: string): string => {
    counter += 1;
    return `${prefix}-${counter}`;
  };

  const title = detectTitle(prompt);
  const fields = detectFields(prompt, genId);

  const schema: UISchema = {
    version: "1.0",
    id: genId("root"),
    type: "Column",
    props: {},
    children: [
      {
        version: "1.0",
        id: genId("title"),
        type: "Text",
        props: { content: title, strong: true },
      },
      {
        version: "1.0",
        id: genId("form"),
        type: "Form",
        props: {
          layout: "vertical",
          submitText: "提交",
        },
        events: [
          {
            trigger: "onSubmit",
            actions: [
              { type: "store", config: {} },
            ],
          },
        ],
        children: fields,
      },
    ],
  };

  return {
    schema,
    title,
    description: `由自然语言生成：${prompt.slice(0, 50)}${prompt.length > 50 ? "..." : ""}`,
  };
}

export async function generateUIFromNL(prompt: string): Promise<{
  schema: UISchema;
  title: string;
  description: string;
}> {
  return generateUIFromNaturalLanguage(prompt);
}
