import { Badge, Button, Card, Descriptions, Input, message, Modal, Space, Tag, Typography } from "antd";
import { CheckCircle, Code2, Loader2, PackageSearch, XCircle } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;

interface PluginSummary {
  id: string;
  name: string;
  version: string;
  description: string;
  kind: string;
  enabled: boolean;
  tools: string[];
  mcp_servers: string[];
  skills: string[];
}

interface PluginManifest {
  name: string;
  version: string;
  description: string;
  permissions: string[];
  default_enabled: boolean;
  hooks: Record<string, string[]>;
  tools: { name: string; description: string }[];
  mcp_servers: { name: string; command: string }[];
  skills: { name: string; path: string }[];
}

interface InstallOutcome {
  plugin_id: string;
  version: string;
  install_path: string;
}

export function PluginMarketplace() {
  const { t } = useTranslation();
  const [plugins, setPlugins] = useState<PluginSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);
  const [installInput, setInstallInput] = useState("");
  const [searchLoading, setSearchLoading] = useState(false);
  const [confirmManifest, setConfirmManifest] = useState<PluginManifest | null>(null);
  const [confirmSource, setConfirmSource] = useState("");

  useEffect(() => {
    fetchPlugins();
  }, []);

  const fetchPlugins = async () => {
    setLoading(true);
    try {
      const { invoke } = await import("@/lib/invoke");
      const data = await invoke<PluginSummary[]>("plugin_list").catch((e) => {
        if (import.meta.env.DEV) { console.warn("Failed to fetch plugins:", e); }
        return [];
      });
      setPlugins(data);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  };

  const handleSearchInstall = async () => {
    const source = installInput.trim();
    if (!source) { return; }
    setSearchLoading(true);
    try {
      const { invoke } = await import("@/lib/invoke");
      const manifest = await invoke<PluginManifest>("plugin_validate_source", {
        source,
      });
      setConfirmManifest(manifest);
      setConfirmSource(source);
    } catch (e: unknown) {
      const errMsg = e instanceof Error ? e.message : String(e);
      message.error(t("chat.plugins.marketplace.validateFailed", { error: errMsg }));
    } finally {
      setSearchLoading(false);
    }
  };

  const handleConfirmInstall = async () => {
    if (!confirmSource) { return; }
    setInstalling(confirmSource);
    setConfirmManifest(null);
    try {
      const { invoke } = await import("@/lib/invoke");
      const result = await invoke<InstallOutcome>("plugin_install", {
        source: confirmSource,
      });
      message.success(
        t("chat.plugins.marketplace.installSuccess", { id: result.plugin_id, version: result.version }),
      );
      setInstallInput("");
      setConfirmSource("");
      await fetchPlugins();
    } catch (e: unknown) {
      const errMsg = e instanceof Error ? e.message : String(e);
      message.error(t("chat.plugins.marketplace.installFailed", { error: errMsg }));
    } finally {
      setInstalling(null);
    }
  };

  const handleToggle = async (pluginId: string, enable: boolean) => {
    try {
      const { invoke } = await import("@/lib/invoke");
      await invoke(enable ? "plugin_enable" : "plugin_disable", { pluginId });
      await fetchPlugins();
    } catch {
      // ignore
    }
  };

  const handleUninstall = async (pluginId: string) => {
    setInstalling(pluginId);
    try {
      const { invoke } = await import("@/lib/invoke");
      await invoke("plugin_uninstall", { pluginId });
      await fetchPlugins();
    } catch {
      // ignore
    } finally {
      setInstalling(null);
    }
  };

  return (
    <>
      <Card size="small">
        <div className="flex items-center justify-between mb-3">
          <Space>
            <PackageSearch size={16} className="text-purple-500" />
            <Title level={5} className="mb-0">
              {t("chat.plugins.marketplace.title")}
            </Title>
            <Badge count={plugins.length} size="small" />
          </Space>
          <Button size="small" onClick={fetchPlugins} loading={loading}>
            {t("chat.plugins.marketplace.refresh")}
          </Button>
        </div>

        <div className="mb-3">
          <Input.Search
            placeholder={t("chat.plugins.marketplace.installPlaceholder")}
            enterButton={t("chat.plugins.marketplace.install")}
            loading={searchLoading}
            value={installInput}
            onChange={(e) => setInstallInput(e.target.value)}
            onSearch={handleSearchInstall}
          />
        </div>

        {loading && plugins.length === 0 && (
          <div className="flex items-center gap-2 py-4 text-sm text-zinc-500">
            <Loader2 size={14} className="animate-spin" />
            <span>{t("chat.plugins.marketplace.loading")}</span>
          </div>
        )}

        <div className="space-y-2 max-h-96 overflow-auto">
          {plugins.map((plugin) => (
            <Card key={plugin.id} size="small" className="plugin-card">
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <Code2 size={14} className="text-purple-500" />
                    <Text strong className="text-sm">
                      {plugin.name}
                    </Text>
                    <Tag color="purple" className="text-xs">
                      {plugin.version}
                    </Tag>
                    {plugin.enabled && <CheckCircle size={12} className="text-green-500" />}
                  </div>
                  <Text type="secondary" className="text-xs block mt-1">
                    {plugin.description}
                  </Text>
                  <Space size="small" className="mt-1">
                    <Tag color="geekblue" className="text-xs">
                      {plugin.kind}
                    </Tag>
                    {(plugin.mcp_servers.length > 0 || plugin.skills.length > 0) && (
                      <Text type="secondary" className="text-xs">
                        MCP:{plugin.mcp_servers.length} Skills:{plugin.skills.length}
                      </Text>
                    )}
                  </Space>
                </div>

                <div className="flex items-center gap-1">
                  <Button
                    size="small"
                    type={plugin.enabled ? "default" : "primary"}
                    onClick={() => handleToggle(plugin.id, !plugin.enabled)}
                  >
                    {plugin.enabled
                      ? t("chat.plugins.marketplace.disable")
                      : t("chat.plugins.marketplace.enable")}
                  </Button>
                  <Button
                    size="small"
                    danger
                    icon={<XCircle size={12} />}
                    loading={installing === plugin.id}
                    onClick={() => handleUninstall(plugin.id)}
                  />
                </div>
              </div>

              {plugin.tools.length > 0 && (
                <div className="flex gap-2 mt-2 flex-wrap">
                  {plugin.tools.slice(0, 5).map((tool, i) => (
                    <Tag key={i} color="cyan" className="text-xs">
                      {tool}
                    </Tag>
                  ))}
                  {plugin.tools.length > 5 && (
                    <Text type="secondary" className="text-xs">
                      +{plugin.tools.length - 5}
                    </Text>
                  )}
                </div>
              )}
            </Card>
          ))}
        </div>
      </Card>

      <Modal
        title={t("chat.plugins.marketplace.installTitle", { name: confirmManifest?.name ?? "" })}
        open={!!confirmManifest}
        onOk={handleConfirmInstall}
        onCancel={() => setConfirmManifest(null)}
        okText={t("chat.plugins.marketplace.confirmInstall")}
        cancelText={t("chat.plugins.marketplace.cancel")}
        width={560}
      >
        {confirmManifest && (
          <Descriptions column={1} size="small" bordered>
            <Descriptions.Item label={t("chat.plugins.marketplace.version")}>
              {confirmManifest.version}
            </Descriptions.Item>
            <Descriptions.Item label={t("chat.plugins.marketplace.description")}>
              {confirmManifest.description}
            </Descriptions.Item>
            <Descriptions.Item label={t("chat.plugins.marketplace.permissions")}>
              {confirmManifest.permissions.length > 0
                ? confirmManifest.permissions.join(", ")
                : t("chat.plugins.marketplace.none")}
            </Descriptions.Item>
            <Descriptions.Item label={t("chat.plugins.marketplace.mcpServers")}>
              {confirmManifest.mcp_servers.length > 0
                ? confirmManifest.mcp_servers
                  .map((s) => `${s.name} (${s.command})`)
                  .join(", ")
                : t("chat.plugins.marketplace.none")}
            </Descriptions.Item>
            <Descriptions.Item label={t("chat.plugins.marketplace.skills")}>
              {confirmManifest.skills.length > 0
                ? confirmManifest.skills.map((s) => s.name).join(", ")
                : t("chat.plugins.marketplace.none")}
            </Descriptions.Item>
            <Descriptions.Item label={t("chat.plugins.marketplace.tools")}>
              {confirmManifest.tools.length > 0
                ? confirmManifest.tools.map((tool) => tool.name).join(", ")
                : t("chat.plugins.marketplace.none")}
            </Descriptions.Item>
          </Descriptions>
        )}
      </Modal>
    </>
  );
}
