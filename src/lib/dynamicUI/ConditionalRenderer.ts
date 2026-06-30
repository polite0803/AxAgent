// SPDX-License-Identifier: AGPL-3.0-only

import type { ConditionalRule } from "@/types";

/**
 * 条件渲染引擎：根据 ConditionalRule 数组判断组件是否应该渲染。
 *
 * 支持 8 种操作符：eq, neq, gt, gte, lt, lte, in, contains
 * 所有规则必须同时满足（AND 逻辑）才返回 true。
 */

/**
 * 评估条件规则数组。
 * @param rules 条件规则数组
 * @param data 数据上下文
 * @returns 是否所有规则都满足（空数组视为满足）
 */
export function evaluateConditions(
  rules: ConditionalRule[],
  data: Record<string, unknown>,
): boolean {
  if (rules.length === 0) {
    return true;
  }

  for (const rule of rules) {
    if (!evaluateSingleRule(rule, data)) {
      return false;
    }
  }

  return true;
}

function evaluateSingleRule(
  rule: ConditionalRule,
  data: Record<string, unknown>,
): boolean {
  const fieldValue = getNestedValue(data, rule.field);
  const compareValue = rule.value;

  switch (rule.operator) {
    case "eq":
      return fieldValue === compareValue;

    case "neq":
      return fieldValue !== compareValue;

    case "gt":
      return compareNumbers(fieldValue, compareValue) > 0;

    case "gte":
      return compareNumbers(fieldValue, compareValue) >= 0;

    case "lt":
      return compareNumbers(fieldValue, compareValue) < 0;

    case "lte":
      return compareNumbers(fieldValue, compareValue) <= 0;

    case "in":
      return isInArray(fieldValue, compareValue);

    case "contains":
      return containsValue(fieldValue, compareValue);

    default:
      return false;
  }
}

function getNestedValue(
  obj: Record<string, unknown>,
  path: string,
): unknown {
  const keys = path.split(".");
  let current: unknown = obj;
  for (const key of keys) {
    if (current === null || current === undefined) {
      return undefined;
    }
    if (typeof current !== "object") {
      return undefined;
    }
    current = (current as Record<string, unknown>)[key];
  }
  return current;
}

function compareNumbers(a: unknown, b: unknown): number {
  const numA = Number(a);
  const numB = Number(b);
  if (isNaN(numA) || isNaN(numB)) {
    return 0;
  }
  return numA - numB;
}

function isInArray(value: unknown, arrayValue: unknown): boolean {
  if (!Array.isArray(arrayValue)) {
    return false;
  }
  return arrayValue.includes(value);
}

function containsValue(value: unknown, searchValue: unknown): boolean {
  // 字符串包含
  if (typeof value === "string" && typeof searchValue === "string") {
    return value.includes(searchValue);
  }
  // 数组包含
  if (Array.isArray(value)) {
    return value.includes(searchValue);
  }
  return false;
}
