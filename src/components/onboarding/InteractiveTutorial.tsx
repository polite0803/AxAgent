// 交互式教程 — 轻量步骤覆盖层
import { useOnboardingStore } from "@/stores";
import { Button, theme } from "antd";
import { ArrowRight, SkipForward, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import "./InteractiveTutorial.css";

interface TutorialStep {
  target: string;
  titleKey: string;
  descKey: string;
}

export function InteractiveTutorial() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const location = useLocation();
  const navigate = useNavigate();

  const steps: TutorialStep[] = useMemo(
    () => [
      {
        target: '[data-tutorial="chat-sidebar"]',
        titleKey: "onboarding.tutorialSidebarTitle",
        descKey: "onboarding.tutorialSidebarDesc",
      },
      {
        target: '[data-tutorial="chat-input"]',
        titleKey: "onboarding.tutorialInputTitle",
        descKey: "onboarding.tutorialInputDesc",
      },
      {
        target: '[data-tutorial="agent-mode"]',
        titleKey: "onboarding.tutorialAgentTitle",
        descKey: "onboarding.tutorialAgentDesc",
      },
      {
        target: '[data-tutorial="knowledge-nav"]',
        titleKey: "onboarding.tutorialKnowledgeTitle",
        descKey: "onboarding.tutorialKnowledgeDesc",
      },
    ],
    [],
  );

  const tutorialActive = useOnboardingStore((s) => s.tutorialActive);
  const tutorialStep = useOnboardingStore((s) => s.tutorialStep);
  const nextTutorialStep = useOnboardingStore((s) => s.nextTutorialStep);
  const skipTutorial = useOnboardingStore((s) => s.skipTutorial);
  const completeTutorial = useOnboardingStore((s) => s.completeTutorial);
  const startTutorial = useOnboardingStore((s) => s.startTutorial);
  const tutorialCompleted = useOnboardingStore((s) => s.tutorialCompleted);

  // 开始教程时自动导航到聊天页（带重试）
  const handleStartTutorial = () => {
    if (location.pathname !== "/") { navigate("/"); }
    // 重试等待目标元素渲染（最多 5 次，每次 200ms）
    let retries = 0;
    const tryStart = () => {
      const el = document.querySelector('[data-tutorial="chat-sidebar"]');
      if (el || retries >= 5) {
        startTutorial();
      } else {
        retries++;
        setTimeout(tryStart, 200);
      }
    };
    setTimeout(tryStart, 200);
  };

  const [spotlight, setSpotlight] = useState<DOMRect | null>(null);
  const [bubblePosition, setBubblePosition] = useState<{ top: number; left: number } | null>(null);
  const rafId = useRef<number>(0);
  const bubbleRef = useRef<HTMLDivElement>(null);
  const prevFocusRef = useRef<HTMLElement | null>(null);

  const step = steps[tutorialStep];
  const isLast = tutorialStep >= steps.length - 1;

  // 定位到目标元素
  useEffect(() => {
    if (!tutorialActive || !step) { return; }
    const updatePos = () => {
      const el = document.querySelector(step.target);
      if (el) {
        const r = el.getBoundingClientRect();
        setSpotlight(r);
        const estimatedHeight = 200;
        let top: number;
        if (r.bottom + 12 + estimatedHeight > window.innerHeight) {
          top = r.top - 12 - estimatedHeight;
        } else {
          top = r.bottom + 12;
        }
        let left = Math.max(12, r.left);
        if (left + 300 > window.innerWidth) {
          left = window.innerWidth - 312;
        }
        setBubblePosition({ top, left });
      } else {
        setSpotlight(null);
        setBubblePosition(null);
      }
    };
    updatePos();
    rafId.current = requestAnimationFrame(function loop() {
      updatePos();
      rafId.current = requestAnimationFrame(loop);
    });
    return () => cancelAnimationFrame(rafId.current);
  }, [tutorialActive, tutorialStep, step]);

  const handleNext = () => {
    if (isLast) { completeTutorial(); }
    else { nextTutorialStep(); }
  };

  useEffect(() => {
    if (tutorialStep === 3) {
      navigate("/knowledge");
    }
  }, [tutorialStep, navigate]);

  useEffect(() => {
    if (!tutorialActive) { return; }
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Enter") {
        handleNext();
      } else if (e.key === "Escape") {
        skipTutorial();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [tutorialActive, handleNext, skipTutorial]);

  useEffect(() => {
    if (!tutorialActive) {
      if (prevFocusRef.current) {
        prevFocusRef.current.focus();
        prevFocusRef.current = null;
      }
      return;
    }
    prevFocusRef.current = document.activeElement as HTMLElement;
    const timer = setTimeout(() => {
      bubbleRef.current?.focus();
    }, 50);
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Tab" || !bubbleRef.current) { return; }
      const focusable = bubbleRef.current.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
      );
      if (focusable.length === 0) { return; }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (e.shiftKey) {
        if (document.activeElement === first) {
          e.preventDefault();
          last.focus();
        }
      } else {
        if (document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      clearTimeout(timer);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [tutorialActive]);

  if (!tutorialActive && tutorialCompleted) { return null; }

  // 开始按钮（向导完成后显示）
  if (!tutorialActive) {
    return (
      <div className="tutorial-start-bar">
        <Button type="link" size="small" onClick={handleStartTutorial}>
          {t("onboarding.tutorialStart")}
        </Button>
      </div>
    );
  }

  const bubbleStyle: React.CSSProperties = bubblePosition
    ? {
      position: "fixed",
      top: bubblePosition.top,
      left: bubblePosition.left,
      zIndex: 10001,
      width: 300,
    }
    : {
      position: "fixed",
      top: "50%",
      left: "50%",
      transform: "translate(-50%, -50%)",
      zIndex: 10001,
      width: 300,
    };

  return createPortal(
    <>
      {/* 遮罩 */}
      <div className="tutorial-overlay" role="presentation" onClick={skipTutorial} />

      {/* 高亮槽 */}
      {spotlight && (
        <div
          className="tutorial-spotlight"
          style={{
            position: "fixed",
            top: spotlight.top - 6,
            left: spotlight.left - 6,
            width: spotlight.width + 12,
            height: spotlight.height + 12,
            zIndex: 10000,
            borderRadius: 8,
            boxShadow: `0 0 0 9999px ${token.colorBgMask}`,
          }}
        />
      )}

      {/* 提示气泡 */}
      <div
        ref={bubbleRef}
        tabIndex={-1}
        className="tutorial-bubble"
        style={{
          ...bubbleStyle,
          background: token.colorBgElevated,
          border: `1px solid ${token.colorBorderSecondary}`,
          borderRadius: token.borderRadiusLG,
          padding: 16,
          boxShadow: token.boxShadowSecondary,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", marginBottom: 8 }}>
          <div style={{ flex: 1 }}>
            <strong style={{ fontSize: 14 }}>{step ? t(step.titleKey) : ""}</strong>
            <div style={{ fontSize: 12, color: token.colorTextSecondary, marginTop: 4 }}>
              {step
                ? (!spotlight
                  ? t("onboarding.targetNotFound")
                  : t(step.descKey))
                : ""}
            </div>
          </div>
          <button
            type="button"
            onClick={skipTutorial}
            style={{
              border: "none",
              background: "none",
              cursor: "pointer",
              color: token.colorTextQuaternary,
              padding: 0,
            }}
          >
            <X size={14} />
          </button>
        </div>

        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <Button size="small" type="link" onClick={skipTutorial} icon={<SkipForward size={12} />}>
            {t("onboarding.tutorialSkip")}
          </Button>
          <div style={{ fontSize: 12, color: token.colorTextQuaternary }}>
            {tutorialStep + 1} / {steps.length}
          </div>
          <Button size="small" type="primary" onClick={handleNext} icon={<ArrowRight size={12} />}>
            {isLast
              ? t("onboarding.done")
              : t("onboarding.next")}
          </Button>
        </div>
      </div>
    </>,
    document.body,
  );
}
