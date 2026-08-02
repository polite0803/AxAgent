// SPDX-License-Identifier: AGPL-3.0-only
// 移植自 react-bits (https://github.com/DavidHDev/react-bits) — GradientText
// 原组件 MIT + Commons Clause 双许可；已按项目规范适配：命名导出、framer-motion import、CSS 内联

import { motion, useAnimationFrame, useMotionValue, useTransform } from "framer-motion";
import { type CSSProperties, type ReactNode, useCallback, useEffect, useRef, useState } from "react";

interface GradientTextProps {
  children: ReactNode;
  className?: string;
  colors?: string[];
  animationSpeed?: number;
  showBorder?: boolean;
  direction?: "horizontal" | "vertical" | "diagonal";
  pauseOnHover?: boolean;
  yoyo?: boolean;
}

/**
 * 流动渐变文本（多色循环渐变）。
 * 源码移植自 react-bits，已按 AxAgent 规范适配：命名导出 + framer-motion import + CSS 内联。
 * 配合 animationStore 使用：动画关闭时由消费方渲染静态文本。
 */
export function GradientText({
  children,
  className = "",
  colors = ["#5227FF", "#FF9FFC", "#B497CF"],
  animationSpeed = 8,
  showBorder = false,
  direction = "horizontal",
  pauseOnHover = false,
  yoyo = true,
}: GradientTextProps) {
  const [isPaused, setIsPaused] = useState(false);
  const progress = useMotionValue(0);
  const elapsedRef = useRef(0);
  const lastTimeRef = useRef<number | null>(null);

  const animationDuration = animationSpeed * 1000;

  useAnimationFrame((time) => {
    if (isPaused) {
      lastTimeRef.current = null;
      return;
    }

    if (lastTimeRef.current === null) {
      lastTimeRef.current = time;
      return;
    }

    const deltaTime = time - lastTimeRef.current;
    lastTimeRef.current = time;
    elapsedRef.current += deltaTime;

    if (yoyo) {
      const fullCycle = animationDuration * 2;
      const cycleTime = elapsedRef.current % fullCycle;

      if (cycleTime < animationDuration) {
        progress.set((cycleTime / animationDuration) * 100);
      } else {
        progress.set(100 - ((cycleTime - animationDuration) / animationDuration) * 100);
      }
    } else {
      // Continuously increase position for seamless looping
      progress.set((elapsedRef.current / animationDuration) * 100);
    }
  });

  useEffect(() => {
    elapsedRef.current = 0;
    progress.set(0);
  }, [animationSpeed, yoyo]);

  const backgroundPosition = useTransform(progress, (p) => {
    if (direction === "horizontal") {
      return `${p}% 50%`;
    } else if (direction === "vertical") {
      return `50% ${p}%`;
    }
    // For diagonal, move only horizontally to avoid interference patterns
    return `${p}% 50%`;
  });

  const handleMouseEnter = useCallback(() => {
    if (pauseOnHover) {
      setIsPaused(true);
    }
  }, [pauseOnHover]);

  const handleMouseLeave = useCallback(() => {
    if (pauseOnHover) {
      setIsPaused(false);
    }
  }, [pauseOnHover]);

  const gradientAngle = direction === "horizontal"
    ? "to right"
    : direction === "vertical"
    ? "to bottom"
    : "to bottom right";
  // Duplicate first color at the end for seamless looping
  const gradientColors = [...colors, colors[0]].join(", ");

  const gradientStyle: CSSProperties = {
    backgroundImage: `linear-gradient(${gradientAngle}, ${gradientColors})`,
    backgroundSize: direction === "horizontal"
      ? "300% 100%"
      : direction === "vertical"
      ? "100% 300%"
      : "300% 300%",
    backgroundRepeat: "repeat",
  };

  return (
    <motion.div
      className={`animated-gradient-text${showBorder ? " with-border" : ""} ${className}`}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      style={{
        position: "relative",
        display: "flex",
        maxWidth: "fit-content",
        alignItems: "center",
        justifyContent: "center",
        borderRadius: "1.25rem",
        fontWeight: 500,
        overflow: "hidden",
        ...(showBorder ? { padding: 1 } : {}),
      }}
    >
      {showBorder && (
        <motion.div
          aria-hidden="true"
          style={{
            position: "absolute",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            ...gradientStyle,
            backgroundPosition,
            borderRadius: "inherit",
            zIndex: 0,
            pointerEvents: "none",
          }}
        />
      )}
      <motion.div
        className="text-content"
        style={{
          display: "inline-block",
          position: "relative",
          zIndex: 2,
          ...gradientStyle,
          backgroundPosition,
          backgroundClip: "text",
          WebkitBackgroundClip: "text",
          color: "transparent",
          ...(showBorder
            ? { backgroundColor: "#120F17", padding: "0.5em 1em", borderRadius: "calc(1.25rem - 1px)" }
            : {}),
        }}
      >
        {children}
      </motion.div>
    </motion.div>
  );
}
