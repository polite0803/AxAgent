import type { RealtimeConfig, VoiceSessionState } from "@/types";
import { App } from "antd";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const VAD_THRESHOLD = 0.015;
const VAD_SILENCE_MS = 1500;

// ─── WebSocket 重连配置 ───

/** 最大重连次数 */
const MAX_RECONNECT_ATTEMPTS = 5;
/** 初始退避延迟（毫秒） */
const RECONNECT_BASE_DELAY_MS = 1000;
/** 最大退避延迟（毫秒） */
const RECONNECT_MAX_DELAY_MS = 30000;

interface UseVoiceChatOptions {
  port?: number;
  host?: string;
  config: RealtimeConfig;
}

interface UseVoiceChatReturn {
  state: VoiceSessionState;
  isMuted: boolean;
  start: () => Promise<void>;
  stop: () => void;
  toggleMute: () => void;
}

export function useVoiceChat({ port = 8080, host = "127.1.0.0", config }: UseVoiceChatOptions): UseVoiceChatReturn {
  const { t } = useTranslation();
  const { message } = App.useApp();

  const [state, setState] = useState<VoiceSessionState>("Idle");
  const [isMuted, setIsMuted] = useState(false);

  const wsRef = useRef<WebSocket | null>(null);
  const audioCtxRef = useRef<AudioContext | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const sourceRef = useRef<MediaStreamAudioSourceNode | null>(null);
  const workletRef = useRef<AudioWorkletNode | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const vadTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const rafRef = useRef<number | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const shouldReconnectRef = useRef(false);

  const cleanup = useCallback((keepReconnecting = false) => {
    if (rafRef.current !== null) {
      cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    }
    if (vadTimerRef.current !== null) {
      clearTimeout(vadTimerRef.current);
      vadTimerRef.current = null;
    }
    workletRef.current?.disconnect();
    workletRef.current = null;
    sourceRef.current?.disconnect();
    sourceRef.current = null;
    analyserRef.current?.disconnect();
    analyserRef.current = null;

    if (streamRef.current) {
      streamRef.current.getTracks().forEach((t) => t.stop());
      streamRef.current = null;
    }
    if (audioCtxRef.current && audioCtxRef.current.state !== "closed") {
      audioCtxRef.current.close().catch((e: unknown) => { console.warn('[IPC]', e); });
      audioCtxRef.current = null;
    }
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    if (!keepReconnecting) {
      shouldReconnectRef.current = false;
      if (reconnectTimerRef.current !== null) {
        clearTimeout(reconnectTimerRef.current);
        reconnectTimerRef.current = null;
      }
      reconnectAttemptsRef.current = 0;
    }
  }, []);

  const runVAD = useCallback(() => {
    const analyser = analyserRef.current;
    if (!analyser) { return; }

    const data = new Float32Array(analyser.fftSize);

    const tick = () => {
      analyser.getFloatTimeDomainData(data);
      let sum = 0;
      for (let i = 0; i < data.length; i++) {
        sum += data[i] * data[i];
      }
      const rms = Math.sqrt(sum / data.length);

      setState((prev) => {
        if (prev !== "Speaking" && prev !== "Listening") { return prev; }

        if (rms > VAD_THRESHOLD) {
          if (vadTimerRef.current !== null) {
            clearTimeout(vadTimerRef.current);
            vadTimerRef.current = null;
          }
          return "Speaking";
        }

        if (prev === "Speaking" && vadTimerRef.current === null) {
          vadTimerRef.current = setTimeout(() => {
            vadTimerRef.current = null;
            setState("Listening");
          }, VAD_SILENCE_MS);
        }
        return prev;
      });

      rafRef.current = requestAnimationFrame(tick);
    };

    rafRef.current = requestAnimationFrame(tick);
  }, []);

  const start = useCallback(async () => {
    if (state !== "Idle") { return; }
    setState("Connecting");

    // 重置重连状态
    reconnectAttemptsRef.current = 0;
    shouldReconnectRef.current = true;
    if (reconnectTimerRef.current !== null) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }

    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: { sampleRate: config.audio_format.sample_rate, channelCount: 1, echoCancellation: true },
      });
      streamRef.current = stream;

      const audioCtx = new AudioContext({ sampleRate: config.audio_format.sample_rate });
      audioCtxRef.current = audioCtx;

      await audioCtx.audioWorklet.addModule("/audio-processor.js");

      const source = audioCtx.createMediaStreamSource(stream);
      sourceRef.current = source;

      const analyser = audioCtx.createAnalyser();
      analyser.fftSize = 2048;
      analyserRef.current = analyser;
      source.connect(analyser);

      const worklet = new AudioWorkletNode(audioCtx, "audio-pcm16-processor");
      workletRef.current = worklet;
      source.connect(worklet);

      connectWebSocket();
    } catch (err) {
      const errMsg = err instanceof DOMException && err.name === "NotAllowedError"
        ? t("voice.micPermissionDenied")
        : t("voice.micError");
      message.error(errMsg);
      cleanup();
      setState("Idle");
    }
  }, [state, port, host, config, isMuted, cleanup, runVAD, message, t]);

  // ── WebSocket 连接与重连 ──

  const connectWebSocket = useCallback(() => {
    // 清理旧连接
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    const ws = new WebSocket(`ws://${host}:${port}/v1/realtime`);
    wsRef.current = ws;
    ws.binaryType = "arraybuffer";

    ws.onopen = () => {
      reconnectAttemptsRef.current = 0;
      ws.send(JSON.stringify({ type: "session.config", config }));
      setState("Connected");
      setTimeout(() => setState("Speaking"), 300);
      runVAD();
    };

    const worklet = workletRef.current;
    if (worklet) {
      worklet.port.onmessage = (e: MessageEvent) => {
        if (ws.readyState === WebSocket.OPEN && !isMuted) {
          ws.send(e.data as ArrayBuffer);
        }
      };
    }

    ws.onmessage = (_e: MessageEvent) => {
      // 服务端音频回放在此处理
    };

    ws.onerror = () => {
      console.warn("[Voice] WebSocket 连接错误");
    };

    ws.onclose = (event) => {
      // 正常关闭（1000）或用户主动断开则不重连
      if (!shouldReconnectRef.current || event.code === 1000) {
        cleanup();
        setState("Idle");
        return;
      }

      const attempts = reconnectAttemptsRef.current;
      if (attempts >= MAX_RECONNECT_ATTEMPTS) {
        message.error(t("voice.connectionError"));
        cleanup();
        setState("Idle");
        return;
      }

      reconnectAttemptsRef.current = attempts + 1;
      setState("Connecting");

      // 指数退避延迟
      const delay = Math.min(
        RECONNECT_BASE_DELAY_MS * Math.pow(2, attempts),
        RECONNECT_MAX_DELAY_MS,
      );

      console.warn(
        `[Voice] WebSocket 断开，${delay}ms 后第 ${attempts + 1}/${MAX_RECONNECT_ATTEMPTS} 次重连...`,
      );

      reconnectTimerRef.current = setTimeout(() => {
        reconnectTimerRef.current = null;
        connectWebSocket();
      }, delay);
    };
  }, [host, port, config, isMuted, cleanup, setState, runVAD, message, t]);

  const stop = useCallback(() => {
    if (state === "Idle" || state === "Disconnecting") { return; }
    setState("Disconnecting");
    shouldReconnectRef.current = false;
    if (reconnectTimerRef.current !== null) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
    cleanup();
    setState("Idle");
  }, [state, cleanup]);

  const toggleMute = useCallback(() => {
    const newMuted = !isMuted;
    setIsMuted(newMuted);
    if (streamRef.current) {
      streamRef.current.getAudioTracks().forEach((track) => {
        track.enabled = !newMuted;
      });
    }
  }, [isMuted]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      cleanup();
    };
  }, [cleanup]);

  return { state, isMuted, start, stop, toggleMute };
}
