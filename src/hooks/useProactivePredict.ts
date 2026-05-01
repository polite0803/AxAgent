import { useConversationStore } from "@/stores";
import { invoke } from "@/lib/invoke";
import { useEffect, useRef } from "react";

/**
 * 监听用户发送消息后，触发主动预测。
 */
export function useProactivePredict() {
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastMsgIdRef = useRef<string>("");

  useEffect(() => {
    const unsub = useConversationStore.subscribe((state, prev) => {
      const activeId = state.activeConversationId;
      if (!activeId) return;
      if (state.messages === prev.messages) return;
      if (state.messages.length === prev.messages.length) return;

      const lastMsg = state.messages[state.messages.length - 1];
      if (lastMsg.role !== "user") return;
      if (lastMsg.id === lastMsgIdRef.current) return;
      lastMsgIdRef.current = lastMsg.id;

      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => {
        invoke("proactive_predict", {
          context: {
            conversationId: activeId,
            recentMessageCount: state.messages.length,
          },
        }).catch(() => {});
      }, 2000);
    });

    return () => {
      unsub();
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);
}
