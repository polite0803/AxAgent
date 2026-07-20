// SPDX-License-Identifier: AGPL-3.0-only

import { logIpcError } from "@/lib/invoke";
import { App } from "antd";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const WAKE_THRESHOLD = 0.02;
/** 需连续检测到语音达到该时长（ms）才触发唤醒，避免环境噪声误触发 */
const WAKE_HOLD_MS = 350;

interface UseVoiceWakeupOptions {
  /** 检测到唤醒词/语音时回调（如打开语音通话浮层） */
  onWake: () => void;
  /** RMS 阈值，默认 0.02 */
  threshold?: number;
}

interface UseVoiceWakeupReturn {
  /** 是否正在常驻监听 */
  active: boolean;
  /** 开始常驻监听 */
  start: () => Promise<void>;
  /** 停止常驻监听 */
  stop: () => void;
}

/**
 * 轻量常驻语音唤醒。
 *
 * 与通话浮层（useVoiceChat）解耦：它只用一个低成本 AnalyserNode 做 RMS VAD，
 * 不接 worklet、不连 WebSocket、不合成音频，CPU/内存开销极低，可长时间挂载。
 * 命中唤醒后调用 onWake（例如打开 VoiceCall 浮层）。
 *
 * 状态机：activeRef / startingRef / stoppingRef 三个 ref 互斥，
 * 防止快速点击两次「启动」造成 MediaStream 泄漏（P0-2 修复）。
 */
export function useVoiceWakeup({
  onWake,
  threshold = WAKE_THRESHOLD,
}: UseVoiceWakeupOptions): UseVoiceWakeupReturn {
  const { t } = useTranslation();
  const { message } = App.useApp();

  const [active, setActive] = useState(false);

  const streamRef = useRef<MediaStream | null>(null);
  const ctxRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const rafRef = useRef<number | null>(null);
  const speechStartRef = useRef<number | null>(null);
  const firedRef = useRef(false);
  const onWakeRef = useRef(onWake);
  onWakeRef.current = onWake;

  // 互斥 ref：防止 start/stop 竞态导致 MediaStream 泄漏
  const activeRef = useRef(false);
  const startingRef = useRef(false);
  const stoppingRef = useRef(false);

  const cleanup = useCallback(() => {
    if (rafRef.current !== null) {
      cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    }
    if (streamRef.current) {
      streamRef.current.getTracks().forEach((track) => track.stop());
      streamRef.current = null;
    }
    if (ctxRef.current && ctxRef.current.state !== "closed") {
      // 关闭失败时记录日志，便于排查（P3-18 改进）
      ctxRef.current.close().catch((e: unknown) => {
        logIpcError("VoiceWakeup.closeAudioCtx")(e);
      });
      ctxRef.current = null;
    }
    analyserRef.current = null;
    speechStartRef.current = null;
    firedRef.current = false;
  }, []);

  const start = useCallback(async () => {
    // 互斥：正在启动 / 已激活 / 正在停止中均直接返回
    if (activeRef.current || startingRef.current || stoppingRef.current) {
      return;
    }
    startingRef.current = true;
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: { echoCancellation: true, noiseSuppression: true },
      });
      // await 期间用户可能已点击 stop，此时丢弃新申请到的 stream
      if (stoppingRef.current) {
        stream.getTracks().forEach((track) => track.stop());
        return;
      }
      streamRef.current = stream;

      const ctx = new AudioContext({ sampleRate: 16000 });
      ctxRef.current = ctx;
      const source = ctx.createMediaStreamSource(stream);
      const analyser = ctx.createAnalyser();
      analyser.fftSize = 512;
      source.connect(analyser);
      analyserRef.current = analyser;

      const data = new Float32Array(analyser.fftSize);
      const tick = () => {
        if (!activeRef.current) {
          return;
        }
        analyser.getFloatTimeDomainData(data);
        let sum = 0;
        for (let i = 0; i < data.length; i++) {
          sum += data[i] * data[i];
        }
        const rms = Math.sqrt(sum / data.length);
        const now = performance.now();

        if (rms > threshold) {
          if (speechStartRef.current === null) {
            speechStartRef.current = now;
          } else if (now - speechStartRef.current > WAKE_HOLD_MS && !firedRef.current) {
            firedRef.current = true;
            onWakeRef.current();
          }
        } else {
          speechStartRef.current = null;
        }
        rafRef.current = requestAnimationFrame(tick);
      };
      activeRef.current = true;
      setActive(true);
      rafRef.current = requestAnimationFrame(tick);
    } catch (err) {
      // P1-8：错误处理补全，按 DOMException name 分类提示
      cleanup();
      activeRef.current = false;
      setActive(false);
      const errName = err instanceof DOMException ? err.name : "";
      let errMsg: string;
      switch (errName) {
        case "NotAllowedError":
        case "SecurityError":
          errMsg = t("voice.micPermissionDenied");
          break;
        case "NotFoundError":
        case "OverconstrainedError":
          errMsg = t("voice.micNotFound");
          break;
        case "NotReadableError":
          errMsg = t("voice.micInUse");
          break;
        default:
          errMsg = err instanceof Error ? err.message : t("voice.micError");
      }
      message.error(errMsg);
    } finally {
      startingRef.current = false;
    }
  }, [cleanup, message, t, threshold]);

  const stop = useCallback(() => {
    // 正在启动时设置 stoppingRef，让 start 的 await 完成后自行丢弃 stream
    if (startingRef.current) {
      stoppingRef.current = true;
    }
    activeRef.current = false;
    setActive(false);
    cleanup();
    stoppingRef.current = false;
  }, [cleanup]);

  useEffect(() => {
    return () => {
      activeRef.current = false;
      cleanup();
    };
  }, [cleanup]);

  return { active, start, stop };
}
