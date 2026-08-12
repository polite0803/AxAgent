// SPDX-License-Identifier: AGPL-3.0-only

import { validate_workflow, type ValidateIssue } from "@/lib/workflowLayout";
import { useEffect, useRef, useState } from "react";
import type { WorkflowEdge, WorkflowNode } from "../types";

/**
 * 封装前端工作流校验的 debounce 逻辑。
 *
 * 并发安全：
 * 1. 使用 setTimeout 做 debounce（300ms），避免每次 keystroke 都重算
 * 2. timerRef 确保前一个 timer 被清理，不会出现多次并行校验
 * 3. 返回的 validationMsgMap 用于节点样式着色，通过 React state 传递
 */
export interface ValidationController {
  issues: ValidateIssue[];
  msgMap: Map<string, string>;
}

export function useWorkflowValidation(
  nodes: WorkflowNode[],
  edges: WorkflowEdge[],
  renderFn: (key: string, params?: Record<string, unknown>) => string,
): ValidationController {
  const [issues, setIssues] = useState<ValidateIssue[]>([]);
  const [msgMap, setMsgMap] = useState<Map<string, string>>(new Map());
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (timerRef.current) { clearTimeout(timerRef.current); }
    timerRef.current = setTimeout(() => {
      const result = validate_workflow(nodes, edges, renderFn);
      setIssues(result.issues);
      const map = new Map<string, string>();
      for (const iss of result.issues) {
        for (const nid of iss.nodeIds) {
          const prev = map.get(nid);
          map.set(nid, prev ? `${prev}; ${iss.message}` : iss.message);
        }
      }
      setMsgMap(map);
    }, 300);
    return () => {
      if (timerRef.current) { clearTimeout(timerRef.current); }
    };
  }, [nodes, edges, renderFn]);

  return { issues, msgMap };
}
