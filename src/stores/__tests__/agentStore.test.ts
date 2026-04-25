import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useAgentStore, setupAgentEventListeners } from '@/stores';
import { listen } from '@/lib/invoke';

// Mock the invoke and listen functions
vi.mock('@/lib/invoke', () => ({
  invoke: vi.fn(),
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

describe('agentStore event handling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset the store state between tests
    const { result } = renderHook(() => useAgentStore());
    act(() => {
      // Clear all conversations to reset state
      for (const convId of Object.keys(result.current.agentStatus)) {
        result.current.clearStatus(convId);
      }
    });
  });

  it('should handle tool use event', () => {
    const { result } = renderHook(() => useAgentStore());

    const toolUseEvent = {
      conversationId: 'conv1',
      assistantMessageId: 'msg1',
      toolUseId: 'tool1',
      toolName: 'echo',
      input: { text: 'Hello' },
      executionId: 'exec1',
    };

    act(() => {
      result.current.handleToolUse(toolUseEvent);
    });

    expect(result.current.toolCalls['tool1']).toEqual({
      toolUseId: 'tool1',
      toolName: 'echo',
      input: { text: 'Hello' },
      assistantMessageId: 'msg1',
      executionStatus: 'queued',
    });

    expect(result.current.toolCalls['exec1']).toEqual({
      toolUseId: 'exec1',
      toolName: 'echo',
      input: { text: 'Hello' },
      assistantMessageId: 'msg1',
      executionStatus: 'queued',
    });

    expect(result.current.sdkIdToExecId['tool1']).toBe('exec1');
  });

  it('should handle tool start event', () => {
    const { result } = renderHook(() => useAgentStore());

    // First, add a tool call
    const toolUseEvent = {
      conversationId: 'conv1',
      assistantMessageId: 'msg1',
      toolUseId: 'tool1',
      toolName: 'echo',
      input: { text: 'Hello' },
    };

    act(() => {
      result.current.handleToolUse(toolUseEvent);
    });

    // Then handle tool start event
    const toolStartEvent = {
      conversationId: 'conv1',
      assistantMessageId: 'msg1',
      toolUseId: 'tool1',
      toolName: 'echo',
      input: { text: 'Hello' },
    };

    act(() => {
      result.current.handleToolStart(toolStartEvent);
    });

    expect(result.current.toolCalls['tool1'].executionStatus).toBe('running');
  });

  it('should handle tool result event', () => {
    const { result } = renderHook(() => useAgentStore());

    // First, add a tool call
    const toolUseEvent = {
      conversationId: 'conv1',
      assistantMessageId: 'msg1',
      toolUseId: 'tool1',
      toolName: 'echo',
      input: { text: 'Hello' },
    };

    act(() => {
      result.current.handleToolUse(toolUseEvent);
    });

    // Then handle tool result event
    const toolResultEvent = {
      conversationId: 'conv1',
      assistantMessageId: 'msg1',
      toolUseId: 'tool1',
      toolName: 'echo',
      content: 'Hello',
      isError: false,
    };

    act(() => {
      result.current.handleToolResult(toolResultEvent);
    });

    expect(result.current.toolCalls['tool1'].executionStatus).toBe('success');
    expect(result.current.toolCalls['tool1'].output).toBe('Hello');
    expect(result.current.toolCalls['tool1'].isError).toBe(false);
  });

  it('should handle permission request event', () => {
    const { result } = renderHook(() => useAgentStore());

    const permissionEvent = {
      conversationId: 'conv1',
      assistantMessageId: 'msg1',
      toolUseId: 'tool1',
      toolName: 'write',
      input: { path: 'test.txt', content: 'Hello' },
      riskLevel: 'write' as const,
      requestId: 'perm_1',
    };

    act(() => {
      result.current.handlePermissionRequest(permissionEvent);
    });

    // Key is requestId when present
    expect(result.current.pendingPermissions['perm_1']).toEqual(permissionEvent);
  });

  it('should handle permission resolved', () => {
    const { result } = renderHook(() => useAgentStore());

    // First, add a permission request (using toolUseId as key since no requestId)
    const permissionEvent = {
      conversationId: 'conv1',
      assistantMessageId: 'msg1',
      toolUseId: 'tool1',
      toolName: 'write',
      input: { path: 'test.txt', content: 'Hello' },
      riskLevel: 'write' as const,
    };

    act(() => {
      result.current.handlePermissionRequest({ ...permissionEvent, requestId: 'req1' });
    });

    // Then add the tool call
    const toolUseEvent = {
      conversationId: 'conv1',
      assistantMessageId: 'msg1',
      toolUseId: 'tool1',
      toolName: 'write',
      input: { path: 'test.txt', content: 'Hello' },
    };

    act(() => {
      result.current.handleToolUse(toolUseEvent);
    });

    // Then resolve the permission
    act(() => {
      result.current.handlePermissionResolved('tool1', 'allow_once');
    });

    expect(result.current.pendingPermissions['tool1']).toBeUndefined();
    expect(result.current.toolCalls['tool1'].approvalStatus).toBe('approved');
  });

  it('should handle done event and record queryStats', () => {
    const { result } = renderHook(() => useAgentStore());

    const doneEvent = {
      conversationId: 'conv1',
      assistantMessageId: 'msg1',
      text: 'Hello World!',
      usage: { input_tokens: 10, output_tokens: 5 },
      numTurns: 1,
    };

    act(() => {
      result.current.handleDone(doneEvent);
    });

    expect(result.current.queryStats['msg1']).toEqual({
      numTurns: 1,
      inputTokens: 10,
      outputTokens: 5,
    });
  });

  it('should handle done event with cost', () => {
    const { result } = renderHook(() => useAgentStore());

    const doneEvent = {
      conversationId: 'conv1',
      assistantMessageId: 'msg1',
      text: 'Hello World!',
      usage: { input_tokens: 100, output_tokens: 50 },
      numTurns: 3,
      costUsd: 0.005,
    };

    act(() => {
      result.current.handleDone(doneEvent);
    });

    expect(result.current.queryStats['msg1']).toEqual({
      numTurns: 3,
      inputTokens: 100,
      outputTokens: 50,
      costUsd: 0.005,
    });
  });

  it('should handle cancelled event', () => {
    const { result } = renderHook(() => useAgentStore());

    // Set a status first
    act(() => {
      result.current.handleStatus('conv1', 'Running tool...');
    });

    expect(result.current.agentStatus['conv1']).toBe('Running tool...');

    const cancelledEvent = {
      conversationId: 'conv1',
      reason: 'User cancelled',
    };

    act(() => {
      result.current.handleCancelled(cancelledEvent);
    });

    expect(result.current.agentStatus['conv1']).toBeUndefined();
  });

  it('should handle rate limit event', () => {
    const { result } = renderHook(() => useAgentStore());

    const rateLimitEvent = {
      conversationId: 'conv1',
      retryAfterMs: 5000,
      message: 'Rate limited, retry in 5s',
    };

    act(() => {
      result.current.handleRateLimit(rateLimitEvent);
    });

    expect(result.current.rateLimitInfo['conv1']).toEqual(rateLimitEvent);
  });

  it('should clear conversation state', () => {
    const { result } = renderHook(() => useAgentStore());

    // Set up some state
    act(() => {
      result.current.handleStatus('conv1', 'Running...');
      result.current.handlePermissionRequest({
        conversationId: 'conv1',
        assistantMessageId: 'msg1',
        toolUseId: 'tool1',
        toolName: 'write',
        input: { path: 'test.txt' },
        requestId: 'req2',
        riskLevel: 'write' as const,
      });
    });

    expect(result.current.agentStatus['conv1']).toBe('Running...');
    expect(Object.keys(result.current.pendingPermissions).length).toBeGreaterThan(0);

    // Clear the conversation
    act(() => {
      result.current.clearConversation('conv1');
    });

    expect(result.current.agentStatus['conv1']).toBeUndefined();
    expect(Object.keys(result.current.pendingPermissions).length).toBe(0);
  });

  it('should setup event listeners', () => {
    const unlistenFn = vi.fn();
    (listen as unknown as ReturnType<typeof vi.fn>).mockReturnValue(Promise.resolve(unlistenFn));

    const cleanup = setupAgentEventListeners();

    // 12 events: tool-use, tool-start, tool-result, permission-request,
    // ask-user, status, done, error, cancelled, rate-limit, agent-query
    expect(listen).toHaveBeenCalledTimes(12);

    cleanup();
  });
});
