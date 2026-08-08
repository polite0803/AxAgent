// SPDX-License-Identifier: AGPL-3.0-only

import type { IntentClarification, IntentState } from "@/types";
import { useCallback, useRef, useState } from "react";

const INITIAL_CLARIFICATION: IntentClarification = {
  state: "draft",
  original_input: "",
  clarification_questions: [],
  clarification_answers: {},
  created_at: 0,
  updated_at: 0,
};

function createTimestamp(): number {
  return Date.now();
}

function createDraft(input: string): IntentClarification {
  const now = createTimestamp();
  return {
    ...INITIAL_CLARIFICATION,
    original_input: input,
    created_at: now,
    updated_at: now,
  };
}

export interface UseIntentClarificationReturn {
  /** 当前澄清状态 */
  clarification: IntentClarification | null;
  /** 是否处于活跃澄清流程中 */
  isActive: boolean;
  /** 当前状态 */
  state: IntentState | null;
  /** 关联的工作流执行 ID */
  workflowExecutionId: string | undefined;
  /** 开始澄清流程（接收原始输入，进入 draft 状态） */
  start: (input: string) => void;
  /** 设置澄清问题列表（进入 clarifying 状态） */
  setQuestions: (questions: string[]) => void;
  /** 回答澄清问题 */
  answerQuestion: (questionId: string, answer: string) => void;
  /** 请求用户确认（进入 needs_confirmation 状态） */
  requestConfirmation: (summary: string, options?: string[]) => void;
  /** 确认意图（进入 submitted 状态） */
  confirm: () => void;
  /** 取消澄清流程 */
  cancel: () => void;
  /** 重置为初始状态 */
  reset: () => void;
  /** 设置工作流执行 ID */
  setWorkflowExecutionId: (executionId: string) => void;
  /** AI 生成意图摘要 */
  setIntentSummary: (summary: string) => void;
}

/**
 * 意图澄清状态机 hook
 *
 * 状态流转：
 * draft → clarifying → needs_confirmation → submitted
 *                  ↓              ↓
 *               cancelled      cancelled
 *
 * 设计哲学：在一切自动化里，人的注意力都是最稀缺的资源。
 * 系统应当尽量少地占用它——通过澄清流程在动手前消除模糊地带。
 */
export function useIntentClarification(): UseIntentClarificationReturn {
  const [clarification, setClarification] = useState<IntentClarification | null>(null);
  const clarificationRef = useRef<IntentClarification | null>(null);

  const update = useCallback((updater: (prev: IntentClarification) => IntentClarification) => {
    setClarification((prev) => {
      if (!prev) { return prev; }
      const next = updater(prev);
      clarificationRef.current = next;
      return { ...next, updated_at: createTimestamp() };
    });
  }, []);

  const transitionTo = useCallback(
    (newState: IntentState, mutator?: (prev: IntentClarification) => IntentClarification) => {
      setClarification((prev) => {
        if (!prev) { return prev; }
        let next: IntentClarification = { ...prev, state: newState };
        if (mutator) {
          next = mutator(next);
        }
        clarificationRef.current = next;
        return { ...next, updated_at: createTimestamp() };
      });
    },
    [],
  );

  const start = useCallback(
    (input: string) => {
      const draft = createDraft(input);
      clarificationRef.current = draft;
      setClarification(draft);
    },
    [],
  );

  const setQuestions = useCallback(
    (questions: string[]) => {
      transitionTo("clarifying", (prev) => ({
        ...prev,
        clarification_questions: questions,
      }));
    },
    [transitionTo],
  );

  const answerQuestion = useCallback(
    (questionId: string, answer: string) => {
      setClarification((prev) => {
        if (!prev) { return prev; }
        const nextAnswers = {
          ...prev.clarification_answers,
          [questionId]: answer,
        };
        const next: IntentClarification = {
          ...prev,
          clarification_answers: nextAnswers,
        };

        // 如果所有问题都已回答完毕，自动流转到 needs_confirmation
        const allAnswered = prev.clarification_questions.length > 0
          && prev.clarification_questions.every((q) => nextAnswers[q] !== undefined);

        if (allAnswered) {
          const summary = prev.clarification_questions
            .map((q) => `${q}: ${nextAnswers[q]}`)
            .join("\n");
          next.state = "needs_confirmation";
          next.intent_summary = summary;
        }

        clarificationRef.current = next;
        return { ...next, updated_at: createTimestamp() };
      });
    },
    [],
  );

  const requestConfirmation = useCallback(
    (summary: string, options?: string[]) => {
      transitionTo("needs_confirmation", (prev) => ({
        ...prev,
        intent_summary: summary,
        confirmation_options: options,
      }));
    },
    [transitionTo],
  );

  const confirm = useCallback(() => {
    transitionTo("submitted", (prev) => ({
      ...prev,
      confirmed_intent: prev.intent_summary ?? prev.original_input,
    }));
  }, [transitionTo]);

  const cancel = useCallback(() => {
    transitionTo("cancelled");
  }, [transitionTo]);

  const reset = useCallback(() => {
    clarificationRef.current = null;
    setClarification(null);
  }, []);

  const setWorkflowExecutionId = useCallback(
    (executionId: string) => {
      update((prev) => ({
        ...prev,
        workflow_execution_id: executionId,
      }));
    },
    [update],
  );

  const setIntentSummary = useCallback(
    (summary: string) => {
      update((prev) => ({
        ...prev,
        intent_summary: summary,
      }));
    },
    [update],
  );

  return {
    clarification,
    isActive: clarification !== null && clarification.state !== "submitted" && clarification.state !== "cancelled",
    state: clarification?.state ?? null,
    workflowExecutionId: clarification?.workflow_execution_id,
    start,
    setQuestions,
    answerQuestion,
    requestConfirmation,
    confirm,
    cancel,
    reset,
    setWorkflowExecutionId,
    setIntentSummary,
  };
}
