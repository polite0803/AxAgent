// SPDX-License-Identifier: AGPL-3.0-only

import {
  COMPONENT_REQUIRED_PROPS,
  type DynamicComponentType,
  type SchemaValidationError,
  type SchemaValidationResult,
  type UISchema,
  VALID_DYNAMIC_COMPONENT_TYPES,
} from "@/types";

/**
 * 使用递归遍历校验 UISchema 的结构合法性。
 * 校验项：
 * 1. 必填字段：version、id、type
 * 2. DynamicComponentType 是否为合法枚举值
 * 3. 组件类型与 props 的兼容性（如 Table 必须有 columns 字段）
 * 4. 递归校验 children
 */
export function validateSchema(schema: unknown): SchemaValidationResult {
  const errors: SchemaValidationError[] = [];
  validateNode(schema as UISchema, "root", errors);
  return {
    valid: errors.length === 0,
    errors,
  };
}

function validateNode(
  node: unknown,
  path: string,
  errors: SchemaValidationError[],
): void {
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
    return; // 后续校验依赖 type
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

  // 校验 conditionalDisplay
  if (Array.isArray(obj.conditionalDisplay)) {
    for (let i = 0; i < obj.conditionalDisplay.length; i++) {
      validateConditionalRule(
        obj.conditionalDisplay[i],
        `${path}.conditionalDisplay[${i}]`,
        errors,
      );
    }
  } else if (obj.conditionalDisplay !== undefined) {
    errors.push({
      path: `${path}.conditionalDisplay`,
      message: "conditionalDisplay 必须为数组类型",
    });
  }

  // 递归校验 children
  if (Array.isArray(obj.children)) {
    for (let i = 0; i < obj.children.length; i++) {
      validateNode(obj.children[i], `${path}.children[${i}]`, errors);
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
  ];
  if (!validOperators.includes(obj.operator as string)) {
    errors.push({
      path: `${path}.operator`,
      message: `无效的操作符 "${String(obj.operator)}"，有效: ${validOperators.join(", ")}`,
    });
  }
}
