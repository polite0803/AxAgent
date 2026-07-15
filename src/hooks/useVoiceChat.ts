// SPDX-License-Identifier: AGPL-3.0-only

import { loadAudioWorklet } from "@/lib/audioProcessorWorklet";
import { logIpcError } from "@/lib/invoke";
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

// ─── 音频播放 ───

/** 将 base64 PCM16 解码为 Float32 ArrayBuffer */
function decodePcm16(base64Audio: string): Float32Array {
  const raw = atob(base64Audio);
  const samples = new Int16Array(raw.length / 2);
  for (let i = 0; i < samples.length; i++) {
    const lo = raw.charCodeAt(i * 2);
    const hi = raw.charCodeAt(i * 2 + 1);
    samples[i] = (hi << 8) | lo;
  }
  const float = new Float32Array(samples.length);
  for (let i = 0; i < samples.length; i++) {
    float[i] = samples[i] / 32768;
  }
  return float;
}

class AudioPlayback {
  private ctx: AudioContext;
  private queue: AudioBuffer[] = [];
  private isPlaying = false;
  private source: AudioBufferSourceNode | null = null;
  private gainNode: GainNode;

  constructor(ctx: AudioContext) {
    this.ctx = ctx;
    this.gainNode = ctx.createGain();
    this.gainNode.gain.value = 1;
    this.gainNode.connect(ctx.destination);
  }

  enqueue(base64Audio: string): void {
    const floatData = decodePcm16(base64Audio);
    const buffer = this.ctx.createBuffer(1, floatData.length, this.ctx.sampleRate);
    buffer.getChannelData(0).set(floatData);
    this.queue.push(buffer);
    if (!this.isPlaying) {
      this.playNext();
    }
  }

  flush(): void {
    // 等待当前播放结束即可，不需要特殊处理
  }

  private playNext(): void {
    if (this.queue.length === 0) {
      this.isPlaying = false;
      return;
    }
    this.isPlaying = true;
    const buffer = this.queue.shift()!;
    this.source = this.ctx.createBufferSource();
    this.source.buffer = buffer;
    this.source.connect(this.gainNode);
    this.source.onended = () => {
      this.source = null;
      this.playNext();
    };
    this.source.start();
  }

  stop(): void {
    if (this.source) {
      this.source.stop();
      this.source = null;
    }
    this.queue = [];
    this.isPlaying = false;
  }

  close(): void {
    this.stop();
  }
}

interface UseVoiceChatOptions {
  port?: number;
  host?: string;
  config: RealtimeConfig;
  apiKey: string;
}

interface UseVoiceChatReturn {
  state: VoiceSessionState;
  isMuted: boolean;
  /** 用户侧语音识别文本（字幕） */
  userTranscript: string;
  /** AI 文本响应（字幕） */
  assistantTranscript: string;
  start: () => Promise<void>;
  stop: () => void;
  toggleMute: () => void;
}

export function useVoiceChat({
  port = 8080,
  host = "127.0.0.1",
  config,
  apiKey,
}: UseVoiceChatOptions): UseVoiceChatReturn {
  const { t } = useTranslation();
  const { message } = App.useApp();

  const [state, setState] = useState<VoiceSessionState>("Idle");
  const [isMuted, setIsMuted] = useState(false);
  const [userTranscript, setUserTranscript] = useState("");
  const [assistantTranscript, setAssistantTranscript] = useState("");
  const isMutedRef = useRef(false);

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
  const stateRef = useRef<VoiceSessionState>("Idle");
  const connectWebSocketRef = useRef<() => void>(() => {});
  const ticketRef = useRef("");
  const audioPlaybackRef = useRef<AudioPlayback | null>(null);
  /** AI 是否正在播报（用于打断判定） */
  const aiRespondingRef = useRef(false);
  /** VAD 上一帧是否检测到语音（用于检测「开始说话」的上升沿） */
  const prevSpeakingRef = useRef(false);

  // Keep refs in sync with state after each render
  useEffect(() => {
    isMutedRef.current = isMuted;
  }, [isMuted]);

  useEffect(() => {
    stateRef.current = state;
  }, [state]);

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
    audioPlaybackRef.current?.close();
    audioPlaybackRef.current = null;

    if (streamRef.current) {
      streamRef.current.getTracks().forEach((t) => t.stop());
      streamRef.current = null;
    }
    if (audioCtxRef.current && audioCtxRef.current.state !== "closed") {
      audioCtxRef.current.close().catch(logIpcError("VoiceChat.closeAudioCtx"));
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

  /// 主动打断：停止本地播放、清空部分字幕、向后端发送 response.cancel
  const interrupt = useCallback(() => {
    audioPlaybackRef.current?.stop();
    aiRespondingRef.current = false;
    setAssistantTranscript("");
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({ type: "response.cancel" }));
    }
  }, []);

  const runVAD = useCallback(() => {
    const analyser = analyserRef.current;
    if (!analyser) {
      return;
    }

    const data = new Float32Array(analyser.fftSize);

    const tick = () => {
      analyser.getFloatTimeDomainData(data);
      let sum = 0;
      for (let i = 0; i < data.length; i++) {
        sum += data[i] * data[i];
      }
      const rms = Math.sqrt(sum / data.length);
      const isSpeech = rms > VAD_THRESHOLD;

      // 用户打断：AI 正在播报时检测到用户开口 → 立即停止播放并通知后端中止生成
      if (isSpeech && !prevSpeakingRef.current && aiRespondingRef.current) {
        interrupt();
      }
      prevSpeakingRef.current = isSpeech;

      setState((prev) => {
        if (prev !== "Speaking" && prev !== "Listening") {
          return prev;
        }

        if (isSpeech) {
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
  }, [interrupt]);

  const start = useCallback(async () => {
    if (stateRef.current !== "Idle") {
      return;
    }
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
        audio: {
          sampleRate: config.audio_format.sample_rate,
          channelCount: 1,
          echoCancellation: true,
        },
      });
      streamRef.current = stream;

      const audioCtx = new AudioContext({
        sampleRate: config.audio_format.sample_rate,
      });
      audioCtxRef.current = audioCtx;
      audioPlaybackRef.current = new AudioPlayback(audioCtx);

      await loadAudioWorklet(audioCtx);

      const source = audioCtx.createMediaStreamSource(stream);
      sourceRef.current = source;

      const analyser = audioCtx.createAnalyser();
      analyser.fftSize = 2048;
      analyserRef.current = analyser;
      source.connect(analyser);

      const worklet = new AudioWorkletNode(audioCtx, "audio-pcm16-processor");
      workletRef.current = worklet;
      source.connect(worklet);

      // 获取 ticket
      const ticketResp = await fetch(
        `http://${host}:${port}/v1/realtime-ticket`,
        {
          method: "POST",
          headers: { Authorization: `Bearer ${apiKey}` },
        },
      );
      if (!ticketResp.ok) {
        throw new Error(`Ticket request failed: ${ticketResp.status}`);
      }
      const { ticket } = await ticketResp.json() as { ticket: string };
      ticketRef.current = ticket;

      connectWebSocketRef.current();
    } catch (err) {
      const errMsg = err instanceof DOMException && err.name === "NotAllowedError"
        ? t("voice.micPermissionDenied")
        : err instanceof Error
        ? err.message
        : t("voice.micError");
      message.error(errMsg);
      cleanup();
      setState("Idle");
    }
  }, [config, apiKey, cleanup, message, t, host, port]);

  // ── WebSocket 连接与重连 ──

  const connectWebSocket = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    const ticket = ticketRef.current;
    if (!ticket) {
      message.error("No ticket available. Please start again.");
      return;
    }

    const ws = new WebSocket(`ws://${host}:${port}/v1/realtime?ticket=${ticket}`);
    wsRef.current = ws;
    ws.binaryType = "arraybuffer";

    ws.onopen = () => {
      reconnectAttemptsRef.current = 0;
      // 发送 session.create（带音色），而非 session.config
      ws.send(
        JSON.stringify({
          type: "session.create",
          model: config.model_id,
          voice: config.voice,
          stt_provider: config.stt_provider_id ?? null,
          tts_provider: config.tts_provider_id ?? null,
        }),
      );
    };

    const worklet = workletRef.current;
    if (worklet) {
      worklet.port.onmessage = null;
      worklet.port.onmessage = (e: MessageEvent) => {
        if (ws.readyState === WebSocket.OPEN && !isMutedRef.current) {
          ws.send(e.data as ArrayBuffer);
        }
      };
    }

    ws.onmessage = (e: MessageEvent) => {
      try {
        const msg = JSON.parse(e.data as string) as Record<string, unknown>;
        switch (msg.type) {
          case "session.created":
            setState("Connected");
            setUserTranscript("");
            setAssistantTranscript("");
            runVAD();
            break;
          case "conversation.item.input_audio_transcription.completed":
            // 用户侧语音识别结果（字幕）
            setUserTranscript((msg.transcript as string) ?? "");
            setAssistantTranscript("");
            break;
          case "response.text.delta":
            // AI 文本增量（字幕）
            setAssistantTranscript((prev) => prev + (msg.delta as string));
            break;
          case "response.audio.delta":
            setState("Listening");
            aiRespondingRef.current = true;
            audioPlaybackRef.current?.enqueue(msg.delta as string);
            break;
          case "response.audio.done":
            audioPlaybackRef.current?.flush();
            break;
          case "response.done":
            setState("Speaking");
            aiRespondingRef.current = false;
            break;
          case "error":
            logIpcError("VoiceChat.serverError")(msg.message as string);
            break;
        }
      } catch {
        logIpcError("VoiceChat.parseError")("Failed to parse server message");
      }
    };

    ws.onerror = () => {
      logIpcError("VoiceChat.wsError")("WebSocket connection error");
    };

    ws.onclose = (event) => {
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

      const delay = Math.min(
        RECONNECT_BASE_DELAY_MS * Math.pow(2, attempts),
        RECONNECT_MAX_DELAY_MS,
      );

      logIpcError("VoiceChat.reconnect")(
        `WebSocket disconnected, ${delay}ms before attempt ${attempts + 1}/${MAX_RECONNECT_ATTEMPTS}`,
      );

      reconnectTimerRef.current = setTimeout(() => {
        reconnectTimerRef.current = null;
        connectWebSocketRef.current();
      }, delay);
    };
  }, [host, port, config.model_id, cleanup, runVAD, message, t]);

  // Keep connectWebSocketRef in sync
  useEffect(() => {
    connectWebSocketRef.current = connectWebSocket;
  }, [connectWebSocket]);

  const stop = useCallback(() => {
    if (stateRef.current === "Idle" || stateRef.current === "Disconnecting") {
      return;
    }
    setState("Disconnecting");
    shouldReconnectRef.current = false;
    if (reconnectTimerRef.current !== null) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
    cleanup();
    setState("Idle");
  }, [cleanup]);

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

  return { state, isMuted, userTranscript, assistantTranscript, start, stop, toggleMute };
}
