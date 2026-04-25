import React, { useEffect, useState } from 'react';
import { invoke } from '@/lib/invoke';
import { useStreamStore, useConversationStore } from '@/stores';
import { useTranslation } from 'react-i18next';
import { Bot, MessageSquare, Database, ChevronDown, ChevronRight, CheckCircle, XCircle, Clock, Loader2, SkipForward, Layers } from 'lucide-react';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface SubAgentData {
  id: string;
  parent_id: string | null;
  name: string;
  description: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
  task: string | null;
  progress: number;
  result: string | null;
  error: string | null;
  children: string[];
  metadata: {
    agent_type: string;
    capabilities: string[];
    model: string | null;
    tools: string[];
  };
}

interface AgentMessageData {
  id: string;
  from_agent: string;
  to_agent: string;
  kind: 'task_assign' | 'progress_report' | 'task_result' | 'task_error' | 'task_cancel' | 'data';
  payload: string;
  timestamp: string;
}

interface MemoryEntryData {
  key: string;
  value: string;
  namespace: string;
  created_at: number;
  updated_at: number;
  ttl_secs: number | null;
  owner_agent: string | null;
}

// ---------------------------------------------------------------------------
// Status helpers
// ---------------------------------------------------------------------------

const AGENT_STATUS_CONFIG: Record<string, { icon: React.ReactNode; color: string }> = {
  pending:   { icon: <Clock size={12} />,           color: '#8c8c8c' },
  running:   { icon: <Loader2 size={12} className="animate-spin" />, color: '#1890ff' },
  completed: { icon: <CheckCircle size={12} />,     color: '#52c41a' },
  failed:    { icon: <XCircle size={12} />,         color: '#ff4d4f' },
  cancelled: { icon: <SkipForward size={12} />,     color: '#8c8c8c' },
};

const MESSAGE_KIND_COLORS: Record<string, string> = {
  task_assign: '#1890ff',
  progress_report: '#52c41a',
  task_result: '#722ed1',
  task_error: '#ff4d4f',
  task_cancel: '#faad14',
  data: '#8c8c8c',
};

// ---------------------------------------------------------------------------
// Agent tree node
// ---------------------------------------------------------------------------

function AgentTreeNode({ agent, allAgents, depth, selectedId, onSelect }: {
  agent: SubAgentData;
  allAgents: SubAgentData[];
  depth: number;
  selectedId: string | null;
  onSelect: (id: string) => void;
}) {
  const [expanded, setExpanded] = useState(true);
  const cfg = AGENT_STATUS_CONFIG[agent.status] ?? AGENT_STATUS_CONFIG.pending;
  const children = allAgents.filter(a => a.parent_id === agent.id);
  const hasChildren = children.length > 0;

  return (
    <div>
      <div
        className="flex items-center gap-1.5 py-1 cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800/50 rounded"
        style={{ paddingLeft: depth * 16 + 8 }}
        onClick={() => onSelect(agent.id)}
      >
        {hasChildren ? (
          <span
            onClick={(e) => { e.stopPropagation(); setExpanded(!expanded); }}
            className="flex items-center text-gray-400"
          >
            {expanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
          </span>
        ) : (
          <span className="w-3" />
        )}
        <span style={{ color: cfg.color, display: 'flex', alignItems: 'center' }}>
          {cfg.icon}
        </span>
        <span
          className={`text-xs font-medium truncate ${selectedId === agent.id ? 'text-blue-600 dark:text-blue-400' : ''}`}
        >
          {agent.name}
        </span>
        {agent.progress > 0 && agent.status === 'running' && (
          <div className="flex-1 max-w-16 h-1 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden ml-1">
            <div
              className="h-full bg-blue-400 rounded-full"
              style={{ width: `${agent.progress * 100}%` }}
            />
          </div>
        )}
      </div>
      {expanded && children.map(child => (
        <AgentTreeNode
          key={child.id}
          agent={child}
          allAgents={allAgents}
          depth={depth + 1}
          selectedId={selectedId}
          onSelect={onSelect}
        />
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

const MultiAgentStatusPanel: React.FC = () => {
  const { t } = useTranslation();
  const activeConversationId = useConversationStore((s) => s.activeConversationId);
  const activeStreams = useStreamStore((s) => s.activeStreams);
  const streaming = activeConversationId ? (activeConversationId in activeStreams) : false;

  const [agents, setAgents] = useState<SubAgentData[]>([]);
  const [messages, setMessages] = useState<AgentMessageData[]>([]);
  const [memoryEntries, setMemoryEntries] = useState<MemoryEntryData[]>([]);
  const [selectedAgentId, setSelectedAgentId] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<'agents' | 'messages' | 'memory'>('agents');
  const [panelOpen, setPanelOpen] = useState(false);

  // Poll data when streaming
  useEffect(() => {
    if (!streaming) {
      // Keep panel visible if there are agents, but stop polling
      return;
    }

    const fetchData = async () => {
      try {
        const [agentList, memList] = await Promise.all([
          invoke<SubAgentData[]>('sub_agent_list'),
          invoke<MemoryEntryData[]>('shared_memory_list', { namespace: 'agents' }).catch(() => []),
        ]);
        setAgents(agentList);
        setMemoryEntries(memList);

        // Fetch messages for the selected agent (or first root agent)
        const targetId = selectedAgentId ?? (agentList.length > 0 ? agentList[0].id : null);
        if (targetId) {
          const msgs = await invoke<AgentMessageData[]>('sub_agent_get_messages', { agentId: targetId }).catch(() => []);
          setMessages(msgs);
        }
      } catch {
        // ignore
      }
    };

    fetchData();
    const interval = setInterval(fetchData, 3000);
    return () => clearInterval(interval);
  }, [streaming, selectedAgentId]);

  // Don't render if no agents
  if (agents.length === 0) return null;

  const rootAgents = agents.filter(a => a.parent_id === null);
  const runningCount = agents.filter(a => a.status === 'running').length;
  const completedCount = agents.filter(a => a.status === 'completed').length;

  const tabs = [
    { key: 'agents' as const, label: `${t('chat.multiAgent.tabs.agents')} (${agents.length})`, icon: <Bot size={12} /> },
    { key: 'messages' as const, label: `${t('chat.multiAgent.tabs.messages')}`, icon: <MessageSquare size={12} /> },
    { key: 'memory' as const, label: `${t('chat.multiAgent.tabs.memory')}`, icon: <Database size={12} /> },
  ];

  return (
    <div className="mx-3 my-1.5">
      {/* Toggle button (collapsed) */}
      {!panelOpen && (
        <button
          onClick={() => setPanelOpen(true)}
          className="flex items-center gap-2 px-3 py-1.5 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg text-xs text-green-700 dark:text-green-300 hover:bg-green-100 dark:hover:bg-green-800/30 transition-colors w-full"
        >
          <Layers size={12} />
          <span className="font-medium">{t('chat.multiAgent.title')}</span>
          <span>{runningCount > 0 ? t('chat.multiAgent.status.running') : t('chat.multiAgent.status.completed')}</span>
          <ChevronDown size={12} className="ml-auto" />
        </button>
      )}

      {/* Expanded panel */}
      {panelOpen && (
        <div className="border border-green-200 dark:border-green-800 rounded-lg bg-green-50/50 dark:bg-green-900/10 overflow-hidden">
          {/* Header */}
          <div className="flex items-center gap-2 px-3 py-1.5 border-b border-green-200 dark:border-green-800">
            <Layers size={14} className="text-green-600 dark:text-green-400" />
            <span className="text-xs font-medium text-green-700 dark:text-green-300">
              {t('chat.multiAgent.title')}
            </span>
            <span className="text-xs text-gray-500 dark:text-gray-400">
              {t('chat.multiAgent.count', { running: runningCount, completed: completedCount })}
            </span>
            <button
              onClick={() => setPanelOpen(false)}
              className="ml-auto text-xs text-gray-400 hover:text-gray-600"
            >
              <ChevronRight size={14} />
            </button>
          </div>

          {/* Tab bar */}
          <div className="flex border-b border-gray-200 dark:border-gray-800">
            {tabs.map(tab => (
              <button
                key={tab.key}
                onClick={() => setActiveTab(tab.key)}
                className={`flex items-center gap-1 px-3 py-1 text-xs transition-colors ${
                  activeTab === tab.key
                    ? 'text-green-700 dark:text-green-300 border-b-2 border-green-500'
                    : 'text-gray-500 hover:text-gray-700 dark:hover:text-gray-300'
                }`}
              >
                {tab.icon}
                {tab.label}
              </button>
            ))}
          </div>

          {/* Tab content */}
          <div className="max-h-56 overflow-auto">
            {activeTab === 'agents' && (
              <div className="py-1">
                {rootAgents.length > 0 ? (
                  rootAgents.map(agent => (
                    <AgentTreeNode
                      key={agent.id}
                      agent={agent}
                      allAgents={agents}
                      depth={0}
                      selectedId={selectedAgentId}
                      onSelect={setSelectedAgentId}
                    />
                  ))
                ) : (
                  <div className="px-3 py-4 text-xs text-gray-400 text-center">
                    {t('chat.multiAgent.noAgents')}
                  </div>
                )}
              </div>
            )}

            {activeTab === 'messages' && (
              <div className="py-1">
                {messages.length > 0 ? (
                  messages.map(msg => (
                    <div key={msg.id} className="flex items-center gap-2 px-3 py-1 text-xs border-b border-gray-100 dark:border-gray-800 last:border-b-0">
                      <span
                        className="w-2 h-2 rounded-full shrink-0"
                        style={{ backgroundColor: MESSAGE_KIND_COLORS[msg.kind] ?? '#8c8c8c' }}
                      />
                      <span className="font-mono text-gray-500 shrink-0">{msg.kind}</span>
                      <span className="text-gray-400 shrink-0">{msg.from_agent.slice(0, 8)}</span>
                      <span className="text-gray-300 dark:text-gray-600 shrink-0">&rarr;</span>
                      <span className="text-gray-400 shrink-0">{msg.to_agent.slice(0, 8)}</span>
                      <span className="text-gray-600 dark:text-gray-400 truncate flex-1">
                        {msg.payload.length > 60 ? msg.payload.slice(0, 60) + '...' : msg.payload}
                      </span>
                    </div>
                  ))
                ) : (
                  <div className="px-3 py-4 text-xs text-gray-400 text-center">
                    {selectedAgentId ? t('chat.multiAgent.noMessages') : t('chat.multiAgent.selectAgent')}
                  </div>
                )}
              </div>
            )}

            {activeTab === 'memory' && (
              <div className="py-1">
                {memoryEntries.length > 0 ? (
                  memoryEntries.map((entry, i) => (
                    <div key={i} className="px-3 py-1 text-xs border-b border-gray-100 dark:border-gray-800 last:border-b-0">
                      <div className="flex items-center gap-2">
                        <span className="font-mono font-medium text-purple-600 dark:text-purple-400">
                          {entry.key}
                        </span>
                        {entry.owner_agent && (
                          <span className="text-gray-400">{t('chat.multiAgent.by')} {entry.owner_agent.slice(0, 8)}</span>
                        )}
                        {entry.ttl_secs && (
                          <span className="text-gray-400">{t('chat.multiAgent.ttl')} {entry.ttl_secs}s</span>
                        )}
                      </div>
                      <div className="text-gray-600 dark:text-gray-400 truncate mt-0.5">
                        {entry.value.length > 100 ? entry.value.slice(0, 100) + '...' : entry.value}
                      </div>
                    </div>
                  ))
                ) : (
                  <div className="px-3 py-4 text-xs text-gray-400 text-center">
                    {t('chat.multiAgent.noMemory')}
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default MultiAgentStatusPanel;
