// SPDX-License-Identifier: AGPL-3.0-only

// 新用户欢迎向导 — 5 步引导流程
import { useOnboardingStore } from "@/stores";
import { Button, Card, Modal, Steps, Tag, theme, Typography } from "antd";
import {
  ArrowRight,
  Bot,
  CheckCircle2,
  Cpu,
  Download,
  Globe,
  Key,
  MessageSquare,
  Search,
  Sparkles,
  Zap,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import "./WelcomeWizard.css";

const { Title, Text, Paragraph } = Typography;

export function WelcomeWizard() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { token } = theme.useToken();

  const wizardCompleted = useOnboardingStore((s) => s.wizardCompleted);
  const wizardDismissed = useOnboardingStore((s) => s.wizardDismissed);
  const currentStep = useOnboardingStore((s) => s.currentStep);
  const ollamaAvailable = useOnboardingStore((s) => s.ollamaAvailable);
  const ollamaModels = useOnboardingStore((s) => s.ollamaModels);
  const detectedKeys = useOnboardingStore((s) => s.detectedKeys);
  const selectedPreset = useOnboardingStore((s) => s.selectedPreset);

  const setStep = useOnboardingStore((s) => s.setStep);
  const dismissWizard = useOnboardingStore((s) => s.dismissWizard);
  const completeWizard = useOnboardingStore((s) => s.completeWizard);
  const detectOllama = useOnboardingStore((s) => s.detectOllama);
  const detectKeys = useOnboardingStore((s) => s.detectKeys);
  const applyPreset = useOnboardingStore((s) => s.applyPreset);
  const startTutorial = useOnboardingStore((s) => s.startTutorial);

  const [applying, setApplying] = useState(false);
  const [presetMsg, setPresetMsg] = useState("");

  // 自动检测
  useEffect(() => {
    void detectOllama();
    void detectKeys();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleApplyPreset = async (preset: string) => {
    if (applying) {
      return;
    }
    setApplying(true);
    setPresetMsg("");
    try {
      const msg = await applyPreset(preset);
      setPresetMsg(msg);
    } finally {
      setApplying(false);
    }
  };

  const presets = [
    {
      key: "ollama",
      icon: Cpu,
      title: t("onboarding.presetOllama"),
      desc: t("onboarding.presetOllamaDesc"),
      color: "#52c41a",
    },
    {
      key: "openai",
      icon: Globe,
      title: t("onboarding.presetOpenAI"),
      desc: t("onboarding.presetOpenAIDesc"),
      color: "#1890ff",
    },
    {
      key: "minimal",
      icon: Download,
      title: t("onboarding.presetMinimal"),
      desc: t("onboarding.presetMinimalDesc"),
      color: "#fa8c16",
    },
  ];

  const steps = [
    {
      title: t("onboarding.stepWelcome"),
      icon: <Sparkles size={18} />,
    },
    {
      title: t("onboarding.stepDetect"),
      icon: <Search size={18} />,
    },
    {
      title: t("onboarding.stepPreset"),
      icon: <Zap size={18} />,
    },
    {
      title: t("onboarding.stepOverview"),
      icon: <Bot size={18} />,
    },
    {
      title: t("onboarding.stepReady"),
      icon: <CheckCircle2 size={18} />,
    },
  ];

  const visible = !wizardCompleted && !wizardDismissed;

  return (
    <Modal
      open={visible}
      closable
      onCancel={dismissWizard}
      footer={null}
      width={560}
      centered
      className="welcome-wizard"
    >
      <Steps
        current={currentStep}
        size="small"
        style={{ marginBottom: 24 }}
        items={steps.map((s) => ({ title: s.title }))}
      />

      {/* Step 0: 欢迎 */}
      {currentStep === 0 && (
        <div className="wizard-step">
          <div className="wizard-hero">
            <Sparkles size={48} style={{ color: token.colorPrimary }} />
          </div>
          <Title level={3} style={{ textAlign: "center", marginTop: 16 }}>
            {t("onboarding.welcome")}
          </Title>
          <Paragraph
            type="secondary"
            style={{
              textAlign: "center",
              maxWidth: 400,
              margin: "8px auto 24px",
            }}
          >
            {t("onboarding.welcomeDesc")}
          </Paragraph>
        </div>
      )}

      {/* Step 1: 环境检测 */}
      {currentStep === 1 && (
        <div className="wizard-step">
          <Title level={4}>
            <Search size={18} style={{ marginRight: 6 }} />
            {t("onboarding.stepDetect")}
          </Title>

          <Card size="small" style={{ marginBottom: 8 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Cpu
                size={16}
                style={{
                  color: ollamaAvailable
                    ? "#52c41a"
                    : token.colorTextQuaternary,
                }}
              />
              <Text>
                {ollamaAvailable
                  ? t("onboarding.ollamaDetected", {
                    count: ollamaModels.length,
                  })
                  : t("onboarding.ollamaNotFound")}
              </Text>
            </div>
            {ollamaModels.length > 0 && (
              <div style={{ marginTop: 8, paddingLeft: 24 }}>
                {ollamaModels.slice(0, 5).map((m) => (
                  <Tag key={m.name} style={{ marginBottom: 4 }}>
                    {m.name}
                  </Tag>
                ))}
                {ollamaModels.length > 5 && (
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    {t("onboarding.moreModels", {
                      count: ollamaModels.length - 5,
                    })}
                  </Text>
                )}
              </div>
            )}
          </Card>

          <Card size="small">
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Key
                size={16}
                style={{
                  color: detectedKeys.length > 0
                    ? "#52c41a"
                    : token.colorTextQuaternary,
                }}
              />
              <Text>
                {detectedKeys.length > 0
                  ? t("onboarding.keysDetected", {
                    count: detectedKeys.length,
                  })
                  : t("onboarding.noKeysDetected")}
              </Text>
            </div>
            {detectedKeys.map((k, _i) => (
              <div
                key={k.envVar}
                style={{ paddingLeft: 24, marginTop: 4, fontSize: 12 }}
              >
                <Text type="secondary">
                  {k.providerType}: {k.prefix}
                </Text>
              </div>
            ))}
          </Card>
        </div>
      )}

      {/* Step 2: 快速预设 */}
      {currentStep === 2 && (
        <div className="wizard-step">
          <Title level={4}>
            <Zap size={18} style={{ marginRight: 6 }} />
            {t("onboarding.stepPreset")}
          </Title>

          <div style={{ display: "flex", gap: 12, flexDirection: "column" }}>
            {presets.map((p) => (
              <Card
                key={p.key}
                size="small"
                hoverable
                onClick={() => handleApplyPreset(p.key)}
                style={{
                  cursor: "pointer",
                  borderColor: selectedPreset === p.key ? p.color : undefined,
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                  <p.icon size={28} style={{ color: p.color }} />
                  <div style={{ flex: 1 }}>
                    <Text strong>{p.title}</Text>
                    <br />
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      {p.desc}
                    </Text>
                  </div>
                  {selectedPreset === p.key && <CheckCircle2 size={18} style={{ color: p.color }} />}
                  {applying && selectedPreset === p.key && (
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      ...
                    </Text>
                  )}
                </div>
              </Card>
            ))}
          </div>
          {presetMsg && (
            <Text
              type="secondary"
              style={{
                display: "block",
                marginTop: 12,
                fontSize: 12,
                textAlign: "center",
              }}
            >
              {presetMsg}
            </Text>
          )}
          {selectedPreset === "openai" && detectedKeys.length === 0 && (
            <div
              style={{
                display: "flex",
                justifyContent: "center",
                marginTop: 8,
              }}
            >
              <Button
                size="small"
                type="link"
                onClick={() => {
                  completeWizard();
                  navigate("/settings");
                }}
              >
                {t("onboarding.goToSettings")}
              </Button>
            </div>
          )}
        </div>
      )}

      {/* Step 3: 功能概览 */}
      {currentStep === 3 && (
        <div className="wizard-step">
          <Title level={4}>
            <Bot size={18} style={{ marginRight: 6 }} />
            {t("onboarding.stepOverview")}
          </Title>

          <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
            {[
              {
                icon: MessageSquare,
                title: t("onboarding.featureChat"),
                desc: t("onboarding.featureChatDesc"),
              },
              {
                icon: Bot,
                title: t("onboarding.featureAgent"),
                desc: t("onboarding.featureAgentDesc"),
              },
              {
                icon: Search,
                title: t("onboarding.featureKnowledge"),
                desc: t("onboarding.featureKnowledgeDesc"),
              },
            ].map((f) => (
              <Card
                key={f.title}
                size="small"
                style={{
                  flex: "1 1 140px",
                  minWidth: 140,
                  textAlign: "center",
                }}
              >
                <f.icon
                  size={24}
                  style={{ color: token.colorPrimary, marginBottom: 8 }}
                />
                <br />
                <Text strong style={{ fontSize: 13 }}>
                  {f.title}
                </Text>
                <br />
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {f.desc}
                </Text>
              </Card>
            ))}
          </div>
        </div>
      )}

      {/* Step 4: 就绪 */}
      {currentStep === 4 && (
        <div className="wizard-step" style={{ textAlign: "center" }}>
          <CheckCircle2
            size={56}
            style={{ color: "#52c41a", marginBottom: 16 }}
          />
          <Title level={3}>{t("onboarding.ready")}</Title>
          <Paragraph type="secondary">
            {selectedPreset
              ? t("onboarding.readyDesc")
              : t("onboarding.readyNoPreset")}
          </Paragraph>
          <div
            style={{
              display: "flex",
              gap: 12,
              justifyContent: "center",
              marginTop: 8,
            }}
          >
            <Button
              type="primary"
              size="large"
              icon={<ArrowRight size={16} />}
              onClick={completeWizard}
            >
              {t("onboarding.startUsing")}
            </Button>
            <Button
              size="large"
              onClick={() => {
                completeWizard();
                startTutorial();
              }}
            >
              {t("onboarding.tutorialStart")}
            </Button>
          </div>
        </div>
      )}

      {/* 底部导航 */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          marginTop: 24,
        }}
      >
        <Button onClick={dismissWizard}>{t("onboarding.skip")}</Button>
        <div style={{ display: "flex", gap: 8 }}>
          {currentStep > 0 && (
            <Button onClick={() => setStep(currentStep - 1)}>
              {t("onboarding.previous")}
            </Button>
          )}
          {currentStep < 4 && (
            <Button type="primary" onClick={() => setStep(currentStep + 1)}>
              {t("onboarding.next")}
            </Button>
          )}
        </div>
      </div>
    </Modal>
  );
}
