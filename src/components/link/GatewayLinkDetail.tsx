import { useGatewayLinkStore } from "@/stores";
import { Button, Tabs, Tag, theme } from "antd";
import { Bot, Gauge, MessageSquarePlus, Shield, Sparkles } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { LinkModels } from "./LinkModels";
import { LinkOverview } from "./LinkOverview";
import { LinkPolicies } from "./LinkPolicies";
import { LinkSkills } from "./LinkSkills";

export function GatewayLinkDetail() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();
  const selectedLinkId = useGatewayLinkStore((s) => s.selectedLinkId);
  const links = useGatewayLinkStore((s) => s.links);
  const createGatewayConversation = useGatewayLinkStore((s) => s.createGatewayConversation);

  const selectedLink = links.find((l) => l.id === selectedLinkId);

  if (!selectedLink) {
    return (
      <div
        className="flex h-full items-center justify-center"
        style={{ color: token.colorTextSecondary }}
      >
        <p>{t("link.selectGateway")}</p>
      </div>
    );
  }

  const handleNewConversation = async () => {
    try {
      const conversationId = await createGatewayConversation(selectedLink.id);
      navigate(`/?conversation=${conversationId}`);
    } catch {
      // error handled in store
    }
  };

  const items = [
    {
      key: "overview",
      label: t("link.overview"),
      icon: <Gauge size={16} />,
      children: <LinkOverview link={selectedLink} />,
    },
    {
      key: "models",
      label: t("link.models"),
      icon: <Bot size={16} />,
      children: <LinkModels link={selectedLink} />,
    },
    {
      key: "skills",
      label: t("link.skills"),
      icon: <Sparkles size={16} />,
      children: <LinkSkills link={selectedLink} />,
    },
    {
      key: "policies",
      label: t("link.policies"),
      icon: <Shield size={16} />,
      children: <LinkPolicies link={selectedLink} />,
    },
  ];

  return (
    <div className="flex flex-col h-full" style={{ overflow: "hidden" }}>
      <div
        className="flex items-center justify-between px-4 py-3"
        style={{ borderBottom: `1px solid ${token.colorBorderSecondary}`, flexShrink: 0 }}
      >
        <div>
          <div className="flex items-center gap-2">
            <span style={{ fontWeight: 600, fontSize: 15 }}>{selectedLink.name}</span>
            <Tag
              color={selectedLink.status === "connected"
                ? "green"
                : selectedLink.status === "error"
                ? "red"
                : "default"}
            >
              {t(`link.status${selectedLink.status.charAt(0).toUpperCase()}${selectedLink.status.slice(1)}`)}
            </Tag>
          </div>
          <div style={{ fontSize: 12, color: token.colorTextTertiary, fontFamily: "monospace" }}>
            {selectedLink.endpoint}
          </div>
        </div>
        <Button
          type="primary"
          icon={<MessageSquarePlus size={14} />}
          onClick={handleNewConversation}
          disabled={selectedLink.status !== "connected"}
        >
          {t("link.newConversation")}
        </Button>
      </div>
      <div className="flex-1 overflow-y-auto px-2">
        <Tabs
          items={items}
          className="link-detail-tabs"
          style={{ minHeight: 0 }}
        />
      </div>
      <style>
        {`
        .link-detail-tabs > .ant-tabs-content-holder {
          overflow-y: auto;
        }
      `}
      </style>
    </div>
  );
}
