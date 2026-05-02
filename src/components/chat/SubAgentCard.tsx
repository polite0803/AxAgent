import type { SubAgentCardData } from "@/types/agent";
import { CheckCircleOutlined, CloseCircleOutlined, LoadingOutlined, RightOutlined } from "@ant-design/icons";
import { useNavigate } from "react-router-dom";
import "./SubAgentCard.css";

const AGENT_COLORS: Record<string, string> = {
  explore: "#1890ff",
  general: "#722ed1",
  build: "#52c41a",
  plan: "#fa8c16",
  research: "#eb2f96",
  review: "#13c2c2",
};

function getAgentColor(agentType: string): string {
  return AGENT_COLORS[agentType] ?? hashColor(agentType);
}

function hashColor(str: string): string {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = str.charCodeAt(i) + ((hash << 5) - hash);
  }
  const hue = hash % 360;
  return `hsl(${Math.abs(hue)}, 60%, 45%)`;
}

function getAgentIcon(agentType: string): string {
  switch (agentType) {
    case "explore":
      return "🔍";
    case "general":
      return "🔧";
    case "build":
      return "🏗";
    case "plan":
      return "📋";
    case "research":
      return "🔬";
    case "review":
      return "✅";
    default:
      return "🤖";
  }
}

interface SubAgentCardProps {
  card: SubAgentCardData;
}

export function SubAgentCard({ card }: SubAgentCardProps) {
  const navigate = useNavigate();
  const color = getAgentColor(card.agentType);
  const icon = getAgentIcon(card.agentType);
  const isRunning = card.status === "running";
  const isCompleted = card.status === "completed";
  const isFailed = card.status === "failed";

  const handleClick = () => {
    if (card.childConversationId && !isRunning) {
      navigate(`/chat/${card.childConversationId}`);
    }
  };

  return (
    <div
      className={`sub-agent-card ${isRunning ? "sub-agent-card--running" : ""} ${
        isCompleted ? "sub-agent-card--completed" : ""
      } ${isFailed ? "sub-agent-card--failed" : ""}`}
      onClick={handleClick}
      style={{ cursor: isRunning ? "default" : "pointer" }}
      data-component="sub-agent-card"
    >
      <div className="sub-agent-card__header">
        <span className="sub-agent-card__icon">{icon}</span>
        <span className="sub-agent-card__name" style={{ color }}>
          {card.agentName || card.agentType}
        </span>
        <span className="sub-agent-card__status">
          {isRunning && <LoadingOutlined spin style={{ color }} />}
          {isCompleted && <CheckCircleOutlined style={{ color: "#52c41a" }} />}
          {isFailed && <CloseCircleOutlined style={{ color: "#ff4d4f" }} />}
        </span>
      </div>
      <div className="sub-agent-card__desc">{card.description}</div>
      {isCompleted && card.childConversationId && (
        <div className="sub-agent-card__action">
          查看子会话 <RightOutlined />
        </div>
      )}
    </div>
  );
}
