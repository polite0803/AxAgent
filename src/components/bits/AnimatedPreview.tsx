// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useState } from "react";
import { BlurText } from "./BlurText";
import { GradientText } from "./GradientText";
import { ShinyText } from "./ShinyText";

/**
 * 动画效果预览 —— 循环展示 react-bits 移植组件的实际效果。
 *
 * 设计要点：
 * - 展示 3 个真实 react-bits 组件：BlurText（逐字浮现）/ GradientText（流动渐变）/ ShinyText（光泽扫过）
 * - 仅在动画启用时通过 lazy() 加载（见 AnimationSettings）
 * - 静态降级由消费方控制：动画关闭时渲染纯文本，不加载本组件
 */

interface AnimatedPreviewProps {
  text: string;
}

type PreviewEffect = "blur" | "gradient" | "shiny";

const EFFECTS: PreviewEffect[] = ["blur", "gradient", "shiny"];

const EFFECT_LABELS: Record<PreviewEffect, string> = {
  blur: "BlurText",
  gradient: "GradientText",
  shiny: "ShinyText",
};

export function AnimatedPreview({ text }: AnimatedPreviewProps) {
  const [effectIndex, setEffectIndex] = useState(0);

  // 每 4 秒切换一次预览效果
  useEffect(() => {
    const timer = setInterval(() => {
      setEffectIndex((prev) => (prev + 1) % EFFECTS.length);
    }, 4000);
    return () => clearInterval(timer);
  }, []);

  const effect = EFFECTS[effectIndex];

  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 10 }}>
      <span style={{ fontSize: 11, opacity: 0.6 }}>{EFFECT_LABELS[effect]}</span>
      <div style={{ minHeight: 32, display: "flex", alignItems: "center", justifyContent: "center" }}>
        {effect === "blur" && <BlurText text={text} animateBy="words" delay={120} />}
        {effect === "gradient" && <GradientText>{text}</GradientText>}
        {effect === "shiny" && <ShinyText text={text} speed={3} />}
      </div>
    </div>
  );
}
