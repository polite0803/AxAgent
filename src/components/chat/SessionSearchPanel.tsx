import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@/lib/invoke';
import { useConversationStore } from '@/stores';
import type { SessionSearchResult } from '@/types';

export default function SessionSearchPanel() {
  const { t } = useTranslation();
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SessionSearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState(false);

  const handleSearch = useCallback(async () => {
    if (!query.trim()) return;
    setSearching(true);
    setError(null);
    try {
      const res = await invoke<SessionSearchResult[]>('session_search', { query: query.trim(), limit: 20 });
      setResults(res);
    } catch (e) {
      setError(String(e));
      setResults([]);
    } finally {
      setSearching(false);
    }
  }, [query]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleSearch();
      }
    },
    [handleSearch]
  );

  if (!expanded) {
    return (
      <div className="border-b border-border/50 px-3 py-2">
        <button
          onClick={() => setExpanded(true)}
          className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
          </svg>
          {t('chat.searchPastSessions')}
        </button>
      </div>
    );
  }

  return (
    <div className="border-b border-border/50 px-3 py-2 space-y-2">
      <div className="flex items-center gap-2">
        <div className="flex-1 flex items-center gap-1.5">
          <svg className="w-3.5 h-3.5 text-muted-foreground shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
          </svg>
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={t('chat.searchPastConversations')}
            className="flex-1 bg-transparent text-xs outline-none placeholder:text-muted-foreground/60"
            autoFocus
          />
        </div>
        <button
          onClick={handleSearch}
          disabled={searching || !query.trim()}
          className="text-xs px-2 py-0.5 rounded bg-primary/10 text-primary hover:bg-primary/20 disabled:opacity-40 transition-colors"
        >
          {searching ? '...' : t('chat.searchGo')}
        </button>
        <button
          onClick={() => {
            setExpanded(false);
            setQuery('');
            setResults([]);
            setError(null);
          }}
          className="text-muted-foreground hover:text-foreground transition-colors"
        >
          <svg className="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      {error && (
        <div className="text-xs text-destructive/80">{error}</div>
      )}

      {results.length > 0 && (
        <div className="max-h-48 overflow-y-auto space-y-1.5">
          <div className="text-[10px] text-muted-foreground">
            {t('chat.searchResults', { count: results.length })}
          </div>
          {results.map((r, i) => (
            <div
              key={`${r.conversation_id}-${i}`}
              className="text-xs p-1.5 rounded bg-muted/30 hover:bg-muted/50 cursor-pointer transition-colors"
              onClick={() => {
                // Navigate to the conversation
                useConversationStore.getState().setActiveConversation(r.conversation_id);
              }}
            >
              <div className="flex items-center gap-1 mb-0.5">
                <span className={`inline-block px-1 rounded text-[10px] font-medium ${
                  r.role === 'user' ? 'bg-blue-500/10 text-blue-500' : 'bg-green-500/10 text-green-500'
                }`}>
                  {r.role}
                </span>
                <span className="text-[10px] text-muted-foreground truncate max-w-50">
                  {r.conversation_title}
                </span>
                <span className="text-[10px] text-muted-foreground/60 ml-auto">
                  {r.rank < -5 ? 'high' : r.rank < -2 ? 'med' : 'low'}
                </span>
              </div>
              <div
                className="text-[11px] text-foreground/80 line-clamp-2"
                dangerouslySetInnerHTML={{
                  __html: r.snippet
                    .replace(/>>/g, '<mark class="bg-yellow-300/30 rounded px-0.5">')
                    .replace(/<</g, '</mark>')
                }}
              />
            </div>
          ))}
        </div>
      )}

      {!searching && results.length === 0 && query && !error && (
        <div className="text-xs text-muted-foreground/60">{t('chat.searchNoResults')}</div>
      )}
    </div>
  );
}
