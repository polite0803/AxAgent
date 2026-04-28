import { Badge, Card, Space, Tooltip, Typography } from "antd";
import { ArrowRight, Bot, MessageSquare } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface AgentNode {
  id: string;
  name: string;
  role: string;
  status: "pending" | "starting" | "running" | "completed" | "failed" | "cancelled";
  task: string;
}

interface AgentMessage {
  id: string;
  from_agent: string;
  to_agent: string;
  kind: string;
  payload: string;
  timestamp: string;
}

interface AgentCommunicationGraphProps {
  agents: AgentNode[];
  messages: AgentMessage[];
  highlightedAgentId?: string;
  onAgentClick?: (agentId: string) => void;
}

const AGENT_COLORS: Record<string, string> = {
  Coordinator: "#722ed1",
  Researcher: "#1890ff",
  Developer: "#52c41a",
  Reviewer: "#faad14",
  Browser: "#eb2f96",
  Executor: "#13c2c2",
};

const STATUS_COLORS: Record<string, string> = {
  pending: "#8c8c8c",
  starting: "#1890ff",
  running: "#1890ff",
  completed: "#52c41a",
  failed: "#ff4d4f",
  cancelled: "#8c8c8c",
};

function AgentCommunicationGraph({
  agents,
  messages,
  highlightedAgentId,
  onAgentClick,
}: AgentCommunicationGraphProps) {
  const { t } = useTranslation();

  const { edges, stats } = useMemo(() => {
    const edgeMap = new Map<string, { count: number; kinds: Set<string> }>();
    for (const msg of messages) {
      const key = `${msg.from_agent}:${msg.to_agent}`;
      if (!edgeMap.has(key)) {
        edgeMap.set(key, { count: 0, kinds: new Set() });
      }
      const edge = edgeMap.get(key)!;
      edge.count++;
      edge.kinds.add(msg.kind);
    }

    const edges = Array.from(edgeMap.entries()).map(([key, value]) => {
      const [from, to] = key.split(":");
      return { from, to, count: value.count, kinds: Array.from(value.kinds) };
    });

    const totalMessages = messages.length;
    const completedAgents = agents.filter((a) => a.status === "completed").length;
    const runningAgents = agents.filter((a) => a.status === "running").length;
    const failedAgents = agents.filter((a) => a.status === "failed").length;

    return {
      edges,
      stats: { totalMessages, completedAgents, runningAgents, failedAgents },
    };
  }, [agents, messages]);

  const agentMap = useMemo(() => {
    const map = new Map<string, AgentNode>();
    for (const agent of agents) {
      map.set(agent.id, agent);
    }
    return map;
  }, [agents]);

  const getAgentColor = (role: string) => AGENT_COLORS[role] || "#8c8c8c";

  return (
    <Card size="small" className="agent-communication-graph">
      <div className="flex items-center justify-between mb-3">
        <Space>
          <MessageSquare size={16} className="text-purple-500" />
          <Text strong>{t("chat.multiAgent.communication.title")}</Text>
        </Space>
        <Space size="small">
          <Badge status="processing" text={<Text type="secondary" className="text-xs">{stats.runningAgents}</Text>} />
          <Badge status="success" text={<Text type="secondary" className="text-xs">{stats.completedAgents}</Text>} />
          <Badge status="error" text={<Text type="secondary" className="text-xs">{stats.failedAgents}</Text>} />
        </Space>
      </div>

      <div className="text-xs text-gray-500 mb-2">
        {t("chat.multiAgent.communication.messageCount", { count: stats.totalMessages })}
      </div>

      <div className="graph-visualization space-y-1">
        {agents.map((agent) => {
          const isHighlighted = highlightedAgentId === agent.id;
          const outgoingEdges = edges.filter((e) => e.from === agent.id);
          const incomingEdges = edges.filter((e) => e.to === agent.id);
          const agentColor = getAgentColor(agent.role);

          return (
            <div
              key={agent.id}
              className={`flex flex-col py-1 px-2 rounded cursor-pointer transition-colors ${
                isHighlighted
                  ? "bg-purple-50 dark:bg-purple-900/20 ring-1 ring-purple-300"
                  : "hover:bg-gray-50 dark:hover:bg-gray-800/50"
              }`}
              onClick={() => onAgentClick?.(agent.id)}
            >
              <div className="flex items-center gap-2">
                <div
                  className="w-2.5 h-2.5 rounded-full shrink-0"
                  style={{ backgroundColor: STATUS_COLORS[agent.status] }}
                />
                <Bot size={14} style={{ color: agentColor }} />
                <Text className="text-sm font-medium">{agent.name}</Text>
                <Badge
                  color={agentColor}
                  text={<Text className="text-xs">{agent.role}</Text>}
                />
                <Text type="secondary" className="text-xs truncate flex-1">
                  {agent.task}
                </Text>
              </div>

              {isHighlighted && (
                <div className="ml-6 mt-1 space-y-0.5">
                  {outgoingEdges.length > 0 && (
                    <div className="flex flex-wrap gap-2">
                      {outgoingEdges.map((edge, i) => {
                        const target = agentMap.get(edge.to);
                        return (
                          <Tooltip
                            key={i}
                            title={`${edge.count} messages: ${edge.kinds.join(", ")}`}
                          >
                            <div className="flex items-center gap-1 text-xs text-gray-500">
                              <ArrowRight size={10} />
                              <span style={{ color: target ? getAgentColor(target.role) : undefined }}>
                                {target?.name || edge.to.slice(0, 8)}
                              </span>
                              <span className="text-gray-400">({edge.count})</span>
                            </div>
                          </Tooltip>
                        );
                      })}
                    </div>
                  )}
                  {incomingEdges.length > 0 && (
                    <div className="flex flex-wrap gap-2">
                      {incomingEdges.map((edge, i) => {
                        const source = agentMap.get(edge.from);
                        return (
                          <Tooltip
                            key={i}
                            title={`${edge.count} messages: ${edge.kinds.join(", ")}`}
                          >
                            <div className="flex items-center gap-1 text-xs text-gray-500">
                              <span style={{ color: source ? getAgentColor(source.role) : undefined }}>
                                {source?.name || edge.from.slice(0, 8)}
                              </span>
                              <ArrowRight size={10} />
                              <span className="text-gray-400">({edge.count})</span>
                            </div>
                          </Tooltip>
                        );
                      })}
                    </div>
                  )}
                  {outgoingEdges.length === 0 && incomingEdges.length === 0 && (
                    <Text type="secondary" className="text-xs">
                      {t("chat.multiAgent.communication.noMessages")}
                    </Text>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {agents.length === 0 && (
        <div className="py-4 text-xs text-gray-400 text-center">
          {t("chat.multiAgent.communication.noAgents")}
        </div>
      )}
    </Card>
  );
}

export default AgentCommunicationGraph;
