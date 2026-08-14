// SPDX-License-Identifier: AGPL-3.0-only

import {
  COMPONENT_REQUIRED_PROPS,
  type DynamicComponentType,
  type SchemaValidationError,
  type SchemaValidationResult,
  type UISchema,
  VALID_DYNAMIC_COMPONENT_TYPES,
} from "@/types";
import { isAllowedFetchUrl, isAllowedInvokeEndpoint } from "./dataSourceSecurity";

const VALID_IMPORTANCE = ["low", "medium", "high", "critical"];
const VALID_STATUS = ["pending", "ready", "error", "loading"];

/** Maximum nesting depth for schema validation to prevent stack overflow. */
const MAX_NESTING_DEPTH = 50;

/**
 * Recursively validates the structural integrity of a UISchema.
 * Validation items:
 * 1. Required fields: version, id, type
 * 2. DynamicComponentType is a valid enum value
 * 3. Component type and props compatibility (e.g. Table must have columns field)
 * 4. Recursively validates children (max depth 50 levels)
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
      message:
        `Node nesting depth exceeds limit ${MAX_NESTING_DEPTH}, possible circular reference or malicious structure`,
    });
    return;
  }
  if (typeof node !== "object" || node === null) {
    errors.push({
      path,
      message: `Node must be an object type, got ${typeof node}`,
    });
    return;
  }

  const obj = node as Record<string, unknown>;

  // Required field validation
  if (typeof obj.id !== "string" || obj.id.length === 0) {
    errors.push({ path: `${path}.id`, message: "Missing required field: id" });
  }
  if (typeof obj.version !== "string" || obj.version.length === 0) {
    errors.push({ path: `${path}.version`, message: "Missing required field: version" });
  }
  if (typeof obj.type !== "string" || obj.type.length === 0) {
    errors.push({ path: `${path}.type`, message: "Missing required field: type" });
    // Continue to validate children without early return to ensure sub-nodes are also validated
    if (Array.isArray(obj.children)) {
      for (let i = 0; i < obj.children.length; i++) {
        validateNode(obj.children[i], `${path}.children[${i}]`, errors, depth + 1);
      }
    }
    return;
  }

  const type = obj.type as string;

  // Validate ComponentType validity
  if (!VALID_DYNAMIC_COMPONENT_TYPES.has(type)) {
    errors.push({
      path: `${path}.type`,
      message: `Unknown component type "${type}", valid types: ${[...VALID_DYNAMIC_COMPONENT_TYPES].sort().join(", ")}`,
    });
  }

  // Validate props
  const props = obj.props;
  if (props !== undefined && (typeof props !== "object" || props === null)) {
    errors.push({
      path: `${path}.props`,
      message: "props must be an object type",
    });
  }

  // Props compatibility validation
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
          message: `Component "${type}" is missing required prop "${field}"`,
        });
      }
    }
  }

  // Shape validation (D-11): enhanced type checking, not just existence
  shapeValidateProps(type, props, path, errors);

  // Validate dataSource
  if (obj.dataSource !== undefined) {
    validateDataSource(obj.dataSource, `${path}.dataSource`, errors);
  }

  // Validate events
  if (Array.isArray(obj.events)) {
    for (let i = 0; i < obj.events.length; i++) {
      validateEventHandler(obj.events[i], `${path}.events[${i}]`, errors);
    }
  } else if (obj.events !== undefined) {
    errors.push({
      path: `${path}.events`,
      message: "events must be an array type",
    });
  }

  // Validate conditionalDisplay (supports both array and object forms)
  if (obj.conditionalDisplay !== undefined) {
    validateConditionalDisplay(
      obj.conditionalDisplay,
      `${path}.conditionalDisplay`,
      errors,
      depth + 1,
    );
  }

  // Validate semantics: importance
  if (obj.importance !== undefined) {
    if (!VALID_IMPORTANCE.includes(obj.importance as string)) {
      errors.push({
        path: `${path}.importance`,
        message: `Invalid importance "${String(obj.importance)}", valid: ${VALID_IMPORTANCE.join("|")}`,
      });
    }
  }

  // Validate semantics: status
  if (obj.status !== undefined) {
    if (!VALID_STATUS.includes(obj.status as string)) {
      errors.push({
        path: `${path}.status`,
        message: `Invalid status "${String(obj.status)}", valid: ${VALID_STATUS.join("|")}`,
      });
    }
  }

  // Validate semantics: fallback (recursively validates fallback schema)
  if (obj.fallback !== undefined) {
    if (typeof obj.fallback !== "object" || obj.fallback === null) {
      errors.push({ path: `${path}.fallback`, message: "fallback must be a UISchema object" });
    } else {
      validateNode(obj.fallback as UISchema, `${path}.fallback`, errors, depth + 1);
    }
  }

  // Recursively validate children
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
    errors.push({ path, message: "dataSource must be an object type" });
    return;
  }
  const obj = ds as Record<string, unknown>;
  const validTypes = ["store", "api", "static", "agent-generated"];
  if (!validTypes.includes(obj.type as string)) {
    errors.push({
      path: `${path}.type`,
      message: `Invalid data source type "${String(obj.type)}", valid types: ${validTypes.join(", ")}`,
    });
  }
  if (
    obj.config === undefined
    || (typeof obj.config !== "object" || obj.config === null)
  ) {
    errors.push({
      path: `${path}.config`,
      message: "dataSource.config must be an object type",
    });
    return;
  }

  // api 类型安全校验：endpoint 必须通过白名单（防任意 IPC / SSRF）
  if (obj.type === "api") {
    const cfg = obj.config as Record<string, unknown>;
    const method = cfg.method as string | undefined;
    const endpoint = cfg.endpoint;
    if (method !== "invoke" && method !== "fetch") {
      errors.push({
        path: `${path}.config.method`,
        message: `dataSource.config.method must be "invoke" or "fetch", got "${String(method)}"`,
      });
    } else if (typeof endpoint !== "string" || endpoint.length === 0) {
      errors.push({
        path: `${path}.config.endpoint`,
        message: "dataSource.config.endpoint must be a non-empty string",
      });
    } else if (
      (method === "invoke" && !isAllowedInvokeEndpoint(endpoint))
      || (method === "fetch" && !isAllowedFetchUrl(endpoint))
    ) {
      errors.push({
        path: `${path}.config.endpoint`,
        message: `dataSource.config.endpoint "${endpoint}" is not allowed (blocked for security)`,
      });
    }
  }
}

function validateEventHandler(
  handler: unknown,
  path: string,
  errors: SchemaValidationError[],
): void {
  if (typeof handler !== "object" || handler === null) {
    errors.push({ path, message: "EventHandler must be an object type" });
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
      message: `Invalid trigger "${String(obj.trigger)}", valid: ${validTriggers.join(", ")}`,
    });
  }
  if (!Array.isArray(obj.actions)) {
    errors.push({
      path: `${path}.actions`,
      message: "actions must be an array type",
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
      message: `conditionalDisplay nesting depth exceeds limit ${MAX_NESTING_DEPTH}`,
    });
    return;
  }

  if (Array.isArray(display)) {
    const arr = display as unknown[];
    if (arr.length === 0) {
      errors.push({ path, message: "conditionalDisplay array cannot be empty" });
      return;
    }
    for (let i = 0; i < arr.length; i++) {
      validateConditionalRule(arr[i], `${path}[${i}]`, errors);
    }
    return;
  }

  if (typeof display !== "object" || display === null) {
    errors.push({ path, message: "conditionalDisplay must be an array or object type" });
    return;
  }

  const obj = display as Record<string, unknown>;

  if (obj.logic !== "and" && obj.logic !== "or") {
    errors.push({
      path: `${path}.logic`,
      message: `conditionalDisplay.logic must be "and" or "or", got "${String(obj.logic)}"`,
    });
  }

  if (!Array.isArray(obj.rules) || obj.rules.length === 0) {
    errors.push({
      path: `${path}.rules`,
      message: "conditionalDisplay.rules must be a non-empty array",
    });
  } else {
    for (let i = 0; i < obj.rules.length; i++) {
      validateConditionalDisplay(obj.rules[i], `${path}.rules[${i}]`, errors, depth + 1);
    }
  }

  if (obj.not !== undefined && typeof obj.not !== "boolean") {
    errors.push({
      path: `${path}.not`,
      message: "conditionalDisplay.not must be a boolean type",
    });
  }
}

function validateConditionalRule(
  rule: unknown,
  path: string,
  errors: SchemaValidationError[],
): void {
  if (typeof rule !== "object" || rule === null) {
    errors.push({ path, message: "ConditionalRule must be an object type" });
    return;
  }
  const obj = rule as Record<string, unknown>;
  if (typeof obj.field !== "string" || obj.field.length === 0) {
    errors.push({ path: `${path}.field`, message: "Missing required field: field" });
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
      message: `Invalid operator "${String(obj.operator)}", valid: ${validOperators.join(", ")}`,
    });
  }
}

/** Known valid chartType values */
const VALID_CHART_TYPES = ["line", "bar", "pie", "scatter", "area"];

/** Component props shape validation rules: type/enum checks beyond existence (D-11) */
const PROPS_SHAPE_RULES: Record<
  string,
  Array<{
    field: string;
    check: (val: unknown) => string | null;
  }>
> = {
  Table: [
    { field: "columns", check: (v) => Array.isArray(v) ? null : "columns must be an array" },
  ],
  Chart: [
    {
      field: "chartType",
      check: (v) =>
        VALID_CHART_TYPES.includes(v as string)
          ? null
          : `chartType must be one of ${VALID_CHART_TYPES.join("|")}, got "${String(v)}"`,
    },
  ],
  Dashboard: [
    { field: "items", check: (v) => Array.isArray(v) ? null : "items must be an array" },
  ],
  Tree: [
    { field: "treeData", check: (v) => Array.isArray(v) ? null : "treeData must be an array" },
  ],
  Timeline: [
    { field: "items", check: (v) => Array.isArray(v) ? null : "items must be an array" },
  ],
  Grid: [
    { field: "columns", check: (v) => typeof v === "number" ? null : "columns must be a number" },
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
      continue; // Existence already validated by COMPONENT_REQUIRED_PROPS
    }
    const errMsg = check(val);
    if (errMsg) {
      errors.push({ path: `${path}.props.${field}`, message: errMsg });
    }
  }
}
