// SPDX-License-Identifier: AGPL-3.0-only

import { message } from "@/lib/toast";
import { useCapabilityStore, usePluginStore } from "@/stores";
import type { PluginManifestDto } from "@/types";
import { Badge, Button, Card, Descriptions, Input, Modal, Space, Tag, Typography } from "antd";
import { Boxes, CheckCircle, Code2, Loader2, PackageSearch, XCircle } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;

export function PluginMarketplace() {
  const { t } = useTranslation();

  // 从 usePluginStore 获取状态和方法
  const plugins = usePluginStore((s) => s.plugins);
  const loading = usePluginStore((s) => s.loading);
  const installing = usePluginStore((s) => s.installing);
  const validating = usePluginStore((s) => s.validating);
  const loadPlugins = usePluginStore((s) => s.loadPlugins);
  const validateSource = usePluginStore((s) => s.validateSource);
  const installPlugin = usePluginStore((s) => s.installPlugin);
  const enablePlugin = usePluginStore((s) => s.enablePlugin);
  const disablePlugin = usePluginStore((s) => s.disablePlugin);
  const uninstallPlugin = usePluginStore((s) => s.uninstallPlugin);

  // 能力注册表（capability_registry_dump，缺陷 #6：前端消费后端检视命令）
  const registry = useCapabilityStore((s) => s.registry);
  const registryLoading = useCapabilityStore((s) => s.isLoading);
  const loadRegistry = useCapabilityStore((s) => s.loadRegistry);

  // 组件本地 UI 状态（不适合放入全局 Store 的交互状态）
  const [installInput, setInstallInput] = useState("");
  const [confirmManifest, setConfirmManifest] = useState<PluginManifestDto | null>(null);
  const [confirmSource, setConfirmSource] = useState("");
  const [registryQuery, setRegistryQuery] = useState("");

  // 初始化加载插件列表 + 能力注册表
  useEffect(() => {
    loadPlugins();
    loadRegistry();
  }, [loadPlugins, loadRegistry]);

  const filteredRegistry = registry.filter(
    (r) =>
      r.id.toLowerCase().includes(registryQuery.toLowerCase())
      || (r.pluginId ?? "").toLowerCase().includes(registryQuery.toLowerCase()),
  );

  const handleRefresh = useCallback(() => {
    loadPlugins();
  }, [loadPlugins]);

  const handleSearchInstall = async () => {
    const source = installInput.trim();
    if (!source) {
      return;
    }

    const manifest = await validateSource(source);
    if (manifest) {
      setConfirmManifest(manifest);
      setConfirmSource(source);
    } else {
      // validateSource 在 store 中已经处理了错误日志
      // 这里可以根据需要补充提示
    }
  };

  const handleConfirmInstall = async () => {
    if (!confirmSource) {
      return;
    }

    const result = await installPlugin(confirmSource);
    if (result) {
      message.success(
        t("chat.plugins.marketplace.installSuccess", {
          id: result.pluginId,
          version: result.version,
        }),
      );
      setInstallInput("");
      setConfirmSource("");
      setConfirmManifest(null);
    } else {
      // installPlugin 在 store 中已经处理了错误日志
    }
  };

  const handleToggle = async (pluginId: string, enable: boolean) => {
    if (enable) {
      await enablePlugin(pluginId);
    } else {
      await disablePlugin(pluginId);
    }
  };

  const handleUninstall = async (pluginId: string) => {
    await uninstallPlugin(pluginId);
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
          <Button size="small" onClick={handleRefresh} loading={loading}>
            {t("chat.plugins.marketplace.refresh")}
          </Button>
        </div>

        <div className="mb-3">
          <Space.Compact>
            <Input
              placeholder={t("chat.plugins.marketplace.installPlaceholder")}
              value={installInput}
              onChange={(e) => setInstallInput(e.target.value)}
              onPressEnter={handleSearchInstall}
            />
            <Button
              type="primary"
              loading={validating}
              onClick={handleSearchInstall}
            >
              {t("chat.plugins.marketplace.install")}
            </Button>
          </Space.Compact>
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
                    {(plugin.mcpServers.length > 0
                      || plugin.skills.length > 0) && (
                      <Text type="secondary" className="text-xs">
                        MCP:{plugin.mcpServers.length} Skills:
                        {plugin.skills.length}
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
                  {plugin.tools.slice(0, 5).map((tool, _i) => (
                    <Tag key={tool} color="cyan" className="text-xs">
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

      <Card size="small" className="mt-3">
        <div className="flex items-center justify-between mb-3 gap-2">
          <Space>
            <Boxes size={16} className="text-teal-500" />
            <Title level={5} className="mb-0">
              {t("chat.plugins.marketplace.registryTitle")}
            </Title>
            <Badge count={registry.length} size="small" />
          </Space>
          <Input.Search
            size="small"
            placeholder={t("chat.plugins.marketplace.registrySearchPlaceholder")}
            value={registryQuery}
            onChange={(e) => setRegistryQuery(e.target.value)}
            allowClear
            className="w-56"
          />
        </div>

        {registryLoading && registry.length === 0 && (
          <div className="flex items-center gap-2 py-4 text-sm text-zinc-500">
            <Loader2 size={14} className="animate-spin" />
            <span>{t("chat.plugins.marketplace.registryLoading")}</span>
          </div>
        )}

        {!registryLoading && registry.length === 0 && (
          <Text type="secondary" className="text-xs">
            {t("chat.plugins.marketplace.registryEmpty")}
          </Text>
        )}

        <div className="space-y-2 max-h-96 overflow-auto">
          {filteredRegistry.map((r) => (
            <div
              key={r.id}
              className="flex items-start justify-between gap-2 border border-zinc-200 rounded-md px-2 py-1.5"
            >
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <Text strong className="text-xs" ellipsis>
                    {r.id}
                  </Text>
                  <Tag
                    color={r.origin === "builtin" ? "green" : "purple"}
                    className="text-xs shrink-0"
                  >
                    {r.origin === "builtin"
                      ? t("chat.plugins.marketplace.registryBuiltin")
                      : t("chat.plugins.marketplace.registryExternal")}
                  </Tag>
                </div>
                <Text type="secondary" className="text-xs block truncate">
                  {r.contract}
                </Text>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                {r.pluginId && (
                  <Tag color="geekblue" className="text-xs">
                    {t("chat.plugins.marketplace.registryPlugin")} · {r.pluginId}
                  </Tag>
                )}
                <Tag className="text-xs">
                  {t("chat.plugins.marketplace.registryVersion")}: {r.version}
                </Tag>
              </div>
            </div>
          ))}
        </div>
      </Card>

      <Modal
        title={t("chat.plugins.marketplace.installTitle", {
          name: confirmManifest?.name ?? "",
        })}
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
            <Descriptions.Item
              label={t("chat.plugins.marketplace.description")}
            >
              {confirmManifest.description}
            </Descriptions.Item>
            <Descriptions.Item
              label={t("chat.plugins.marketplace.permissions")}
            >
              {confirmManifest.permissions.length > 0
                ? confirmManifest.permissions.join(", ")
                : t("chat.plugins.marketplace.none")}
            </Descriptions.Item>
            <Descriptions.Item label={t("chat.plugins.marketplace.mcpServers")}>
              {confirmManifest.mcpServers.length > 0
                ? confirmManifest.mcpServers
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
