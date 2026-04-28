import { Badge, Button, Card, Space, Tag, Typography } from "antd";
import {
  CheckCircle,
  Code2,
  Download,
  Loader2,
  PackageSearch,
  XCircle,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;

interface PluginInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  kind: string;
  marketplace: string;
  tools: string[];
  hooks: string[];
  is_installed: boolean;
  is_enabled: boolean;
  installed_version: string | null;
}

function PluginMarketplace() {
  const { t } = useTranslation();
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);

  useEffect(() => {
    fetchPlugins();
  }, []);

  const fetchPlugins = async () => {
    setLoading(true);
    try {
      const { invoke } = await import("@/lib/invoke");
      const data = await invoke<PluginInfo[]>(
        "plugin_list_available"
      ).catch(() => []);
      setPlugins(data);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  };

  const handleInstall = async (pluginId: string) => {
    setInstalling(pluginId);
    try {
      const { invoke } = await import("@/lib/invoke");
      await invoke("plugin_install", { pluginId });
      await fetchPlugins();
    } catch {
      // ignore
    } finally {
      setInstalling(null);
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

  const handleToggle = async (pluginId: string, enable: boolean) => {
    try {
      const { invoke } = await import("@/lib/invoke");
      await invoke(enable ? "plugin_enable" : "plugin_disable", { pluginId });
      await fetchPlugins();
    } catch {
      // ignore
    }
  };

  return (
    <Card size="small" className="plugin-marketplace">
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

      {loading && plugins.length === 0 && (
        <div className="flex items-center gap-2 py-4 text-sm text-gray-500">
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
                  <Tag color="purple" className="text-xs">{plugin.version}</Tag>
                  {plugin.is_installed && (
                    <CheckCircle size={12} className="text-green-500" />
                  )}
                </div>
                <Text type="secondary" className="text-xs block mt-1">
                  {plugin.description}
                </Text>
                <Space size="small" className="mt-1">
                  <Text type="secondary" className="text-xs">
                    {plugin.author}
                  </Text>
                  <Tag color="geekblue" className="text-xs">
                    {plugin.kind}
                  </Tag>
                </Space>
              </div>

              <div className="flex items-center gap-1">
                {!plugin.is_installed ? (
                  <Button
                    size="small"
                    type="primary"
                    icon={<Download size={12} />}
                    loading={installing === plugin.id}
                    onClick={() => handleInstall(plugin.id)}
                  >
                    {t("chat.plugins.marketplace.install")}
                  </Button>
                ) : (
                  <Space size="small">
                    <Button
                      size="small"
                      type={plugin.is_enabled ? "default" : "primary"}
                      onClick={() => handleToggle(plugin.id, !plugin.is_enabled)}
                    >
                      {plugin.is_enabled
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
                  </Space>
                )}
              </div>
            </div>

            {(plugin.tools.length > 0 || plugin.hooks.length > 0) && (
              <div className="flex gap-2 mt-2">
                {plugin.tools.slice(0, 4).map((tool, i) => (
                  <Tag key={i} color="cyan" className="text-xs">
                    {tool}
                  </Tag>
                ))}
                {plugin.tools.length > 4 && (
                  <Text type="secondary" className="text-xs">
                    +{plugin.tools.length - 4}
                  </Text>
                )}
              </div>
            )}
          </Card>
        ))}
      </div>
    </Card>
  );
}

export default PluginMarketplace;
