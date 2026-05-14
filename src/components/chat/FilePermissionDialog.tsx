import { invoke } from "@/lib/invoke";
import { Alert, Button, Descriptions, Input, Modal, Radio, Space, Tag, Typography } from "antd";
import { AlertTriangle, Clock, FileText, Shield } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

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
  const { t } = useTranslation();
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
    read: { label: t("filePermission.levelRead"), desc: t("filePermission.levelReadDesc") },
    write: { label: t("filePermission.levelWrite"), desc: t("filePermission.levelWriteDesc") },
    readwrite: { label: t("filePermission.levelReadWrite"), desc: t("filePermission.levelReadWriteDesc") },
    temp: { label: t("filePermission.levelTemp"), desc: t("filePermission.levelTempDesc") },
  };

  return (
    <Modal
      title={
        <Space>
          <Shield size={18} />
          <span>{t("filePermission.title")}</span>
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
              message={t("filePermission.authRequest")}
              description={
                <Space direction="vertical" size={4}>
                  <Typography.Text>
                    {t("filePermission.accessRequestDesc")}
                  </Typography.Text>
                  <Tag icon={<FileText size={12} />}>{path}</Tag>
                </Space>
              }
            />

            <Descriptions column={1} size="small">
              <Descriptions.Item label={t("filePermission.requestReason")}>
                <Input.TextArea
                  id="file-permission-dialog-input-textarea-21"
                  value={customReason}
                  onChange={(e) => setCustomReason(e.target.value)}
                  placeholder={t("filePermission.purposePlaceholder")}
                  rows={2}
                  autoSize={{ minRows: 1, maxRows: 3 }}
                />
              </Descriptions.Item>
            </Descriptions>

            <div>
              <Typography.Text strong>{t("filePermission.authLevel")}</Typography.Text>
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
                <Typography.Text strong>{t("filePermission.authDuration")}</Typography.Text>
                <Space style={{ marginTop: 8 }}>
                  <Input
                    id="file-permission-dialog-input-22"
                    type="number"
                    value={duration}
                    onChange={(e) => setDuration(Number(e.target.value))}
                    style={{ width: 80 }}
                    min={5}
                    max={1440}
                  />
                  <Typography.Text type="secondary">{t("filePermission.minutes")}</Typography.Text>
                  <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                    {t("filePermission.maxDuration")}
                  </Typography.Text>
                </Space>
                <div style={{ marginTop: 8 }}>
                  <Typography.Text
                    type="secondary"
                    style={{ fontSize: 12, cursor: "pointer" }}
                    onClick={() => setDuration(30)}
                  >
                    {t("filePermission.duration30min")}
                  </Typography.Text>
                  <Typography.Text type="secondary" style={{ margin: "0 8px" }}>|</Typography.Text>
                  <Typography.Text
                    type="secondary"
                    style={{ fontSize: 12, cursor: "pointer" }}
                    onClick={() => setDuration(60)}
                  >
                    {t("filePermission.duration1hour")}
                  </Typography.Text>
                  <Typography.Text type="secondary" style={{ margin: "0 8px" }}>|</Typography.Text>
                  <Typography.Text
                    type="secondary"
                    style={{ fontSize: 12, cursor: "pointer" }}
                    onClick={() => setDuration(240)}
                  >
                    {t("filePermission.duration4hours")}
                  </Typography.Text>
                </div>
              </div>
            )}

            <Space style={{ width: "100%", justifyContent: "flex-end" }}>
              <Button onClick={onClose}>{t("filePermission.deny")}</Button>
              <Button type="primary" onClick={handleAuthorize} loading={loading}>
                {t("filePermission.authorize")}
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
                    message={t("filePermission.authSuccess")}
                    description={
                      <Space direction="vertical" size={4}>
                        <Typography.Text>{result.message}</Typography.Text>
                        {result.expires_at && (
                          <Tag icon={<Clock size={12} />}>
                            {t("filePermission.validUntil")}
                            {new Date(result.expires_at).toLocaleString()}
                          </Tag>
                        )}
                      </Space>
                    }
                  />
                  <Descriptions column={1} size="small" bordered>
                    <Descriptions.Item label={t("filePermission.authId")}>{result.auth_id}</Descriptions.Item>
                    <Descriptions.Item label={t("filePermission.filePath")}>{result.path}</Descriptions.Item>
                    <Descriptions.Item label={t("filePermission.authLevel")}>{result.level}</Descriptions.Item>
                  </Descriptions>
                  <Space style={{ width: "100%", justifyContent: "flex-end" }}>
                    <Button onClick={handleRevoke} danger>
                      {t("filePermission.revoke")}
                    </Button>
                    <Button type="primary" onClick={onClose}>
                      {t("filePermission.done")}
                    </Button>
                  </Space>
                </>
              )
              : (
                <>
                  <Alert
                    type="error"
                    showIcon
                    message={t("filePermission.authFailed")}
                    description={result.message}
                  />
                  <Space style={{ width: "100%", justifyContent: "flex-end" }}>
                    <Button onClick={onClose}>{t("filePermission.close")}</Button>
                  </Space>
                </>
              )}
          </Space>
        )}
    </Modal>
  );
}
