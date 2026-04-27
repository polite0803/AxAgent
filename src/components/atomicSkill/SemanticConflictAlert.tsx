import { Alert, Typography } from "antd";
import React from "react";
import type { AtomicSkill } from "../../types";

const { Text } = Typography;

interface SemanticConflictAlertProps {
  conflict: AtomicSkill | null;
  onClose?: () => void;
}

export const SemanticConflictAlert: React.FC<SemanticConflictAlertProps> = ({ conflict, onClose }) => {
  if (!conflict) { return null; }

  return (
    <Alert
      type="error"
      showIcon
      closable={onClose ? true : false}
      onClose={onClose}
      style={{ marginBottom: 16 }}
      message="语义冲突检测"
      description={
        <div>
          <Text>已存在语义相同的原子Skill：</Text>
          <div style={{ marginTop: 8, padding: "8px 12px", background: "#fff1f0", borderRadius: 4 }}>
            <div>
              <Text strong>名称：</Text>
              {conflict.name}
            </div>
            <div>
              <Text strong>ID：</Text>
              <Text copyable={{ text: conflict.id }}>{conflict.id}</Text>
            </div>
            <div>
              <Text strong>入口类型：</Text>
              {conflict.entry_type}
            </div>
            <div>
              <Text strong>入口引用：</Text>
              {conflict.entry_ref}
            </div>
          </div>
          <Text type="secondary" style={{ fontSize: 12, display: "block", marginTop: 8 }}>
            请修改执行入口类型、入口引用或参数模式以避免冲突。
          </Text>
        </div>
      }
    />
  );
};
