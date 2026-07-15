// SPDX-License-Identifier: AGPL-3.0-only

import { useVoiceChat } from "@/hooks/useVoiceChat";
import type { RealtimeConfig, VoiceSessionState } from "@/types";
import { Button, Spin, theme, Typography } from "antd";
import { Loader, Mic, MicOff, Phone, Volume2 } from "lucide-react";
import { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";

interface VoiceCallProps {
  visible: boolean;
  onClose: () => void;
  port?: number;
  host?: string;
  config: RealtimeConfig;
  apiKey: string;
}

function StatusDisplay({ state }: { state: VoiceSessionState }) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const textColor = token.colorWhite;

  const content = useMemo(() => {
    switch (state) {
      case "Connecting":
        return (
          <div className="flex flex-col items-center gap-4">
            <Spin
              indicator={
                <Loader
                  size={48}
                  style={{
                    color: textColor,
                    animation: "spin 1s linear infinite",
                  }}
                />
              }
            />
            <Typography.Text style={{ color: textColor, fontSize: 18 }}>
              {t("voice.connecting")}
            </Typography.Text>
          </div>
        );
      case "Connected":
        return (
          <div className="flex flex-col items-center gap-4">
            <Mic size={48} style={{ color: token.colorSuccess }} />
            <Typography.Text style={{ color: textColor, fontSize: 18 }}>
              {t("voice.connected")}
            </Typography.Text>
          </div>
        );
      case "Speaking":
        return (
          <div className="flex flex-col items-center gap-4">
            <div className="voice-waveform">
              {[...Array(5)].map((_, i) => (
                <div
                  key={i}
                  className="voice-bar"
                  style={{
                    animationDelay: `${i * 0.1}s`,
                  }}
                />
              ))}
            </div>
            <Typography.Text style={{ color: textColor, fontSize: 18 }}>
              {t("voice.speaking")}
            </Typography.Text>
          </div>
        );
      case "Listening":
        return (
          <div className="flex flex-col items-center gap-4">
            <Volume2
              size={48}
              style={{ color: token.colorPrimary }}
              className="animate-pulse"
            />
            <Typography.Text style={{ color: textColor, fontSize: 18 }}>
              {t("voice.listening")}
            </Typography.Text>
          </div>
        );
      case "Disconnecting":
        return (
          <div className="flex flex-col items-center gap-4">
            <Spin
              indicator={
                <Loader
                  size={48}
                  style={{
                    color: textColor,
                    animation: "spin 1s linear infinite",
                  }}
                />
              }
            />
            <Typography.Text style={{ color: textColor, fontSize: 18 }}>
              {t("voice.disconnecting")}
            </Typography.Text>
          </div>
        );
      default:
        return null;
    }
  }, [state, t, textColor, token]);

  return <>{content}</>;
}

export function VoiceCall({
  visible,
  onClose,
  port,
  host,
  config,
  apiKey,
}: VoiceCallProps) {
  const { t } = useTranslation();
  const { token: controlToken } = theme.useToken();
  const btnTextColor = controlToken.colorWhite;
  const { state, isMuted, userTranscript, assistantTranscript, start, stop, toggleMute } = useVoiceChat({
    port,
    host,
    config,
    apiKey,
  });

  // Auto-start when overlay becomes visible — 用 useEffect 代替渲染副作用
  useEffect(() => {
    if (visible && state === "Idle") {
      start();
    }
  }, [visible, state, start]);

  const handleEndCall = () => {
    stop();
    onClose();
  };

  if (!visible) {
    return null;
  }

  return (
    <div
      className="fixed inset-0 z-[1000] flex flex-col items-center justify-center"
      style={{ background: controlToken.colorBgMask }}
    >
      {/* Status display */}
      <div className="flex-1 flex flex-col items-center justify-center gap-6 w-full px-8">
        <StatusDisplay state={state} />

        {/* 字幕：用户侧识别文本（右对齐）与 AI 文本增量（左对齐） */}
        <div className="w-full max-w-md flex flex-col gap-3">
          {userTranscript && (
            <div
              className="self-end max-w-[80%] rounded-2xl px-4 py-2 text-sm leading-relaxed"
              style={{ background: controlToken.colorPrimary, color: controlToken.colorWhite }}
            >
              {userTranscript}
            </div>
          )}
          {assistantTranscript && (
            <div
              className="self-start max-w-[80%] rounded-2xl px-4 py-2 text-sm leading-relaxed"
              style={{
                background: controlToken.colorFillSecondary,
                color: controlToken.colorText,
              }}
            >
              {assistantTranscript}
            </div>
          )}
        </div>
      </div>

      {/* Controls */}
      <div className="flex items-center gap-8 pb-16">
        <Button
          shape="circle"
          size="large"
          icon={isMuted ? <MicOff size={20} /> : <Mic size={20} />}
          onClick={toggleMute}
          style={{
            width: 56,
            height: 56,
            background: isMuted ? controlToken.colorError : controlToken.colorFillTertiary,
            border: "none",
            color: btnTextColor,
          }}
          title={t("voice.toggleMute")}
        />
        <Button
          shape="circle"
          size="large"
          icon={<Phone size={24} style={{ transform: "rotate(225deg)" }} />}
          onClick={handleEndCall}
          style={{
            width: 72,
            height: 72,
            background: controlToken.colorError,
            border: "none",
            color: btnTextColor,
            fontSize: 24,
          }}
          title={t("voice.endCall")}
        />
      </div>

      {/* Waveform CSS */}
      <style>
        {`
        .voice-waveform {
          display: flex;
          align-items: center;
          gap: 6px;
          height: 60px;
        }
        .voice-bar {
          width: 6px;
          height: 20px;
          background: #52c41a;
          border-radius: 3px;
          animation: voiceWave 0.8s ease-in-out infinite alternate;
        }
        @keyframes voiceWave {
          0% { height: 12px; }
          100% { height: 48px; }
        }
      `}
      </style>
    </div>
  );
}
