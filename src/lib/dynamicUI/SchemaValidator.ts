// SPDX-License-Identifier: AGPL-3.0-only

import {
  COMPONENT_REQUIRED_PROPS,
  type DynamicComponentType,
  type SchemaValidationError,
  type SchemaValidationResult,
  type UISchema,
  VALID_DYNAMIC_COMPONENT_TYPES,
} from "@/types";

const VALID_IMPORTANCE = ["low", "medium", "high", "critical"];
const VALID_STATUS = ["pending", "ready", "error", "loading"];

/** Maximum nesting depth for schema validation to prevent stack overflow. */
const MAX_NESTING_DEPTH = 50;

/**
 * 使用递归遍历校验 UISchema 的结构合法性。
 * 校验项：
 * 1. 必填字段：version、id、type
 * 2. DynamicComponentType 是否为合法枚举值
 * 3. 组件类型与 props 的兼容性（如 Table 必须有 columns 字段）
 * 4. 递归校验 children（最大深度 50 层）
 */
export function validateSchema(schema: unknown): SchemaValidationResult {
  const errors: SchemaValidationError[] = [];
  validateNode(schema as UISchema, "root", errors, 0);
  return {
    valid: errors.length === 0,
    errors,
  };
}

function validateNode(
  node: unknown,
  path: string,
  errors: SchemaValidationError[],
  depth: number,
): void {
  if (depth > MAX_NESTING_DEPTH) {
    errors.push({
      path,
      message: `节点嵌套深度超过上限 ${MAX_NESTING_DEPTH}，可能存在循环引用或恶意构造`,
    });
    return;
  }
  if (typeof node !== "object" || node === null) {
    errors.push({
      path,
      message: `节点必须为对象类型，实际为 ${typeof node}`,
    });
    return;
  }

  const obj = node as Record<string, unknown>;

  // 必填字段校验
  if (typeof obj.id !== "string" || obj.id.length === 0) {
    errors.push({ path: `${path}.id`, message: "缺少必填字段 id" });
  }
  if (typeof obj.version !== "string" || obj.version.length === 0) {
    errors.push({ path: `${path}.version`, message: "缺少必填字段 version" });
  }
  if (typeof obj.type !== "string" || obj.type.length === 0) {
    errors.push({ path: `${path}.type`, message: "缺少必填字段 type" });
    // 继续校验 children，不提前 return，确保子节点也能被校验到
    if (Array.isArray(obj.children)) {
      for (let i = 0; i < obj.children.length; i++) {
        validateNode(obj.children[i], `${path}.children[${i}]`, errors, depth + 1);
      }
    }
    return;
  }

  const type = obj.type as string;

  // 校验 ComponentType 合法性
  if (!VALID_DYNAMIC_COMPONENT_TYPES.has(type)) {
    errors.push({
      path: `${path}.type`,
      message: `未知组件类型 "${type}"，有效类型: ${[...VALID_DYNAMIC_COMPONENT_TYPES].sort().join(", ")}`,
    });
  }

  // 校验 props
  const props = obj.props;
  if (props !== undefined && (typeof props !== "object" || props === null)) {
    errors.push({
      path: `${path}.props`,
      message: "props 必须为对象类型",
    });
  }

  // props 兼容性校验
  const requiredProps = COMPONENT_REQUIRED_PROPS[type as DynamicComponentType];
  if (requiredProps && requiredProps.length > 0) {
    const propsObj = (props as Record<string, unknown>) || {};
    for (const field of requiredProps) {
      if (
        propsObj[field] === undefined
        || propsObj[field] === null
      ) {
        errors.push({
          path: `${path}.props.${field}`,
          message: `组件 "${type}" 缺少必填属性 "${field}"`,
        });
      }
    }
  }

  // 形状校验（D-11）：增强类型检查，不仅判断存在性
  shapeValidateProps(type, props, path, errors);

  // 校验 dataSource
  if (obj.dataSource !== undefined) {
    validateDataSource(obj.dataSource, `${path}.dataSource`, errors);
  }

  // 校验 events
  if (Array.isArray(obj.events)) {
    for (let i = 0; i < obj.events.length; i++) {
      validateEventHandler(obj.events[i], `${path}.events[${i}]`, errors);
    }
  } else if (obj.events !== undefined) {
    errors.push({
      path: `${path}.events`,
      message: "events 必须为数组类型",
    });
  }

  // 校验 conditionalDisplay（支持数组形式和对象形式）
  if (obj.conditionalDisplay !== undefined) {
    validateConditionalDisplay(
      obj.conditionalDisplay,
      `${path}.conditionalDisplay`,
      errors,
      depth + 1,
    );
  }

  // 校验语义化：importance
  if (obj.importance !== undefined) {
    if (!VALID_IMPORTANCE.includes(obj.importance as string)) {
      errors.push({
        path: `${path}.importance`,
        message: `无效的 importance "${String(obj.importance)}"，有效: ${VALID_IMPORTANCE.join("|")}`,
      });
    }
  }

  // 校验语义化：status
  if (obj.status !== undefined) {
    if (!VALID_STATUS.includes(obj.status as string)) {
      errors.push({
        path: `${path}.status`,
        message: `无效的 status "${String(obj.status)}"，有效: ${VALID_STATUS.join("|")}`,
      });
    }
  }

  // 校验语义化：fallback（递归校验 fallback schema）
  if (obj.fallback !== undefined) {
    if (typeof obj.fallback !== "object" || obj.fallback === null) {
      errors.push({ path: `${path}.fallback`, message: "fallback 必须为 UISchema 对象" });
    } else {
      validateNode(obj.fallback as UISchema, `${path}.fallback`, errors, depth + 1);
    }
  }

  // 递归校验 children
  if (Array.isArray(obj.children)) {
    for (let i = 0; i < obj.children.length; i++) {
      validateNode(obj.children[i], `${path}.children[${i}]`, errors, depth + 1);
    }
  }
}

function validateDataSource(
  ds: unknown,
  path: string,
  errors: SchemaValidationError[],
): void {
  if (typeof ds !== "object" || ds === null) {
    errors.push({ path, message: "dataSource 必须为对象类型" });
    return;
  }
  const obj = ds as Record<string, unknown>;
  const validTypes = ["store", "api", "static", "agent-generated"];
  if (!validTypes.includes(obj.type as string)) {
    errors.push({
      path: `${path}.type`,
      message: `无效的数据源类型 "${String(obj.type)}"，有效类型: ${validTypes.join(", ")}`,
    });
  }
  if (
    obj.config === undefined
    || (typeof obj.config !== "object" || obj.config === null)
  ) {
    errors.push({
      path: `${path}.config`,
      message: "dataSource.config 必须为对象类型",
    });
  }
}

function validateEventHandler(
  handler: unknown,
  path: string,
  errors: SchemaValidationError[],
): void {
  if (typeof handler !== "object" || handler === null) {
    errors.push({ path, message: "EventHandler 必须为对象类型" });
    return;
  }
  const obj = handler as Record<string, unknown>;
  const validTriggers = [
    "onClick",
    "onChange",
    "onSubmit",
    "onMount",
    "onUnmount",
  ];
  if (!validTriggers.includes(obj.trigger as string)) {
    errors.push({
      path: `${path}.trigger`,
      message: `无效的触发器 "${String(obj.trigger)}"，有效: ${validTriggers.join(", ")}`,
    });
  }
  if (!Array.isArray(obj.actions)) {
    errors.push({
      path: `${path}.actions`,
      message: "actions 必须为数组类型",
    });
  }
}

function validateConditionalDisplay(
  display: unknown,
  path: string,
  errors: SchemaValidationError[],
  depth: number,
): void {
  if (depth > MAX_NESTING_DEPTH) {
    errors.push({
      path,
      message: `conditionalDisplay 嵌套深度超过上限 ${MAX_NESTING_DEPTH}`,
    });
    return;
  }

  if (Array.isArray(display)) {
    const arr = display as unknown[];
    if (arr.length === 0) {
      errors.push({ path, message: "conditionalDisplay 数组不能为空" });
      return;
    }
    for (let i = 0; i < arr.length; i++) {
      validateConditionalRule(arr[i], `${path}[${i}]`, errors);
    }
    return;
  }

  if (typeof display !== "object" || display === null) {
    errors.push({ path, message: "conditionalDisplay 必须为数组或对象类型" });
    return;
  }

  const obj = display as Record<string, unknown>;

  if (obj.logic !== "and" && obj.logic !== "or") {
    errors.push({
      path: `${path}.logic`,
      message: `conditionalDisplay.logic 必须为 "and" 或 "or"，实际为 "${String(obj.logic)}"`,
    });
  }

  if (!Array.isArray(obj.rules) || obj.rules.length === 0) {
    errors.push({
      path: `${path}.rules`,
      message: "conditionalDisplay.rules 必须为非空数组",
    });
  } else {
    for (let i = 0; i < obj.rules.length; i++) {
      validateConditionalDisplay(obj.rules[i], `${path}.rules[${i}]`, errors, depth + 1);
    }
  }

  if (obj.not !== undefined && typeof obj.not !== "boolean") {
    errors.push({
      path: `${path}.not`,
      message: "conditionalDisplay.not 必须为布尔类型",
    });
  }
}

function validateConditionalRule(
  rule: unknown,
  path: string,
  errors: SchemaValidationError[],
): void {
  if (typeof rule !== "object" || rule === null) {
    errors.push({ path, message: "ConditionalRule 必须为对象类型" });
    return;
  }
  const obj = rule as Record<string, unknown>;
  if (typeof obj.field !== "string" || obj.field.length === 0) {
    errors.push({ path: `${path}.field`, message: "缺少必填字段 field" });
  }
  const validOperators = [
    "eq",
    "neq",
    "gt",
    "gte",
    "lt",
    "lte",
    "in",
    "contains",
    "exists",
    "empty",
  ];
  if (!validOperators.includes(obj.operator as string)) {
    errors.push({
      path: `${path}.operator`,
      message: `无效的操作符 "${String(obj.operator)}"，有效: ${validOperators.join(", ")}`,
    });
  }
}

/** 已知的有效 chartType 值 */
const VALID_CHART_TYPES = ["line", "bar", "pie", "scatter", "area"];

/** 组件 props 形状校验规则：存在性之外的类型/枚举检查（D-11） */
const PROPS_SHAPE_RULES: Record<
  string,
  Array<{
    field: string;
    check: (val: unknown) => string | null;
  }>
> = {
  Table: [
    { field: "columns", check: (v) => Array.isArray(v) ? null : "columns 必须为数组" },
  ],
  Chart: [
    {
      field: "chartType",
      check: (v) =>
        VALID_CHART_TYPES.includes(v as string)
          ? null
          : `chartType 必须为 ${VALID_CHART_TYPES.join("|")}，实际为 "${String(v)}"`,
    },
  ],
  Dashboard: [
    { field: "items", check: (v) => Array.isArray(v) ? null : "items 必须为数组" },
  ],
  Tree: [
    { field: "treeData", check: (v) => Array.isArray(v) ? null : "treeData 必须为数组" },
  ],
  Timeline: [
    { field: "items", check: (v) => Array.isArray(v) ? null : "items 必须为数组" },
  ],
  Grid: [
    { field: "columns", check: (v) => typeof v === "number" ? null : "columns 必须为数字" },
  ],
};

function shapeValidateProps(
  type: string,
  props: unknown,
  path: string,
  errors: SchemaValidationError[],
): void {
  const rules = PROPS_SHAPE_RULES[type];
  if (!rules) {
    return;
  }
  const propsObj = (props as Record<string, unknown>) || {};
  for (const { field, check } of rules) {
    const val = propsObj[field];
    if (val === undefined || val === null) {
      continue; // 存在性已由 COMPONENT_REQUIRED_PROPS 校验
    }
    const errMsg = check(val);
    if (errMsg) {
      errors.push({ path: `${path}.props.${field}`, message: errMsg });
    }
  }
}
