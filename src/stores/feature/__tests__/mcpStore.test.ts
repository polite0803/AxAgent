// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@/lib/invoke", () => ({
  invoke: invokeMock,
  isTauri: () => false,
}));

import { useMcpStore } from "@/stores/feature/mcpStore";

const SERVER_ID = "srv-1";

function makeServer(overrides?: Record<string, unknown>) {
  return {
    id: SERVER_ID,
    name: "Test MCP Server",
    command: "node",
    argsJson: JSON.stringify(["server.js"]),
    transport: "stdio" as const,
    enabled: true,
    permissionPolicy: "ask" as const,
    source: "custom" as const,
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
    ...overrides,
  };
}

function makeToolDescriptor(overrides?: Record<string, unknown>) {
  return {
    id: "tool-1",
    serverId: SERVER_ID,
    name: "test_tool",
    description: "A test tool",
    inputSchemaJson: JSON.stringify({ type: "object", properties: {} }),
    ...overrides,
  };
}

function makeToolExecution(overrides?: Record<string, unknown>) {
  return {
    id: "exec-1",
    toolName: "test_tool",
    serverId: SERVER_ID,
    conversationId: "conv-1",
    status: "completed",
    result: "success",
    created_at: "2025-01-01T00:00:00Z",
    ...overrides,
  };
}

describe("mcpStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useMcpStore.setState({
      servers: [],
      toolDescriptors: {},
      toolExecutions: [],
      loading: false,
      error: null,
    });
  });

  describe("loadServers", () => {
    it("loads servers from backend", async () => {
      const servers = [makeServer(), makeServer({ id: "srv-2", name: "Server 2" })];
      invokeMock.mockResolvedValueOnce(servers);

      await useMcpStore.getState().loadServers();

      expect(invokeMock).toHaveBeenCalledWith("list_mcp_servers");
      expect(useMcpStore.getState().servers).toEqual(servers);
      expect(useMcpStore.getState().loading).toBe(false);
    });

    it("sets error on failure", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Network error"));

      await useMcpStore.getState().loadServers();

      expect(useMcpStore.getState().error).toBe("Error: Network error");
      expect(useMcpStore.getState().loading).toBe(false);
    });
  });

  describe("createServer", () => {
    it("creates a server and adds to store", async () => {
      const server = makeServer();
      invokeMock.mockResolvedValueOnce(server);

      const result = await useMcpStore.getState().createServer({
        name: "New Server",
        command: "node",
        transport: "stdio",
      });

      expect(invokeMock).toHaveBeenCalledWith("create_mcp_server", {
        input: { name: "New Server", command: "node", transport: "stdio" },
      });
      expect(result).toEqual(server);
      expect(useMcpStore.getState().servers).toContainEqual(server);
    });

    it("returns null on failure", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Create failed"));

      const result = await useMcpStore.getState().createServer({
        name: "New",
        command: "node",
        transport: "stdio",
      });

      expect(result).toBeNull();
      expect(useMcpStore.getState().error).toBe("Error: Create failed");
    });
  });

  describe("updateServer", () => {
    it("updates a server in store", async () => {
      const server = makeServer();
      const updated = makeServer({ name: "Updated Server" });
      useMcpStore.setState({ servers: [server] });

      invokeMock.mockResolvedValueOnce(updated);

      await useMcpStore.getState().updateServer(SERVER_ID, { name: "Updated Server" });

      expect(invokeMock).toHaveBeenCalledWith("update_mcp_server", {
        id: SERVER_ID,
        input: { name: "Updated Server" },
      });
      expect(useMcpStore.getState().servers[0].name).toBe("Updated Server");
    });
  });

  describe("deleteServer", () => {
    it("deletes a server and removes from store", async () => {
      const server = makeServer();
      useMcpStore.setState({
        servers: [server],
        toolDescriptors: { [SERVER_ID]: [makeToolDescriptor()] },
      });

      invokeMock.mockResolvedValueOnce(undefined);

      await useMcpStore.getState().deleteServer(SERVER_ID);

      expect(invokeMock).toHaveBeenCalledWith("delete_mcp_server", { id: SERVER_ID });
      expect(useMcpStore.getState().servers).toHaveLength(0);
      expect(useMcpStore.getState().toolDescriptors[SERVER_ID]).toBeUndefined();
    });
  });

  describe("testServer", () => {
    it("tests a server connection successfully", async () => {
      invokeMock.mockResolvedValueOnce({ ok: true });

      const result = await useMcpStore.getState().testServer(SERVER_ID);

      expect(invokeMock).toHaveBeenCalledWith("test_mcp_server", { id: SERVER_ID });
      expect(result).toEqual({ ok: true });
    });

    it("returns error on failure", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Connection refused"));

      const result = await useMcpStore.getState().testServer(SERVER_ID);

      expect(result).toEqual({ ok: false, error: "Error: Connection refused" });
    });
  });

  describe("loadToolDescriptors", () => {
    it("loads tool descriptors for a server", async () => {
      const tools = [makeToolDescriptor(), makeToolDescriptor({ name: "tool_2" })];
      invokeMock.mockResolvedValueOnce(tools);

      await useMcpStore.getState().loadToolDescriptors(SERVER_ID);

      expect(invokeMock).toHaveBeenCalledWith("list_mcp_tools", { serverId: SERVER_ID });
      expect(useMcpStore.getState().toolDescriptors[SERVER_ID]).toEqual(tools);
    });
  });

  describe("discoverTools", () => {
    it("discovers tools and stores them", async () => {
      const tools = [makeToolDescriptor()];
      invokeMock.mockResolvedValueOnce(tools);

      const result = await useMcpStore.getState().discoverTools(SERVER_ID);

      expect(invokeMock).toHaveBeenCalledWith("discover_mcp_tools", { id: SERVER_ID });
      expect(result).toEqual(tools);
      expect(useMcpStore.getState().toolDescriptors[SERVER_ID]).toEqual(tools);
    });
  });

  describe("loadToolExecutions", () => {
    it("loads tool executions for a conversation", async () => {
      const executions = [makeToolExecution(), makeToolExecution({ id: "exec-2" })];
      invokeMock.mockResolvedValueOnce(executions);

      await useMcpStore.getState().loadToolExecutions("conv-1");

      expect(invokeMock).toHaveBeenCalledWith("list_tool_executions", { conversationId: "conv-1" });
      expect(useMcpStore.getState().toolExecutions).toEqual(executions);
    });
  });

  describe("discoverAvailableServers", () => {
    it("discovers available servers", async () => {
      const discovered = [
        {
          name: "server-1",
          packageName: "pkg1",
          description: "Test",
          command: "node",
          args: [],
          transport: "stdio",
        },
      ];
      invokeMock.mockResolvedValueOnce(discovered);

      const result = await useMcpStore.getState().discoverAvailableServers();

      expect(invokeMock).toHaveBeenCalledWith("discover_available_mcp_servers");
      expect(result).toEqual(discovered);
    });

    it("returns empty array on failure", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Failed"));

      const result = await useMcpStore.getState().discoverAvailableServers();

      expect(result).toEqual([]);
    });
  });
});
