import { Card, Space, Tabs, Typography } from "antd";
import {
  Bot,
  Layers,
  MessageSquare,
  Network,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import AgentCommunicationGraph from "./AgentCommunicationGraph";
import ConsensusView from "./ConsensusView";
import MultiAgentStatusPanel from "./MultiAgentStatusPanel";

const { Title } = Typography;

interface AgentData {
  id: string;
  parent_id: string | null;
  name: string;
  description: string;
  status: "pending" | "running" | "completed" | "failed" | "cancelled";
  task: string | null;
  progress: number;
  result: string | null;
  error: string | null;
  children: string[];
  metadata: {
    agent_type: string;
    capabilities: string[];
    model: string | null;
    tools: string[];
  };
}

interface MessageData {
  id: string;
  from_agent: string;
  to_agent: string;
  kind: string;
  payload: string;
  timestamp: string;
}

interface ConsensusResultData {
  decision: "approved" | "rejected" | "pending" | "conflict";
  approve_count: number;
  reject_count: number;
  abstain_count: number;
  total_votes: number;
  threshold: number;
  strategy: { type: string; leader_role?: string; weights?: Record<string, number> };
  votes: Array<{
    agent_id: string;
    agent_name: string;
    agent_role: string;
    vote: "approve" | "reject" | "abstain";
    reason?: string;
    confidence: number;
  }>;
}

function MultiAgentDashboard() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<string>("overview");
  const [agents, setAgents] = useState<AgentData[]>([]);
  const [messages, setMessages] = useState<MessageData[]>([]);
  const [consensusResult, setConsensusResult] = useState<ConsensusResultData | null>(null);
  const [selectedAgentId, setSelectedAgentId] = useState<string | null>(null);

  useEffect(() => {
    const fetchData = async () => {
      try {
        const { invoke } = await import("@/lib/invoke");
        const agentList = await invoke<AgentData[]>("sub_agent_list").catch(() => []);
        setAgents(agentList);

        if (agentList.length > 0) {
          const targetId = selectedAgentId ?? agentList[0].id;
          const msgs = await invoke<MessageData[]>(
            "sub_agent_get_messages",
            { agentId: targetId }
          ).catch(() => []);
          setMessages(msgs);
        }

        const consensus = await invoke<ConsensusResultData | null>(
          "sub_agent_get_consensus"
        ).catch(() => null);
        setConsensusResult(consensus);
      } catch {
        // ignore
      }
    };

    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, [selectedAgentId]);

  const communicationAgents = agents.map((a) => ({
    id: a.id,
    name: a.name,
    role: a.metadata.agent_type,
    status: a.status,
    task: a.task || "",
  }));

  const communicationMessages = messages.map((m) => ({
    id: m.id,
    from_agent: m.from_agent,
    to_agent: m.to_agent,
    kind: m.kind,
    payload: m.payload,
    timestamp: m.timestamp,
  }));

  const tabItems = [
    {
      key: "overview",
      label: (
        <Space size="small">
          <Layers size={14} />
          <span>{t("chat.multiAgent.dashboard.overview")}</span>
        </Space>
      ),
      children: (
        <div className="space-y-3">
          <MultiAgentStatusPanel />
          {consensusResult && (
            <ConsensusView
              title={t("chat.multiAgent.consensus.title")}
              result={{
                ...consensusResult,
                strategy: consensusResult.strategy as any,
              }}
              onViewDetails={(agentId) => {
                setSelectedAgentId(agentId);
                setActiveTab("communication");
              }}
            />
          )}
        </div>
      ),
    },
    {
      key: "communication",
      label: (
        <Space size="small">
          <Network size={14} />
          <span>{t("chat.multiAgent.dashboard.communication")}</span>
        </Space>
      ),
      children: (
        <AgentCommunicationGraph
          agents={communicationAgents}
          messages={communicationMessages}
          highlightedAgentId={selectedAgentId ?? undefined}
          onAgentClick={setSelectedAgentId}
        />
      ),
    },
    {
      key: "messages",
      label: (
        <Space size="small">
          <MessageSquare size={14} />
          <span>{t("chat.multiAgent.dashboard.messages")}</span>
        </Space>
      ),
      children: (
        <MultiAgentStatusPanel />
      ),
    },
  ];

  return (
    <Card
      size="small"
      className="multi-agent-dashboard"
      title={
        <Space>
          <Bot size={16} className="text-purple-500" />
          <Title level={5} className="mb-0">
            {t("chat.multiAgent.dashboard.title")}
          </Title>
        </Space>
      }
    >
      <Tabs
        activeKey={activeTab}
        onChange={setActiveTab}
        items={tabItems}
        size="small"
      />
    </Card>
  );
}

export default MultiAgentDashboard;
