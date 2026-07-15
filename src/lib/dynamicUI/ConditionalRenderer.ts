// SPDX-License-Identifier: AGPL-3.0-only

import type { ConditionalDisplay, ConditionalRule } from "@/types";
import { getNestedValue } from "./utils";

function evaluateRule(
  rule: ConditionalRule,
  data: Record<string, unknown>,
): boolean {
  const fieldValue = getNestedValue(data, rule.field);

  switch (rule.operator) {
    case "eq":
      return fieldValue === rule.value;
    case "neq":
      return fieldValue !== rule.value;
    case "gt":
      return typeof fieldValue === "number" && typeof rule.value === "number"
        ? fieldValue > rule.value
        : false;
    case "gte":
      return typeof fieldValue === "number" && typeof rule.value === "number"
        ? fieldValue >= rule.value
        : false;
    case "lt":
      return typeof fieldValue === "number" && typeof rule.value === "number"
        ? fieldValue < rule.value
        : false;
    case "lte":
      return typeof fieldValue === "number" && typeof rule.value === "number"
        ? fieldValue <= rule.value
        : false;
    case "in":
      return Array.isArray(rule.value)
        ? (rule.value as unknown[]).includes(fieldValue)
        : false;
    case "contains":
      if (typeof fieldValue === "string" && typeof rule.value === "string") {
        return fieldValue.includes(rule.value);
      }
      if (Array.isArray(fieldValue)) {
        return fieldValue.includes(rule.value);
      }
      return false;
    case "exists":
      return fieldValue !== undefined && fieldValue !== null;
    case "empty":
      return fieldValue === undefined
        || fieldValue === null
        || fieldValue === ""
        || (Array.isArray(fieldValue) && fieldValue.length === 0);
    default:
      return true;
  }
}

function evaluateCondition(
  condition: ConditionalDisplay,
  data: Record<string, unknown>,
): boolean {
  if (Array.isArray(condition)) {
    return condition.every((rule) => evaluateRule(rule, data));
  }

  const { logic, rules, not } = condition;
  const results = rules.map((r: ConditionalDisplay) => evaluateCondition(r, data));
  let result: boolean;

  if (logic === "or") {
    result = results.some(Boolean);
  } else {
    result = results.every(Boolean);
  }

  return not ? !result : result;
}

export function evaluateConditions(
  conditions: ConditionalDisplay | undefined,
  data: Record<string, unknown>,
): boolean {
  if (!conditions) {
    return true;
  }
  return evaluateCondition(conditions, data);
}
