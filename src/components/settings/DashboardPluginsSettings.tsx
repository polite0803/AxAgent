import { invoke } from "@/lib/invoke";
import { Button, Card, Empty, message, Spin, Switch, Table, Tag, Typography } from "antd";
import { FolderOpen, PanelRight, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph, Title } = Typography;

interface DashboardPanel {
  id: string;
  title: string;
  component_name: string;
  position: "Main" | "Sidebar" | "Header" | "Footer";
  size: "Small" | "Medium" | "Large" | "FullWidth";
}

interface DashboardPluginInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  author?: string;
  panels: DashboardPanel[];
  enabled: boolean;
}

const POSITION_COLORS: Record<string, string> = {
  Main: "blue",
  Sidebar: "green",
  Header: "orange",
  Footer: "purple",
};

export default function DashboardPluginsSettings() {
  const { t } = useTranslation();
  const [plugins, setPlugins] = useState<DashboardPluginInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [unloadingId, setUnloadingId] = useState<string | null>(null);

  const loadPlugins = async () => {
    try {
      const result = await invoke<DashboardPluginInfo[]>("dashboard_list_plugins");
      setPlugins(result);
    } catch (error) {
      message.error(`Failed to load plugins: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadPlugins();
  }, []);

  const handleRefresh = async () => {
    setRefreshing(true);
    try {
      await invoke("dashboard_reload_plugins");
      await loadPlugins();
      message.success(t("settings.dashboardPlugins.refreshSuccess"));
    } catch (error) {
      message.error(`Refresh failed: ${error}`);
    } finally {
      setRefreshing(false);
    }
  };

  const handleToggle = async (pluginId: string, enabled: boolean) => {
    try {
      if (enabled) {
        await invoke("dashboard_enable_plugin", { pluginId });
      } else {
        await invoke("dashboard_disable_plugin", { pluginId });
      }
      setPlugins((prev) => prev.map((p) => (p.id === pluginId ? { ...p, enabled } : p)));
      message.success(
        enabled
          ? t("settings.dashboardPlugins.enabled")
          : t("settings.dashboardPlugins.disabled"),
      );
    } catch (error) {
      message.error(`Toggle failed: ${error}`);
    }
  };

  const handleUnload = async (pluginId: string) => {
    setUnloadingId(pluginId);
    try {
      await invoke("dashboard_unregister_plugin", { pluginId });
      setPlugins((prev) => prev.filter((p) => p.id !== pluginId));
      message.success(t("settings.dashboardPlugins.unloaded"));
    } catch (error) {
      message.error(`Unload failed: ${error}`);
    } finally {
      setUnloadingId(null);
    }
  };

  const columns = [
    {
      title: t("settings.dashboardPlugins.name"),
      dataIndex: "name",
      key: "name",
      width: 200,
      render: (name: string, record: DashboardPluginInfo) => (
        <div>
          <Text strong>{name}</Text>
          <br />
          <Text type="secondary" className="text-xs">
            v{record.version}
          </Text>
          {record.author && (
            <>
              <br />
              <Text type="secondary" className="text-xs">
                by {record.author}
              </Text>
            </>
          )}
        </div>
      ),
    },
    {
      title: t("settings.dashboardPlugins.description"),
      dataIndex: "description",
      key: "description",
      ellipsis: true,
    },
    {
      title: t("settings.dashboardPlugins.panels"),
      dataIndex: "panels",
      key: "panels",
      width: 200,
      render: (panels: DashboardPanel[]) => (
        <div className="flex flex-wrap gap-1">
          {panels.map((panel) => (
            <Tag
              key={panel.id}
              color={POSITION_COLORS[panel.position] || "default"}
              icon={<PanelRight size={12} />}
            >
              {panel.title}
            </Tag>
          ))}
        </div>
      ),
    },
    {
      title: t("settings.dashboardPlugins.status"),
      dataIndex: "enabled",
      key: "enabled",
      width: 100,
      render: (enabled: boolean, record: DashboardPluginInfo) => (
        <Switch
          checked={enabled}
          onChange={(checked) => handleToggle(record.id, checked)}
        />
      ),
    },
    {
      title: "",
      key: "actions",
      width: 120,
      render: (_: unknown, record: DashboardPluginInfo) => (
        <Button
          type="text"
          danger
          size="small"
          icon={<Trash2 size={14} />}
          onClick={() => handleUnload(record.id)}
          loading={unloadingId === record.id}
        >
          {t("settings.dashboardPlugins.unload")}
        </Button>
      ),
    },
  ];

  if (loading) {
    return (
      <div className="flex items-center justify-center h-48">
        <Spin size="large" />
      </div>
    );
  }

  return (
    <div className="max-w-5xl">
      <div className="flex items-center justify-between mb-6">
        <div>
          <Title level={4}>{t("settings.dashboardPlugins.title")}</Title>
          <Paragraph type="secondary">
            {t("settings.dashboardPlugins.description")}
          </Paragraph>
        </div>
        <div className="flex gap-2">
          <Button
            icon={<RefreshCw size={16} className={refreshing ? "animate-spin" : ""} />}
            onClick={handleRefresh}
            loading={refreshing}
          >
            {t("settings.dashboardPlugins.refresh")}
          </Button>
          <Button type="primary" icon={<Plus size={16} />}>
            {t("settings.dashboardPlugins.install")}
          </Button>
        </div>
      </div>

      {plugins.length > 0
        ? (
          <Table
            dataSource={plugins}
            columns={columns}
            rowKey="id"
            pagination={false}
          />
        )
        : (
          <Card>
            <Empty
              image={<FolderOpen size={48} className="text-text-quaternary" />}
              description={
                <div>
                  <Paragraph>{t("settings.dashboardPlugins.noPlugins")}</Paragraph>
                  <Button type="primary" icon={<Plus size={16} />}>
                    {t("settings.dashboardPlugins.installFirst")}
                  </Button>
                </div>
              }
            />
          </Card>
        )}

      <Card className="mt-6">
        <Title level={5}>{t("settings.dashboardPlugins.pluginDirs")}</Title>
        <Paragraph type="secondary" className="mb-4">
          {t("settings.dashboardPlugins.pluginDirsDescription")}
        </Paragraph>
        <Button icon={<FolderOpen size={16} />}>
          {t("settings.dashboardPlugins.openPluginsFolder")}
        </Button>
      </Card>
    </div>
  );
}
