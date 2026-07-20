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

/** AudioPlayback 队列容量上限，防止 TTS 推送速度大于播放速度时 OOM */
const AUDIO_PLAYBACK_MAX_QUEUE = 100;

/** 安全解码 base64 字符串：长度为奇数时丢弃最后一字节，避免 Int16Array 截断 */
function decodePcm16(base64Audio: string): Float32Array {
  let raw: string;
  try {
    raw = atob(base64Audio);
  } catch (e) {
    // base64 解码失败（输入非 string 或非法字符）— 返回空数组，避免抛错中断播放
    logIpcError("VoiceChat.decodePcm16")(e);
    return new Float32Array(0);
  }
  const sampleCount = Math.floor(raw.length / 2);
  const samples = new Int16Array(sampleCount);
  for (let i = 0; i < sampleCount; i++) {
    const lo = raw.charCodeAt(i * 2);
    const hi = raw.charCodeAt(i * 2 + 1);
    samples[i] = (hi << 8) | lo;
  }
  const float = new Float32Array(sampleCount);
  for (let i = 0; i < sampleCount; i++) {
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
    if (floatData.length === 0) {
      return;
    }
    const buffer = this.ctx.createBuffer(1, floatData.length, this.ctx.sampleRate);
    buffer.getChannelData(0).set(floatData);
    this.queue.push(buffer);
    // P2-13：队列容量上限，超限丢弃最旧（保留最新音频以减少延迟）
    if (this.queue.length > AUDIO_PLAYBACK_MAX_QUEUE) {
      const dropCount = this.queue.length - AUDIO_PLAYBACK_MAX_QUEUE;
      this.queue.splice(0, dropCount);
    }
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
      try {
        this.source.stop();
      } catch {
        // source 已 stop 或未 start 时会抛 InvalidStateError，忽略
      }
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

/** 解析 ws.onmessage 收到的 JSON 消息，运行时校验字段类型 */
function parseServerMessage(
  raw: unknown,
): { type: string; delta?: string; transcript?: string; message?: string } | null {
  if (typeof raw !== "string") {
    return null;
  }
  let obj: unknown;
  try {
    obj = JSON.parse(raw);
  } catch {
    return null;
  }
  if (!obj || typeof obj !== "object") {
    return null;
  }
  const record = obj as Record<string, unknown>;
  const type = record.type;
  if (typeof type !== "string") {
    return null;
  }
  const result: { type: string; delta?: string; transcript?: string; message?: string } = { type };
  if (typeof record.delta === "string") {
    result.delta = record.delta;
  }
  if (typeof record.transcript === "string") {
    result.transcript = record.transcript;
  }
  if (typeof record.message === "string") {
    result.message = record.message;
  }
  return result;
}

/** 创建 AudioContext，目标 sampleRate 失败时回退到硬件默认（P1-12 跨平台兼容） */
async function createAudioContextWithFallback(targetSampleRate: number): Promise<AudioContext> {
  try {
    const ctx = new AudioContext({ sampleRate: targetSampleRate });
    // 某些浏览器不会抛出但会忽略 sampleRate，校验一下
    if (Math.abs(ctx.sampleRate - targetSampleRate) > 1) {
      logIpcError("VoiceChat.audioCtxSampleRateMismatch")(
        `requested=${targetSampleRate} actual=${ctx.sampleRate}`,
      );
    }
    return ctx;
  } catch (e) {
    logIpcError("VoiceChat.createAudioCtxFallback")(e);
    return new AudioContext();
  }
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
  // P3-19：isMutedRef 立即同步，避免 toggleMute 后 WS 仍用旧值
  const isMutedRef = useRef(false);
  // P3-20：mounted 标记，防止卸载后 setState 警告
  const mountedRef = useRef(true);

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
  // P0-3：start 互斥 token，await 完成后检查是否仍是最新一次 start
  const startTokenRef = useRef(0);

  // Keep refs in sync with state after each render
  useEffect(() => {
    isMutedRef.current = isMuted;
  }, [isMuted]);

  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  // P3-20：卸载时设置 mounted 标记
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

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
      streamRef.current.getTracks().forEach((track) => track.stop());
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
      // P3-20：组件已卸载或 cleanup 已取消 RAF 时停止
      if (!mountedRef.current || rafRef.current === null) {
        return;
      }
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

      if (mountedRef.current) {
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
              if (mountedRef.current) {
                setState("Listening");
              }
            }, VAD_SILENCE_MS);
          }
          return prev;
        });
      }

      rafRef.current = requestAnimationFrame(tick);
    };

    rafRef.current = requestAnimationFrame(tick);
  }, [interrupt]);

  const start = useCallback(async () => {
    if (stateRef.current !== "Idle" && stateRef.current !== "Error") {
      return;
    }
    // P0-3：递增 startToken，await 完成后检查是否仍是最新一次
    const myToken = ++startTokenRef.current;
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
      // P0-3：await 期间用户可能点了 stop 或重新 start，token 不匹配则丢弃本次 stream
      if (myToken !== startTokenRef.current) {
        stream.getTracks().forEach((track) => track.stop());
        return;
      }
      streamRef.current = stream;

      // P1-12：目标 sampleRate 失败时回退到硬件默认
      const audioCtx = await createAudioContextWithFallback(config.audio_format.sample_rate);
      if (myToken !== startTokenRef.current) {
        // await 期间被取消
        stream.getTracks().forEach((track) => track.stop());
        audioCtx.close().catch(logIpcError("VoiceChat.cancelCloseAudioCtx"));
        return;
      }
      audioCtxRef.current = audioCtx;
      audioPlaybackRef.current = new AudioPlayback(audioCtx);

      await loadAudioWorklet(audioCtx);
      if (myToken !== startTokenRef.current) {
        return;
      }

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
      if (myToken !== startTokenRef.current) {
        return;
      }
      if (!ticketResp.ok) {
        // P1-10：错误消息 i18n 化，不暴露 HTTP 状态码
        message.error(t("voice.ticketFailed"));
        cleanup();
        setState("Error");
        return;
      }
      const ticketJson = (await ticketResp.json()) as { ticket?: string };
      if (myToken !== startTokenRef.current) {
        return;
      }
      if (!ticketJson.ticket) {
        message.error(t("voice.ticketFailed"));
        cleanup();
        setState("Error");
        return;
      }
      ticketRef.current = ticketJson.ticket;

      connectWebSocketRef.current();
    } catch (err) {
      // P0-3：被取消的 start 不处理错误
      if (myToken !== startTokenRef.current) {
        return;
      }
      // P1-7：麦克风错误分支补全，按 DOMException name 分类提示
      const errName = err instanceof DOMException ? err.name : "";
      let errMsg: string;
      switch (errName) {
        case "NotAllowedError":
          errMsg = t("voice.micPermissionDenied");
          break;
        case "NotFoundError":
        case "OverconstrainedError":
          errMsg = t("voice.micNotFound");
          break;
        case "NotReadableError":
          errMsg = t("voice.micInUse");
          break;
        case "SecurityError":
          errMsg = t("voice.micSecurity");
          break;
        default:
          errMsg = err instanceof Error ? err.message : t("voice.micError");
      }
      message.error(errMsg);
      cleanup();
      setState("Error");
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
      message.error(t("voiceChat.noTicket"));
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

    // P0-4：worklet port 在每次 connectWebSocket 时重新绑定，
    // 防止重连后 workletRef 已被 cleanup 置空导致音频不被发送。
    // cleanup(keepReconnecting=true) 不会清理 worklet，但保险起见始终重绑。
    const worklet = workletRef.current;
    if (worklet) {
      worklet.port.onmessage = (e: MessageEvent) => {
        if (ws.readyState === WebSocket.OPEN && !isMutedRef.current) {
          ws.send(e.data as ArrayBuffer);
        }
      };
    }

    ws.onmessage = (e: MessageEvent) => {
      // P2-14：运行时类型校验
      const msg = parseServerMessage(e.data);
      if (!msg) {
        logIpcError("VoiceChat.parseError")("Failed to parse server message");
        return;
      }
      switch (msg.type) {
        case "session.created":
          if (mountedRef.current) {
            setState("Connected");
            setUserTranscript("");
            setAssistantTranscript("");
          }
          runVAD();
          break;
        case "conversation.item.input_audio_transcription.completed":
          // 用户侧语音识别结果（字幕）
          if (mountedRef.current) {
            setUserTranscript(msg.transcript ?? "");
            setAssistantTranscript("");
          }
          break;
        case "response.text.delta":
          // AI 文本增量（字幕）— delta 缺失时跳过
          if (msg.delta && mountedRef.current) {
            setAssistantTranscript((prev) => prev + msg.delta!);
          }
          break;
        case "response.audio.delta":
          if (mountedRef.current) {
            setState("Listening");
          }
          aiRespondingRef.current = true;
          // delta 缺失时不入队，避免 decodePcm16 抛错
          if (msg.delta) {
            audioPlaybackRef.current?.enqueue(msg.delta);
          }
          break;
        case "response.audio.done":
          audioPlaybackRef.current?.flush();
          break;
        case "response.done":
          if (mountedRef.current) {
            setState("Speaking");
          }
          aiRespondingRef.current = false;
          break;
        case "error":
          // P3-18：服务端错误同时通知用户，避免静默丢失
          logIpcError("VoiceChat.serverError")(msg.message ?? "unknown");
          if (msg.message && mountedRef.current) {
            message.error(msg.message);
          }
          break;
        default:
          logIpcError("VoiceChat.unknownMsgType")(msg.type);
      }
    };

    ws.onerror = () => {
      // P3-18：WS 错误同时通知用户，避免用户感知不到失败
      logIpcError("VoiceChat.wsError")("WebSocket connection error");
    };

    ws.onclose = (event) => {
      if (!shouldReconnectRef.current || event.code === 1000) {
        cleanup();
        if (mountedRef.current) {
          setState("Idle");
        }
        return;
      }

      const attempts = reconnectAttemptsRef.current;
      if (attempts >= MAX_RECONNECT_ATTEMPTS) {
        // P1-11：重连耗尽进入 Error 状态，UI 可区分正常/异常断开
        message.error(t("voice.connectionError"));
        cleanup();
        if (mountedRef.current) {
          setState("Error");
        }
        return;
      }

      reconnectAttemptsRef.current = attempts + 1;
      if (mountedRef.current) {
        setState("Connecting");
      }

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
  }, [
    host,
    port,
    config.model_id,
    config.voice,
    config.stt_provider_id,
    config.tts_provider_id,
    cleanup,
    runVAD,
    message,
    t,
  ]);

  // Keep connectWebSocketRef in sync
  useEffect(() => {
    connectWebSocketRef.current = connectWebSocket;
  }, [connectWebSocket]);

  const stop = useCallback(() => {
    if (stateRef.current === "Idle" || stateRef.current === "Disconnecting") {
      return;
    }
    // P0-3：递增 startToken 让进行中的 start 失效
    startTokenRef.current++;
    setState("Disconnecting");
    shouldReconnectRef.current = false;
    if (reconnectTimerRef.current !== null) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
    cleanup();
    if (mountedRef.current) {
      setState("Idle");
    }
  }, [cleanup]);

  const toggleMute = useCallback(() => {
    const newMuted = !isMuted;
    // P3-19：立即同步 ref，避免 useEffect 延迟导致 WS 短暂错发/漏发音频
    isMutedRef.current = newMuted;
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
