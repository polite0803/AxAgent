/**
 * AxAgent SDK — TypeScript 客户端 SDK
 *
 * 允许外部程序通过 ACP 协议与 AxAgent 交互。
 *
 * @example
 * ```typescript
 * import { AxAgentClient } from './sdk';
 *
 * const client = new AxAgentClient('http://localhost:9876');
 * const session = await client.createSession({ workDir: '/path/to/project' });
 * const result = await client.sendPrompt(session.sessionId, '帮我分析项目结构');
 * console.log(result.content);
 * await client.closeSession(session.sessionId);
 * ```
 */

// ── 类型定义 ──

/** ACP 协议版本 */
export const ACP_VERSION = '1.0.0';

/** 会话状态 */
export type SessionStatus = 'idle' | 'running' | 'waiting_permission' | 'compacting' | 'closed';

/** 权限模式 */
export type PermissionMode = 'read-only' | 'workspace-write' | 'danger-full-access';

/** 创建会话参数 */
export interface CreateSessionParams {
  workDir: string;
  model?: string;
  permissionMode?: PermissionMode;
  systemPrompt?: string;
}

/** 会话信息 */
export interface Session {
  sessionId: string;
  workDir: string;
  status: SessionStatus;
  createdAt: string;
  lastActive: string;
  permissionMode: string;
  activeTasks: number;
}

/** Prompt 响应 */
export interface PromptResult {
  sessionId: string;
  content: string;
  toolCalls: ToolCallRecord[];
  turns: number;
  tokensUsed: number;
}

/** 工具调用记录 */
export interface ToolCallRecord {
  toolName: string;
  toolInput: Record<string, unknown>;
  toolResult?: string;
  isError: boolean;
}

/** Hook 注册参数 */
export interface RegisterHookParams {
  sessionId: string;
  event: string;
  callbackUrl: string;
}

/** ACP 通知 */
export interface AcpNotification {
  event: string;
  sessionId: string;
  data: Record<string, unknown>;
  timestamp: string;
}

// ── SDK 客户端 ──

export class AxAgentClient {
  private baseUrl: string;
  private authToken?: string;

  constructor(baseUrl: string, authToken?: string) {
    this.baseUrl = baseUrl.replace(/\/$/, '');
    this.authToken = authToken;
  }

  private headers(): Record<string, string> {
    const h: Record<string, string> = { 'Content-Type': 'application/json' };
    if (this.authToken) {
      h['Authorization'] = `Bearer ${this.authToken}`;
    }
    return h;
  }

  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const response = await fetch(url, {
      method,
      headers: this.headers(),
      body: body ? JSON.stringify(body) : undefined,
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ message: response.statusText }));
      throw new Error(`ACP 请求失败: ${(error as { message?: string }).message || response.statusText}`);
    }

    return response.json();
  }

  /** 创建会话 */
  async createSession(params: CreateSessionParams): Promise<Session> {
    return this.request<Session>('POST', '/acp/v1/sessions', params);
  }

  /** 查询会话 */
  async getSession(sessionId: string): Promise<Session> {
    return this.request<Session>('GET', `/acp/v1/sessions/${sessionId}`);
  }

  /** 列出所有会话 */
  async listSessions(): Promise<Session[]> {
    return this.request<Session[]>('GET', '/acp/v1/sessions');
  }

  /** 发送 prompt */
  async sendPrompt(sessionId: string, prompt: string, maxTurns?: number): Promise<PromptResult> {
    return this.request<PromptResult>('POST', `/acp/v1/sessions/${sessionId}/prompts`, {
      sessionId,
      prompt,
      maxTurns,
    });
  }

  /** 中断执行 */
  async interrupt(sessionId: string): Promise<void> {
    await this.request('POST', `/acp/v1/sessions/${sessionId}/interrupt`);
  }

  /** 关闭会话 */
  async closeSession(sessionId: string): Promise<void> {
    await this.request('POST', `/acp/v1/sessions/${sessionId}/close`);
  }

  /** 注册 hook 回调 */
  async registerHook(params: RegisterHookParams): Promise<void> {
    await this.request('POST', '/acp/v1/hooks', params);
  }

  /** 健康检查 */
  async healthCheck(): Promise<boolean> {
    try {
      await this.getSession('health');
      return true;
    } catch {
      return false;
    }
  }

  /**
   * 连接到 WebSocket 事件流
   * 返回一个 AsyncIterator，可用于 for-await-of 循环
   */
  async *connectWebSocket(): AsyncGenerator<AcpNotification> {
    const wsUrl = this.baseUrl.replace(/^http/, 'ws') + '/acp/v1/ws';
    const ws = new WebSocket(wsUrl);

    const messageQueue: AcpNotification[] = [];
    let resolveNext: ((value: AcpNotification) => void) | null = null;
    let done = false;

    ws.onmessage = (event) => {
      const notification: AcpNotification = JSON.parse(event.data);
      if (resolveNext) {
        resolveNext(notification);
        resolveNext = null;
      } else {
        messageQueue.push(notification);
      }
    };

    ws.onclose = () => {
      done = true;
      if (resolveNext) {
        resolveNext({} as AcpNotification);
        resolveNext = null;
      }
    };

    ws.onerror = () => {
      done = true;
      if (resolveNext) {
        resolveNext({} as AcpNotification);
        resolveNext = null;
      }
    };

    try {
      while (!done) {
        if (messageQueue.length > 0) {
          yield messageQueue.shift()!;
        } else {
          yield await new Promise<AcpNotification>((resolve) => {
            resolveNext = resolve;
          });
        }
      }
    } finally {
      ws.close();
    }
  }
}

// ── React Hook ──

import { useState, useCallback, useRef } from 'react';

/**
 * useAxAgent — React Hook，简化 ACP 客户端在 React 组件中的使用
 */
export function useAxAgent(baseUrl: string) {
  const clientRef = useRef<AxAgentClient>(new AxAgentClient(baseUrl));
  const [sessions, setSessions] = useState<Session[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const createSession = useCallback(async (params: CreateSessionParams) => {
    setLoading(true);
    setError(null);
    try {
      const session = await clientRef.current.createSession(params);
      setSessions((prev) => [...prev, session]);
      return session;
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      throw e;
    } finally {
      setLoading(false);
    }
  }, []);

  const sendPrompt = useCallback(async (sessionId: string, prompt: string) => {
    setLoading(true);
    setError(null);
    try {
      return await clientRef.current.sendPrompt(sessionId, prompt);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      throw e;
    } finally {
      setLoading(false);
    }
  }, []);

  const closeSession = useCallback(async (sessionId: string) => {
    await clientRef.current.closeSession(sessionId);
    setSessions((prev) => prev.filter((s) => s.sessionId !== sessionId));
  }, []);

  const refreshSessions = useCallback(async () => {
    try {
      const list = await clientRef.current.listSessions();
      setSessions(list);
    } catch {
      // 静默失败
    }
  }, []);

  return {
    client: clientRef.current,
    sessions,
    loading,
    error,
    createSession,
    sendPrompt,
    closeSession,
    refreshSessions,
  };
}
