import { DeleteOutlined } from "@ant-design/icons";
import { Divider, Popconfirm, theme, Typography } from "antd";
import { Focus, GitGraph, Link2, PenLine } from "lucide-react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface WikiNodeContextMenuProps {
  visible: boolean;
  position: { x: number; y: number };
  nodeId: string;
  nodeTitle: string;
  onClose: () => void;
  onEdit: (nodeId: string) => void;
  onViewBacklinks: (nodeId: string) => void;
  onFocusLocal: (nodeId: string) => void;
  onCreateLinked: (nodeId: string) => void;
  onDelete: (nodeId: string) => void;
}

export function WikiNodeContextMenu({
  visible,
  position,
  nodeId,
  nodeTitle,
  onClose,
  onEdit,
  onViewBacklinks,
  onFocusLocal,
  onCreateLinked,
  onDelete,
}: WikiNodeContextMenuProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();

  if (!visible) { return null; }

  const menuItemStyle: React.CSSProperties = {
    padding: "6px 12px",
    display: "flex",
    alignItems: "center",
    gap: 8,
    cursor: "pointer",
    fontSize: 13,
    borderRadius: 4,
    transition: "background-color 0.15s",
    whiteSpace: "nowrap",
  };

  const handleClick = (action: () => void) => {
    action();
    onClose();
  };

  return (
    <>
      {/* 遮罩层，点击关闭 */}
      <div
        onClick={onClose}
        onContextMenu={(e) => {
          e.preventDefault();
          onClose();
        }}
        style={{
          position: "fixed",
          inset: 0,
          zIndex: 999,
        }}
      />

      {/* 菜单 — 玻璃态 */}
      <div
        className="animate-[fadeIn_150ms_ease-out]"
        style={{
          position: "fixed",
          left: Math.min(position.x, window.innerWidth - 200),
          top: Math.min(position.y, window.innerHeight - 280),
          zIndex: 1000,
          backgroundColor: `${token.colorBgElevated}ee`,
          backdropFilter: "blur(16px)",
          WebkitBackdropFilter: "blur(16px)",
          border: `1px solid ${token.colorBorderSecondary}30`,
          borderRadius: 14,
          boxShadow: `0 8px 32px ${token.colorBgLayout}40, 0 2px 8px ${token.colorBgLayout}20`,
          padding: 6,
          minWidth: 190,
        }}
      >
        <div style={{ padding: "6px 12px 6px" }}>
          <Text strong style={{ fontSize: 12 }} ellipsis>
            {nodeTitle}
          </Text>
        </div>
        <Divider style={{ margin: "2px 0", borderColor: `${token.colorBorderSecondary}20` }} />

        <div
          style={menuItemStyle}
          onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = token.colorPrimaryBg)}
          onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "transparent")}
          onClick={() => handleClick(() => onEdit(nodeId))}
        >
          <PenLine size={14} />
          <span>{t("wiki.edit", "Edit Note")}</span>
        </div>

        <div
          style={menuItemStyle}
          onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = token.colorPrimaryBg)}
          onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "transparent")}
          onClick={() => handleClick(() => onViewBacklinks(nodeId))}
        >
          <GitGraph size={14} />
          <span>{t("wiki.viewBacklinks", "View Backlinks")}</span>
        </div>

        <div
          style={menuItemStyle}
          onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = token.colorPrimaryBg)}
          onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "transparent")}
          onClick={() => handleClick(() => onFocusLocal(nodeId))}
        >
          <Focus size={14} />
          <span>{t("wiki.focusLocal", "Focus Local Graph")}</span>
        </div>

        <div
          style={menuItemStyle}
          onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = token.colorPrimaryBg)}
          onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "transparent")}
          onClick={() => handleClick(() => onCreateLinked(nodeId))}
        >
          <Link2 size={14} />
          <span>{t("wiki.createLinkedNote", "Create Linked Note")}</span>
        </div>

        <Divider style={{ margin: "2px 0" }} />

        <Popconfirm
          title={t("wiki.confirmDelete", "Delete this note?")}
          onConfirm={() => handleClick(() => onDelete(nodeId))}
          okText={t("wiki.delete", "Delete")}
          cancelText={t("wiki.cancel", "Cancel")}
        >
          <div
            style={{ ...menuItemStyle, color: token.colorError }}
            onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = token.colorErrorBg)}
            onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "transparent")}
          >
            <DeleteOutlined />
            <span>{t("wiki.delete", "Delete")}</span>
          </div>
        </Popconfirm>
      </div>
    </>
  );
}
