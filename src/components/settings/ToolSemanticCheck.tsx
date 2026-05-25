import type { ToolUpgradeSuggestion } from "@/components/workflow/types/workflow.types";
import { invoke } from "@/lib/invoke";
import { useUIStore } from "@/stores";
import type { LocalToolGroupInfo, LocalToolInfo } from "@/types";
import { Button, Card, Empty, Input, List, message, Modal, Spin, Typography } from "antd";
import { ArrowRight, CheckCircle, Search, Zap } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;
const { Search: AntSearch } = Input;

interface ToolToCheck {
  name: string;
  description: string;
  tool_type: string;
  node_id?: string;
}

interface ToolMatch {
  tool_name: string;
  tool_type: string;
  description: string;
  similarity_score: number;
  match_reasons: string[];
}

interface NodeToolMatches {
  node_id?: string;
  tool_name: string;
  matches: ToolMatch[];
}

interface ToolSemanticCheckResponse {
  matches: NodeToolMatches[];
}

interface ToolUpgradeRequest {
  existing_tool_name: string;
  existing_tool_description: string;
  existing_tool_type: string;
  existing_input_schema?: any;
  existing_output_schema?: any;
  generated_name: string;
  generated_description: string;
  generated_input_schema?: any;
  generated_output_schema?: any;
}

interface ToolUpgradeResponse {
  suggestion: ToolUpgradeSuggestion;
}

export function ToolSemanticCheck() {
  const { t } = useTranslation();
  const deviceLayout = useUIStore((s) => s.deviceLayout);
  const isSmall = deviceLayout === "mobile" || deviceLayout === "tablet";
  const [searchLoading, setSearchLoading] = useState(false);
  const [searchTerm, setSearchTerm] = useState("");
  const [matches, setMatches] = useState<NodeToolMatches[]>([]);
  const [selectedMatch, setSelectedMatch] = useState<
    {
      source: ToolToCheck;
      match: ToolMatch;
    } | null
  >(null);
  const [upgradeModalVisible, setUpgradeModalVisible] = useState(false);
  const [upgradeLoading, setUpgradeLoading] = useState(false);
  const [upgradeSuggestion, setUpgradeSuggestion] = useState<ToolUpgradeSuggestion | null>(null);
  const [selectedTool, setSelectedTool] = useState<LocalToolInfo | null>(null);
  const [allTools, setAllTools] = useState<LocalToolInfo[]>([]);
  const [toolsLoading, setToolsLoading] = useState(false);

  useEffect(() => {
    const loadTools = async () => {
      setToolsLoading(true);
      try {
        const groups = await invoke<LocalToolGroupInfo[]>("list_local_tools");
        const tools = Array.isArray(groups) ? groups.flatMap((g) => g.tools) : [];
        setAllTools(tools);
      } catch (e) {
        message.error(String(e));
      } finally {
        setToolsLoading(false);
      }
    };
    loadTools();
  }, []);

  const filteredTools = allTools.filter(
    (tool) =>
      tool.name.toLowerCase().includes(searchTerm.toLowerCase())
      || tool.description.toLowerCase().includes(searchTerm.toLowerCase()),
  );

  const checkSemanticMatches = useCallback(
    async (tool: LocalToolInfo) => {
      setSearchLoading(true);
      try {
        const toolsToCheck: ToolToCheck[] = [
          {
            name: tool.name,
            description: tool.description,
            tool_type: "local",
          },
        ];

        const response: ToolSemanticCheckResponse = await invoke(
          "check_tool_semantic_matches",
          {
            request: { tools: toolsToCheck },
            min_similarity: 0.6,
          },
        );

        setMatches(response.matches);
        if (response.matches.length === 0) {
          message.info(t("settings.toolSemanticCheck.noMatches"));
        }
      } catch (error) {
        message.error(String(error));
      } finally {
        setSearchLoading(false);
      }
    },
    [t],
  );

  useEffect(() => {
    if (!selectedTool) {
      return;
    }
    checkSemanticMatches(selectedTool);
  }, [selectedTool, checkSemanticMatches]);

  const handleUpgradeTool = useCallback(async () => {
    if (!selectedMatch) {
      return;
    }

    setUpgradeLoading(true);
    try {
      const request: ToolUpgradeRequest = {
        existing_tool_name: selectedMatch.match.tool_name,
        existing_tool_description: selectedMatch.match.description,
        existing_tool_type: selectedMatch.match.tool_type,
        generated_name: selectedMatch.source.name,
        generated_description: selectedMatch.source.description,
      };

      const response: ToolUpgradeResponse = await invoke(
        "upgrade_tool_with_llm",
        {
          request,
        },
      );

      setUpgradeSuggestion(response.suggestion);
      message.success(t("settings.toolSemanticCheck.upgradeSuccess"));
    } catch (error) {
      message.error(String(error));
    } finally {
      setUpgradeLoading(false);
    }
  }, [selectedMatch, t]);

  const handleMatchSelect = useCallback(
    (source: ToolToCheck, match: ToolMatch) => {
      setSelectedMatch({ source, match });
      setUpgradeModalVisible(true);
    },
    [],
  );

  return (
    <div
      className="flex gap-4 flex-1"
      style={{ minHeight: 0, flexDirection: isSmall ? "column" : "row" }}
    >
      <div
        className="border rounded-lg overflow-hidden flex flex-col"
        style={{
          minHeight: 0,
          width: isSmall ? "100%" : "33.333%",
          flexShrink: 0,
          maxHeight: isSmall ? 300 : undefined,
        }}
      >
        <div className="p-3 border-b shrink-0">
          <AntSearch
            placeholder={t("settings.toolSemanticCheck.searchPlaceholder")}
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            onSearch={() => {
              if (filteredTools.length > 0) {
                setSelectedTool(filteredTools[0]);
              }
            }}
            allowClear
          />
        </div>
        <div className="flex-1 overflow-auto" style={{ minHeight: 0 }}>
          {toolsLoading
            ? (
              <div className="flex justify-center items-center py-8">
                <Spin />
              </div>
            )
            : filteredTools.length === 0
            ? (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description={t("settings.toolSemanticCheck.empty")}
                className="py-8"
              />
            )
            : (
              filteredTools.map((tool) => (
                <div
                  key={tool.name}
                  role="button"
                  tabIndex={0}
                  className={`px-3 py-2 cursor-pointer hover:bg-bg-container-hover border-b border-border/50 ${
                    selectedTool?.name === tool.name ? "bg-primary/10" : ""
                  }`}
                  onClick={() => setSelectedTool(tool)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      setSelectedTool(tool);
                    }
                  }}
                >
                  <Text strong className="text-sm">
                    {tool.name}
                  </Text>
                  <Text
                    type="secondary"
                    className="block text-xs mt-0.5 line-clamp-2"
                  >
                    {tool.description}
                  </Text>
                </div>
              ))
            )}
        </div>
      </div>

      <div className="flex-1 flex flex-col" style={{ minHeight: 0 }}>
        {selectedTool
          ? (
            <Card
              title={
                <div className="flex items-center gap-2">
                  <Search size={18} />
                  {t("settings.toolSemanticCheck.checkingTool", {
                    tool: selectedTool.name,
                  })}
                </div>
              }
              className="flex-1 flex flex-col"
              styles={{ body: { flex: 1, overflow: "auto", minHeight: 0 } }}
            >
              {searchLoading
                ? (
                  <div className="flex justify-center items-center py-8">
                    <Spin />
                  </div>
                )
                : (
                  <List
                    itemLayout="vertical"
                    size="large"
                    dataSource={matches}
                    locale={{ emptyText: t("settings.toolSemanticCheck.empty") }}
                    renderItem={(item) => (
                      <List.Item
                        key={item.tool_name}
                        actions={[
                          <Button
                            key="upgrade"
                            type="primary"
                            icon={<Zap size={16} />}
                            onClick={() =>
                              handleMatchSelect(
                                {
                                  name: item.tool_name,
                                  description: "",
                                  tool_type: "local",
                                },
                                item.matches[0],
                              )}
                          >
                            {t("settings.toolSemanticCheck.upgrade")}
                          </Button>,
                        ]}
                      >
                        <List.Item.Meta
                          title={
                            <div className="flex items-center gap-2">
                              <Text strong>{item.tool_name}</Text>
                              <Text type="secondary">
                                (
                                {t("settings.toolSemanticCheck.matches", {
                                  count: item.matches.length,
                                })}
                                )
                              </Text>
                            </div>
                          }
                          description={t(
                            "settings.toolSemanticCheck.checkingTool",
                            { tool: item.tool_name },
                          )}
                        />
                        <div className="mt-2">
                          {item.matches.map((match, _index) => (
                            <Card
                              key={match.tool_name}
                              size="small"
                              className="mb-2"
                              extra={
                                <Text type="success">
                                  {t("settings.toolSemanticCheck.similarity", {
                                    score: Math.round(match.similarity_score * 100),
                                  })}
                                  %
                                </Text>
                              }
                            >
                              <div className="flex items-start gap-3">
                                <CheckCircle
                                  size={18}
                                  className="text-success mt-1 shrink-0"
                                />
                                <div className="flex-1">
                                  <Text strong>{match.tool_name}</Text>
                                  <Text
                                    type="secondary"
                                    className="block mt-1 text-sm"
                                  >
                                    {match.description}
                                  </Text>
                                  <div className="mt-2">
                                    {match.match_reasons.map((reason, rIndex) => (
                                      <Text
                                        key={rIndex}
                                        type="secondary"
                                        className="block text-xs"
                                      >
                                        • {reason}
                                      </Text>
                                    ))}
                                  </div>
                                </div>
                              </div>
                            </Card>
                          ))}
                        </div>
                      </List.Item>
                    )}
                  />
                )}
            </Card>
          )
          : (
            <div className="flex-1 flex items-center justify-center">
              <Empty description={t("settings.toolSemanticCheck.selectTool")} />
            </div>
          )}
      </div>

      <Modal
        title={t("settings.toolSemanticCheck.upgradeModalTitle")}
        open={upgradeModalVisible}
        onCancel={() => setUpgradeModalVisible(false)}
        footer={[
          <Button key="cancel" onClick={() => setUpgradeModalVisible(false)}>
            {t("common.cancel")}
          </Button>,
          <Button
            key="upgrade"
            type="primary"
            loading={upgradeLoading}
            onClick={handleUpgradeTool}
          >
            {t("settings.toolSemanticCheck.performUpgrade")}
          </Button>,
        ]}
        width={700}
      >
        {selectedMatch && (
          <div>
            <div className="mb-4">
              <Title level={5}>
                {t("settings.toolSemanticCheck.existingTool")}
              </Title>
              <Card size="small">
                <Text strong>{selectedMatch.match.tool_name}</Text>
                <Text type="secondary" className="block mt-1">
                  {selectedMatch.match.description}
                </Text>
                <Text type="secondary" className="block mt-1 text-sm">
                  {t("settings.toolSemanticCheck.toolType")}: {selectedMatch.match.tool_type}
                </Text>
              </Card>
            </div>

            <div className="flex justify-center my-4">
              <ArrowRight size={24} className="text-zinc-400" />
            </div>

            <div className="mb-4">
              <Title level={5}>
                {t("settings.toolSemanticCheck.generatedTool")}
              </Title>
              <Card size="small">
                <Text strong>{selectedMatch.source.name}</Text>
                <Text type="secondary" className="block mt-1">
                  {selectedMatch.source.description
                    || t("settings.toolSemanticCheck.noDescription")}
                </Text>
              </Card>
            </div>

            {upgradeSuggestion && (
              <div className="mt-6">
                <Card size="small" className="bg-blue-50">
                  <Text strong>{upgradeSuggestion.name}</Text>
                  <Text type="secondary" className="block mt-1">
                    {upgradeSuggestion.description}
                  </Text>
                  <Text type="secondary" className="block mt-3 text-sm">
                    <strong>
                      {t("settings.toolSemanticCheck.reasoning")}:
                    </strong>{" "}
                    {upgradeSuggestion.reasoning}
                  </Text>
                </Card>
              </div>
            )}
          </div>
        )}
      </Modal>
    </div>
  );
}
