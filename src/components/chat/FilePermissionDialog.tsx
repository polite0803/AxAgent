import { invoke } from "@tauri-apps/api/core";
import { Alert, Button, Descriptions, Input, Modal, Radio, Space, Tag, Typography } from "antd";
import { AlertTriangle, Clock, FileText, Shield } from "lucide-react";
import { useState } from "react";

interface AuthorizationResponse {
  authorized: boolean;
  auth_id?: string;
  path: string;
  level: string;
  expires_at?: string;
  message: string;
}

interface FilePermissionDialogProps {
  open: boolean;
  onClose: () => void;
  path: string;
  reason?: string;
  onAuthorize?: (authId: string) => void;
}

type PermissionLevel = "read" | "write" | "readwrite" | "temp";

export function FilePermissionDialog({
  open,
  onClose,
  path,
  reason = "",
  onAuthorize,
}: FilePermissionDialogProps) {
  const [level, setLevel] = useState<PermissionLevel>("temp");
  const [duration, setDuration] = useState(30);
  const [customReason, setCustomReason] = useState(reason);
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<AuthorizationResponse | null>(null);

  const handleAuthorize = async () => {
    setLoading(true);
    try {
      const response = await invoke<AuthorizationResponse>("file_authorize", {
        request: {
          path,
          level,
          reason: customReason,
          duration_minutes: level === "temp" ? duration : undefined,
          auto_renew: true,
        },
      });
      setResult(response);
      if (response.authorized && response.auth_id && onAuthorize) {
        onAuthorize(response.auth_id);
      }
    } catch (e) {
      setResult({
        authorized: false,
        path,
        level,
        message: String(e),
      });
    } finally {
      setLoading(false);
    }
  };

  const handleRevoke = async () => {
    if (result?.auth_id) {
      try {
        await invoke("file_revoke_authorization", { authId: result.auth_id });
        setResult(null);
        onClose();
      } catch (e) {
        console.error(e);
      }
    }
  };

  const levelLabels: Record<PermissionLevel, { label: string; desc: string }> = {
    read: { label: "只读", desc: "只能读取文件内容" },
    write: { label: "只写", desc: "只能写入文件内容" },
    readwrite: { label: "读写", desc: "可以读取和写入文件" },
    temp: { label: "临时授权", desc: "临时授权后自动回收" },
  };

  return (
    <Modal
      title={
        <Space>
          <Shield size={18} />
          <span>文件访问授权</span>
        </Space>
      }
      open={open}
      onCancel={onClose}
      footer={null}
      width={500}
    >
      {!result
        ? (
          <Space direction="vertical" style={{ width: "100%" }} size="middle">
            <Alert
              type="warning"
              showIcon
              icon={<AlertTriangle size={14} />}
              message="授权请求"
              description={
                <Space direction="vertical" size={4}>
                  <Typography.Text>
                    应用程序请求访问以下文件：
                  </Typography.Text>
                  <Tag icon={<FileText size={12} />}>{path}</Tag>
                </Space>
              }
            />

            <Descriptions column={1} size="small">
              <Descriptions.Item label="请求原因">
                <Input.TextArea
                  value={customReason}
                  onChange={(e) => setCustomReason(e.target.value)}
                  placeholder="说明访问此文件的用途..."
                  rows={2}
                  autoSize={{ minRows: 1, maxRows: 3 }}
                />
              </Descriptions.Item>
            </Descriptions>

            <div>
              <Typography.Text strong>授权级别</Typography.Text>
              <Radio.Group
                value={level}
                onChange={(e) => setLevel(e.target.value)}
                style={{ display: "block", marginTop: 8 }}
              >
                {(Object.keys(levelLabels) as PermissionLevel[]).map((l) => (
                  <Radio.Button key={l} value={l} style={{ width: "50%", textAlign: "center" }}>
                    {levelLabels[l].label}
                  </Radio.Button>
                ))}
              </Radio.Group>
              <Typography.Text type="secondary" style={{ display: "block", marginTop: 4 }}>
                {levelLabels[level].desc}
              </Typography.Text>
            </div>

            {level === "temp" && (
              <div>
                <Typography.Text strong>授权时长</Typography.Text>
                <Space style={{ marginTop: 8 }}>
                  <Input
                    type="number"
                    value={duration}
                    onChange={(e) => setDuration(Number(e.target.value))}
                    style={{ width: 80 }}
                    min={5}
                    max={1440}
                  />
                  <Typography.Text type="secondary">分钟</Typography.Text>
                  <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                    (最大 24 小时)
                  </Typography.Text>
                </Space>
                <div style={{ marginTop: 8 }}>
                  <Typography.Text
                    type="secondary"
                    style={{ fontSize: 12, cursor: "pointer" }}
                    onClick={() => setDuration(30)}
                  >
                    30 分钟
                  </Typography.Text>
                  <Typography.Text type="secondary" style={{ margin: "0 8px" }}>|</Typography.Text>
                  <Typography.Text
                    type="secondary"
                    style={{ fontSize: 12, cursor: "pointer" }}
                    onClick={() => setDuration(60)}
                  >
                    1 小时
                  </Typography.Text>
                  <Typography.Text type="secondary" style={{ margin: "0 8px" }}>|</Typography.Text>
                  <Typography.Text
                    type="secondary"
                    style={{ fontSize: 12, cursor: "pointer" }}
                    onClick={() => setDuration(240)}
                  >
                    4 小时
                  </Typography.Text>
                </div>
              </div>
            )}

            <Space style={{ width: "100%", justifyContent: "flex-end" }}>
              <Button onClick={onClose}>拒绝</Button>
              <Button type="primary" onClick={handleAuthorize} loading={loading}>
                授权
              </Button>
            </Space>
          </Space>
        )
        : (
          <Space direction="vertical" style={{ width: "100%" }} size="middle">
            {result.authorized
              ? (
                <>
                  <Alert
                    type="success"
                    showIcon
                    message="授权成功"
                    description={
                      <Space direction="vertical" size={4}>
                        <Typography.Text>{result.message}</Typography.Text>
                        {result.expires_at && (
                          <Tag icon={<Clock size={12} />}>
                            有效期至：{new Date(result.expires_at).toLocaleString()}
                          </Tag>
                        )}
                      </Space>
                    }
                  />
                  <Descriptions column={1} size="small" bordered>
                    <Descriptions.Item label="授权ID">{result.auth_id}</Descriptions.Item>
                    <Descriptions.Item label="文件路径">{result.path}</Descriptions.Item>
                    <Descriptions.Item label="授权级别">{result.level}</Descriptions.Item>
                  </Descriptions>
                  <Space style={{ width: "100%", justifyContent: "flex-end" }}>
                    <Button onClick={handleRevoke} danger>
                      撤销授权
                    </Button>
                    <Button type="primary" onClick={onClose}>
                      完成
                    </Button>
                  </Space>
                </>
              )
              : (
                <>
                  <Alert
                    type="error"
                    showIcon
                    message="授权失败"
                    description={result.message}
                  />
                  <Space style={{ width: "100%", justifyContent: "flex-end" }}>
                    <Button onClick={onClose}>关闭</Button>
                  </Space>
                </>
              )}
          </Space>
        )}
    </Modal>
  );
}
