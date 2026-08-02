// SPDX-License-Identifier: AGPL-3.0-only

import { motion } from "framer-motion";
import { useMemo } from "react";

/**
 * 逐词浮现文本动画 —— 动画效果预览演示组件。
 *
 * 设计要点：
 * - 纯 framer-motion 实现（项目已有依赖，无新增依赖）
 * - 仅在动画启用时通过 lazy() 加载（见 AnimationSettings）
 * - 静态降级由消费方控制：动画关闭时渲染纯文本，不加载本组件
 */

interface AnimatedPreviewProps {
  text: string;
}

/** 将文本按空格拆分为词（保留空格作为词的一部分，避免单词粘连） */
function splitWords(text: string): string[] {
  return text.split(/(\s+)/).filter((part) => part.length > 0);
}

export function AnimatedPreview({ text }: AnimatedPreviewProps) {
  const words = useMemo(() => splitWords(text), [text]);

  return (
    <motion.span
      initial="hidden"
      animate="visible"
      transition={{ staggerChildren: 0.06 }}
      style={{
        display: "inline-block",
        fontSize: 16,
        fontWeight: 500,
      }}
      aria-label={text}
    >
      {words.map((word, index) => (
        <motion.span
          key={`${word}-${index}`}
          variants={{
            hidden: { opacity: 0, y: 10, filter: "blur(4px)" },
            visible: {
              opacity: 1,
              y: 0,
              filter: "blur(0px)",
              transition: { duration: 0.45, ease: "easeOut" },
            },
          }}
          style={{ display: "inline-block", whiteSpace: "pre" }}
          aria-hidden="true"
        >
          {word}
        </motion.span>
      ))}
    </motion.span>
  );
}
