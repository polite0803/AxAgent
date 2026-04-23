import { useEffect } from 'react';
import { Switch, Tag, Spin, Typography } from 'antd';
import { Wrench, Globe, FileSearch, FileEdit, Terminal, Search, BookOpen, HardDrive, Brain, MessageSquare } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useLocalToolStore } from '@/stores';
import type { LocalToolGroupInfo } from '@/types/localTool';

const { Text, Paragraph } = Typography;

/** Map group_id to a display icon */
const GROUP_ICONS: Record<string, React.ReactNode> = {
  'builtin-fetch': <Globe size={18} />,
  'builtin-search-file': <FileSearch size={18} />,
  'builtin-filesystem': <FileEdit size={18} />,
  'builtin-system': <Terminal size={18} />,
  'builtin-search': <Search size={18} />,
  'builtin-knowledge': <BookOpen size={18} />,
  'builtin-storage': <HardDrive size={18} />,
  'builtin-skills': <Wrench size={18} />,
  'builtin-session': <MessageSquare size={18} />,
  'builtin-memory': <Brain size={18} />,
};

/** i18n key for group display name */
const GROUP_NAME_KEYS: Record<string, string> = {
  'builtin-fetch': 'settings.localTools.groupFetch',
  'builtin-search-file': 'settings.localTools.groupSearchFile',
  'builtin-filesystem': 'settings.localTools.groupFilesystem',
  'builtin-system': 'settings.localTools.groupSystem',
  'builtin-search': 'settings.localTools.groupSearch',
  'builtin-knowledge': 'settings.localTools.groupKnowledge',
  'builtin-storage': 'settings.localTools.groupStorage',
  'builtin-skills': 'settings.localTools.groupSkills',
  'builtin-session': 'settings.localTools.groupSession',
  'builtin-memory': 'settings.localTools.groupMemory',
};

function ToolGroupCard({ group, onToggle }: { group: LocalToolGroupInfo; onToggle: (groupId: string) => void }) {
  const { t } = useTranslation();
  const icon = GROUP_ICONS[group.groupId] ?? <Wrench size={18} />;
  const nameKey = GROUP_NAME_KEYS[group.groupId];
  const displayName = nameKey ? t(nameKey) : group.groupName;

  return (
    <div className="flex items-center justify-between py-3 px-4 border-b border-border last:border-b-0">
      <div className="flex items-center gap-3 min-w-0 flex-1">
        <span className="text-text-secondary shrink-0">{icon}</span>
        <div className="min-w-0 flex-1">
          <Text strong className="block">{displayName}</Text>
          <div className="flex flex-wrap gap-1 mt-1">
            {group.tools.map((tool) => (
              <Tag key={tool.toolName} className="text-xs">
                {tool.toolName}
              </Tag>
            ))}
          </div>
        </div>
      </div>
      <Switch
        checked={group.enabled}
        onChange={() => onToggle(group.groupId)}
        className="shrink-0 ml-3"
      />
    </div>
  );
}

function LocalToolSettings() {
  const { t } = useTranslation();
  const { groups, loading, loadGroups, toggleGroup } = useLocalToolStore();

  useEffect(() => {
    loadGroups();
  }, [loadGroups]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-48">
        <Spin size="large" />
      </div>
    );
  }

  return (
    <div className="p-6 max-w-2xl">
      <Typography.Title level={4}>
        {t('settings.localTools.title')}
      </Typography.Title>
      <Paragraph type="secondary" className="mb-4">
        {t('settings.localTools.description')}
      </Paragraph>

      <div className="border border-border rounded-lg overflow-hidden">
        {groups.map((group) => (
          <ToolGroupCard
            key={group.groupId}
            group={group}
            onToggle={toggleGroup}
          />
        ))}
      </div>
    </div>
  );
}

export default LocalToolSettings;
