// SPDX-License-Identifier: AGPL-3.0-only

import { useVoiceChat } from "@/hooks/useVoiceChat";
import type { RealtimeConfig } from "@/types";
import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, type Mock, vi } from "vitest";

// ── Mock 工厂 ────────────────────────────────────────────────

const messageError = vi.fn();

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: vi.fn() },
  useTranslation: () => ({ t: (k: string) => k }),
}));

vi.mock("@/lib/invoke", () => ({
  logIpcError: () => vi.fn(),
}));

vi.mock("@/lib/audioProcessorWorklet", () => ({
  loadAudioWorklet: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("antd", () => {
  const MockApp = ({ children }: { children: React.ReactNode }) => <>{children}</>;
  (MockApp as any).useApp = () => ({
    message: { info: vi.fn(), success: vi.fn(), error: messageError, warning: vi.fn() },
  });
  return { App: MockApp };
});

// ── Global mock 实例追踪 ─────────────────────────────────────

interface MockWsInstance {
  onopen: null | (() => void);
  onmessage: null | ((e: MessageEvent) => void);
  onerror: null | (() => void);
  onclose: null | ((e: CloseEvent) => void);
  send: Mock;
  close: Mock;
  readyState: number;
  binaryType: string;
}

const mockWsInstances: MockWsInstance[] = [];
const mockWorkletInstances: MockAudioWorkletNode[] = [];
let wsConstructorUrl = "";

class MockAudioWorkletNode {
  port = { onmessage: null as ((e: MessageEvent) => void) | null, postMessage: vi.fn() };
  disconnect = vi.fn();
  connect = vi.fn();

  constructor() {
    mockWorkletInstances.push(this);
  }
}

function createMockWs(url: string): MockWsInstance {
  wsConstructorUrl = url;
  const inst: MockWsInstance = {
    onopen: null,
    onmessage: null,
    onerror: null,
    onclose: null,
    send: vi.fn(),
    close: vi.fn(),
    readyState: 1,
    binaryType: "blob",
  };
  mockWsInstances.push(inst);
  setTimeout(() => inst.onopen?.(), 0);
  return inst;
}
createMockWs.CONNECTING = 0;
createMockWs.OPEN = 1;
createMockWs.CLOSING = 2;
createMockWs.CLOSED = 3;

// ── 测试配置 ──────────────────────────────────────────────────

const defaultConfig: RealtimeConfig = {
  modelId: "gpt-4o-realtime-preview",
  voice: null,
  audioFormat: { sampleRate: 24000, channels: 1, encoding: "Pcm16" },
};

const defaultApiKey = "sk-test-key-123";

function renderVoiceChat(apiKey = defaultApiKey) {
  return renderHook(() => useVoiceChat({ config: defaultConfig, apiKey }));
}

function simulateWsMessage(type: string, extra: Record<string, unknown> = {}) {
  const ws = mockWsInstances[0];
  if (!ws || !ws.onmessage) { return; }
  ws.onmessage(
    new MessageEvent("message", {
      data: JSON.stringify({ type, ...extra }),
    }),
  );
}

async function flushTimers(): Promise<void> {
  await act(async () => {
    await new Promise((r) => setTimeout(r, 0));
  });
}

// ── 测试套件 ──────────────────────────────────────────────────

describe("useVoiceChat", () => {
  let mockFetch: Mock;
  let mockMediaDevices: Mock;

  beforeEach(() => {
    mockWsInstances.length = 0;
    mockWorkletInstances.length = 0;
    wsConstructorUrl = "";

    mockFetch = vi.fn();
    vi.stubGlobal("fetch", mockFetch);

    const mockTrackStop = vi.fn();
    const mockTrack = { stop: mockTrackStop, enabled: true } as unknown as MediaStreamTrack;
    const mockStream = {
      getTracks: () => [mockTrack],
      getAudioTracks: () => [mockTrack],
    } as unknown as MediaStream;
    mockMediaDevices = vi.fn().mockResolvedValue(mockStream);
    Object.defineProperty(globalThis.navigator, "mediaDevices", {
      value: { getUserMedia: mockMediaDevices },
      configurable: true,
    });

    vi.stubGlobal(
      "AudioContext",
      class {
        state = "running";
        sampleRate = 24000;
        destination = {} as AudioDestinationNode;
        createGain = () => ({ gain: { value: 1 } as unknown as AudioParam, connect: vi.fn() });
        createBuffer = vi.fn(() => ({
          getChannelData: vi.fn(() => new Float32Array(1024)),
          length: 1024,
          numberOfChannels: 1,
          sampleRate: 24000,
        }));
        createBufferSource = () => ({
          buffer: null,
          connect: vi.fn(),
          start: vi.fn(),
          stop: vi.fn(),
          get onended() {
            return null;
          },
          set onended(_: unknown) {},
        });
        createMediaStreamSource = vi.fn(() => ({ connect: vi.fn(), disconnect: vi.fn() }));
        createAnalyser = () => ({
          fftSize: 2048,
          getFloatTimeDomainData: vi.fn(),
          connect: vi.fn(),
          disconnect: vi.fn(),
        });
        close = vi.fn().mockResolvedValue(undefined);
        audioWorklet = { addModule: vi.fn().mockResolvedValue(undefined) };
      },
    );

    vi.stubGlobal("AudioWorkletNode", MockAudioWorkletNode);
    vi.stubGlobal("requestAnimationFrame", vi.fn().mockReturnValue(42));
    vi.stubGlobal("cancelAnimationFrame", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  // ═══════════════════════════════════════════════════════════
  // 初始状态
  // ═══════════════════════════════════════════════════════════

  it("starts in Idle state and not muted", () => {
    const { result } = renderVoiceChat();
    expect(result.current.state).toBe("Idle");
    expect(result.current.isMuted).toBe(false);
  });

  // ═══════════════════════════════════════════════════════════
  // start() — ticket 获取 + WebSocket 连接
  // ═══════════════════════════════════════════════════════════

  it("fetches ticket and connects WebSocket with ticket in query", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ ticket: "ticket-abc-123" }),
    });
    vi.stubGlobal("WebSocket", createMockWs as unknown as typeof WebSocket);

    const { result } = renderVoiceChat();
    await act(async () => {
      result.current.start();
    });
    await flushTimers();

    expect(mockFetch).toHaveBeenCalledWith(
      "http://127.0.0.1:8080/v1/realtime-ticket",
      expect.objectContaining({
        method: "POST",
        headers: { Authorization: "Bearer sk-test-key-123" },
      }),
    );
    expect(wsConstructorUrl).toContain("?ticket=ticket-abc-123");
    expect(mockMediaDevices).toHaveBeenCalledTimes(1);
  });

  it("sends session.create on WebSocket open", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ ticket: "ticket-xyz" }),
    });
    vi.stubGlobal("WebSocket", createMockWs as unknown as typeof WebSocket);

    const { result } = renderVoiceChat();
    await act(async () => {
      result.current.start();
    });
    await flushTimers();

    expect(mockWsInstances[0].send).toHaveBeenCalledWith(
      JSON.stringify({
        type: "session.create",
        model: defaultConfig.modelId,
        voice: null,
        stt_provider: null,
        tts_provider: null,
      }),
    );
  });

  it("transitions to Connected on session.created", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ ticket: "ticket-xyz" }),
    });
    vi.stubGlobal("WebSocket", createMockWs as unknown as typeof WebSocket);

    const { result } = renderVoiceChat();
    await act(async () => {
      result.current.start();
    });
    await flushTimers();

    await act(async () => {
      simulateWsMessage("session.created", { session_id: "sess-1" });
    });

    expect(result.current.state).toBe("Connected");
  });

  // ═══════════════════════════════════════════════════════════
  // 音频响应消息处理
  // ═══════════════════════════════════════════════════════════

  it("transitions to Listening on response.audio.delta", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ ticket: "ticket-xyz" }),
    });
    vi.stubGlobal("WebSocket", createMockWs as unknown as typeof WebSocket);

    const { result } = renderVoiceChat();
    await act(async () => {
      result.current.start();
    });
    await flushTimers();
    await act(async () => simulateWsMessage("session.created"));
    await act(async () => simulateWsMessage("response.audio.delta", { delta: "AAAA" }));

    expect(result.current.state).toBe("Listening");
  });

  it("transitions to Speaking on response.done", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ ticket: "ticket-xyz" }),
    });
    vi.stubGlobal("WebSocket", createMockWs as unknown as typeof WebSocket);

    const { result } = renderVoiceChat();
    await act(async () => {
      result.current.start();
    });
    await flushTimers();
    await act(async () => simulateWsMessage("session.created"));
    await act(async () => simulateWsMessage("response.done"));

    expect(result.current.state).toBe("Speaking");
  });

  // ═══════════════════════════════════════════════════════════
  // stop()
  // ═══════════════════════════════════════════════════════════

  it("resets to Idle on stop after connection", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ ticket: "ticket-xyz" }),
    });
    vi.stubGlobal("WebSocket", createMockWs as unknown as typeof WebSocket);

    const { result } = renderVoiceChat();
    await act(async () => {
      result.current.start();
    });
    await flushTimers();

    expect(["Connecting", "Connected"]).toContain(result.current.state);

    await act(async () => {
      result.current.stop();
    });
    expect(result.current.state).toBe("Idle");
  });

  it("is a no-op when already Idle", () => {
    const { result } = renderVoiceChat();
    expect(result.current.state).toBe("Idle");
    act(() => result.current.stop());
    expect(result.current.state).toBe("Idle");
  });

  // ═══════════════════════════════════════════════════════════
  // toggleMute()
  // ═══════════════════════════════════════════════════════════

  it("toggles mute state on each call", () => {
    const { result } = renderVoiceChat();
    expect(result.current.isMuted).toBe(false);
    act(() => result.current.toggleMute());
    expect(result.current.isMuted).toBe(true);
    act(() => result.current.toggleMute());
    expect(result.current.isMuted).toBe(false);
  });

  // ═══════════════════════════════════════════════════════════
  // 错误处理
  // ═══════════════════════════════════════════════════════════

  it("transitions to Error when ticket request fails", async () => {
    mockFetch.mockRejectedValueOnce(new Error("Network unreachable"));

    const { result } = renderVoiceChat();
    await act(async () => {
      result.current.start();
    });
    await flushTimers();

    expect(messageError).toHaveBeenCalled();
    expect(result.current.state).toBe("Error");
  });

  it("transitions to Error when ticket response is not ok", async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 401 });

    const { result } = renderVoiceChat();
    await act(async () => {
      result.current.start();
    });
    await flushTimers();

    expect(messageError).toHaveBeenCalled();
    expect(result.current.state).toBe("Error");
  });

  // ═══════════════════════════════════════════════════════════
  // 空闲重入防护
  // ═══════════════════════════════════════════════════════════

  it("ignores start() when not in Idle state", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ ticket: "ticket-xyz" }),
    });
    vi.stubGlobal("WebSocket", createMockWs as unknown as typeof WebSocket);

    const { result } = renderVoiceChat();
    await act(async () => {
      result.current.start();
    });
    await flushTimers();

    const wsCountBefore = mockWsInstances.length;

    await act(async () => {
      result.current.start();
    });
    await flushTimers();

    expect(mockWsInstances.length).toBe(wsCountBefore);
  });

  // ═══════════════════════════════════════════════════════════
  // 音频转发（worklet → WebSocket）
  // ═══════════════════════════════════════════════════════════

  it("forwards audio data from worklet to WebSocket when not muted", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ ticket: "ticket-xyz" }),
    });
    vi.stubGlobal("WebSocket", createMockWs as unknown as typeof WebSocket);

    const { result } = renderVoiceChat();
    await act(async () => {
      result.current.start();
    });
    await flushTimers();

    const worklet = mockWorkletInstances[0];
    expect(worklet).toBeDefined();
    expect(worklet.port.onmessage).not.toBeNull();

    // 手动触发 worklet port 消息
    const buf = new ArrayBuffer(4);
    mockWsInstances[0].send.mockClear();

    // 通过 worklet 的端口模拟 AudioWorklet 发送 PCM16 数据
    const handler = worklet.port.onmessage!;
    handler(new MessageEvent("message", { data: buf }));

    // 验证 WS send 被调用
    expect(mockWsInstances[0].send).toHaveBeenCalledTimes(1);
    expect(mockWsInstances[0].send).toHaveBeenCalledWith(buf);
  });

  it("does NOT forward audio data when muted", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ ticket: "ticket-xyz" }),
    });
    vi.stubGlobal("WebSocket", createMockWs as unknown as typeof WebSocket);

    const { result } = renderVoiceChat();
    await act(async () => {
      result.current.start();
    });
    await flushTimers();

    act(() => result.current.toggleMute());

    const worklet = mockWorkletInstances[0];
    mockWsInstances[0].send.mockClear();

    const buf = new ArrayBuffer(4);
    await act(async () => {
      worklet.port.onmessage!(new MessageEvent("message", { data: buf }));
    });

    expect(mockWsInstances[0].send).not.toHaveBeenCalled();
  });

  // ═══════════════════════════════════════════════════════════
  // 重连（使用 fake timers 控制异步间隔）
  // ═══════════════════════════════════════════════════════════

  it("reconnects with exponential backoff on unexpected close", { timeout: 10000 }, async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });

    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ ticket: "ticket-xyz" }),
    });
    vi.stubGlobal("WebSocket", createMockWs as unknown as typeof WebSocket);

    const { result } = renderVoiceChat();
    await act(async () => {
      result.current.start();
      await vi.advanceTimersByTimeAsync(100);
    });

    const wsCountBefore = mockWsInstances.length;

    await act(async () => {
      mockWsInstances[0].onclose?.(new CloseEvent("close", { code: 1006, reason: "Abnormal" }));
      await vi.advanceTimersByTimeAsync(100);
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });

    expect(mockWsInstances.length).toBe(wsCountBefore + 1);

    vi.useRealTimers();
  });
});
