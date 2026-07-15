// SPDX-License-Identifier: AGPL-3.0-only

import { useCallback, useEffect, useRef, useState } from "react";

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
 */
export function useVoiceWakeup({
  onWake,
  threshold = WAKE_THRESHOLD,
}: UseVoiceWakeupOptions): UseVoiceWakeupReturn {
  const [active, setActive] = useState(false);

  const streamRef = useRef<MediaStream | null>(null);
  const ctxRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const rafRef = useRef<number | null>(null);
  const speechStartRef = useRef<number | null>(null);
  const firedRef = useRef(false);
  const onWakeRef = useRef(onWake);
  onWakeRef.current = onWake;

  const cleanup = useCallback(() => {
    if (rafRef.current !== null) {
      cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    }
    if (streamRef.current) {
      streamRef.current.getTracks().forEach((t) => t.stop());
      streamRef.current = null;
    }
    if (ctxRef.current && ctxRef.current.state !== "closed") {
      ctxRef.current.close().catch(() => {});
      ctxRef.current = null;
    }
    analyserRef.current = null;
    speechStartRef.current = null;
    firedRef.current = false;
  }, []);

  const start = useCallback(async () => {
    if (active) {
      return;
    }
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: { echoCancellation: true, noiseSuppression: true },
      });
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
      rafRef.current = requestAnimationFrame(tick);
      setActive(true);
    } catch {
      cleanup();
      setActive(false);
    }
  }, [active, cleanup, threshold]);

  const stop = useCallback(() => {
    cleanup();
    setActive(false);
  }, [cleanup]);

  useEffect(() => {
    return () => {
      cleanup();
    };
  }, [cleanup]);

  return { active, start, stop };
}
