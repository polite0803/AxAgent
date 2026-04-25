import { useState, useCallback } from 'react';
import { Button, Card, Input, List, Modal, Typography, message, Divider } from 'antd';
import { Search, RefreshCw, CheckCircle, ArrowRight, Zap } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import type { ToolUpgradeSuggestion } from '@/components/workflow/types/workflow.types';

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

export default function ToolSemanticCheck() {
  const { t } = useTranslation();
  const [searchLoading, setSearchLoading] = useState(false);
  const [searchTerm, setSearchTerm] = useState('');
  const [matches, setMatches] = useState<NodeToolMatches[]>([]);
  const [selectedMatch, setSelectedMatch] = useState<{ source: ToolToCheck; match: ToolMatch } | null>(null);
  const [upgradeModalVisible, setUpgradeModalVisible] = useState(false);
  const [upgradeLoading, setUpgradeLoading] = useState(false);
  const [upgradeSuggestion, setUpgradeSuggestion] = useState<ToolUpgradeSuggestion | null>(null);

  const checkSemanticMatches = useCallback(async () => {
    if (!searchTerm.trim()) {
      message.error(t('settings.toolSemanticCheck.emptySearch'));
      return;
    }

    setSearchLoading(true);
    try {
      const toolsToCheck: ToolToCheck[] = [
        {
          name: searchTerm,
          description: searchTerm,
          tool_type: 'local',
        },
      ];

      const response: ToolSemanticCheckResponse = await invoke('check_tool_semantic_matches', {
        request: { tools: toolsToCheck },
        min_similarity: 0.6,
      });

      setMatches(response.matches);
      if (response.matches.length === 0) {
        message.info(t('settings.toolSemanticCheck.noMatches'));
      }
    } catch (error) {
      message.error(String(error));
    } finally {
      setSearchLoading(false);
    }
  }, [searchTerm, t]);

  const handleUpgradeTool = useCallback(async () => {
    if (!selectedMatch) return;

    setUpgradeLoading(true);
    try {
      const request: ToolUpgradeRequest = {
        existing_tool_name: selectedMatch.match.tool_name,
        existing_tool_description: selectedMatch.match.description,
        existing_tool_type: selectedMatch.match.tool_type,
        generated_name: selectedMatch.source.name,
        generated_description: selectedMatch.source.description,
      };

      const response: ToolUpgradeResponse = await invoke('upgrade_tool_with_llm', {
        request,
      });

      setUpgradeSuggestion(response.suggestion);
      message.success(t('settings.toolSemanticCheck.upgradeSuccess'));
    } catch (error) {
      message.error(String(error));
    } finally {
      setUpgradeLoading(false);
    }
  }, [selectedMatch, t]);

  const handleMatchSelect = useCallback((source: ToolToCheck, match: ToolMatch) => {
    setSelectedMatch({ source, match });
    setUpgradeModalVisible(true);
  }, []);

  return (
    <div className="max-w-3xl">
      <Card
        title={
          <div className="flex items-center gap-2">
            <Search size={18} />
            {t('settings.toolSemanticCheck.title')}
          </div>
        }
        className="mb-6"
      >
        <div className="flex gap-2">
          <AntSearch
            placeholder={t('settings.toolSemanticCheck.searchPlaceholder')}
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            onSearch={checkSemanticMatches}
            enterButton={
              <Button type="primary" loading={searchLoading}>
                {t('common.search')}
              </Button>
            }
            style={{ flex: 1 }}
          />
          <Button
            icon={<RefreshCw size={16} />}
            onClick={() => {
              setSearchTerm('');
              setMatches([]);
            }}
          >
            {t('common.reset')}
          </Button>
        </div>
      </Card>

      
      <List
        className="max-w-3xl"
        itemLayout="vertical"
        size="large"
        dataSource={matches}
        locale={{ emptyText: t('settings.toolSemanticCheck.empty') }}
        renderItem={(item) => (
            <List.Item
              key={item.tool_name}
              actions={[
                <Button
                  key="upgrade"
                  type="primary"
                  icon={<Zap size={16} />}
                  onClick={() => handleMatchSelect({ name: item.tool_name, description: '', tool_type: 'local' }, item.matches[0])}
                >
                  {t('settings.toolSemanticCheck.upgrade')}
                </Button>,
              ]}
            >
              <List.Item.Meta
                title={
                  <div className="flex items-center gap-2">
                    <Text strong>{item.tool_name}</Text>
                    <Text type="secondary">({t('settings.toolSemanticCheck.matches', { count: item.matches.length })})</Text>
                  </div>
                }
                description={t('settings.toolSemanticCheck.checkingTool', { tool: item.tool_name })}
              />
              <div className="mt-2">
                {item.matches.map((match, index) => (
                  <Card
                    key={index}
                    size="small"
                    className="mb-2"
                    extra={
                      <Text type="success">
                        {t('settings.toolSemanticCheck.similarity', { score: Math.round(match.similarity_score * 100) })}%
                      </Text>
                    }
                  >
                    <div className="flex items-start gap-3">
                      <CheckCircle size={18} className="text-success mt-1 shrink-0" />
                      <div className="flex-1">
                        <Text strong>{match.tool_name}</Text>
                        <Text type="secondary" className="block mt-1 text-sm">
                          {match.description}
                        </Text>
                        <div className="mt-2">
                          {match.match_reasons.map((reason, rIndex) => (
                            <Text key={rIndex} type="secondary" className="block text-xs">
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

      <Modal
        title={t('settings.toolSemanticCheck.upgradeModalTitle')}
        open={upgradeModalVisible}
        onCancel={() => setUpgradeModalVisible(false)}
        footer={[
          <Button key="cancel" onClick={() => setUpgradeModalVisible(false)}>
            {t('common.cancel')}
          </Button>,
          <Button
            key="upgrade"
            type="primary"
            loading={upgradeLoading}
            onClick={handleUpgradeTool}
          >
            {t('settings.toolSemanticCheck.performUpgrade')}
          </Button>,
        ]}
        width={700}
      >
        {selectedMatch && (
          <div>
            <div className="mb-4">
              <Title level={5}>{t('settings.toolSemanticCheck.existingTool')}</Title>
              <Card size="small">
                <Text strong>{selectedMatch.match.tool_name}</Text>
                <Text type="secondary" className="block mt-1">
                  {selectedMatch.match.description}
                </Text>
                <Text type="secondary" className="block mt-1 text-sm">
                  {t('settings.toolSemanticCheck.toolType')}: {selectedMatch.match.tool_type}
                </Text>
              </Card>
            </div>

            <div className="flex justify-center my-4">
              <ArrowRight size={24} className="text-gray-400" />
            </div>

            <div className="mb-4">
              <Title level={5}>{t('settings.toolSemanticCheck.generatedTool')}</Title>
              <Card size="small">
                <Text strong>{selectedMatch.source.name}</Text>
                <Text type="secondary" className="block mt-1">
                  {selectedMatch.source.description || t('settings.toolSemanticCheck.noDescription')}
                </Text>
              </Card>
            </div>

            {upgradeSuggestion && (
              <div className="mt-6">
                <Divider>{t('settings.toolSemanticCheck.upgradeSuggestion')}</Divider>
                <Card size="small" className="bg-blue-50">
                  <Text strong>{upgradeSuggestion.name}</Text>
                  <Text type="secondary" className="block mt-1">
                    {upgradeSuggestion.description}
                  </Text>
                  <Text type="secondary" className="block mt-3 text-sm">
                    <strong>{t('settings.toolSemanticCheck.reasoning')}:</strong> {upgradeSuggestion.reasoning}
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