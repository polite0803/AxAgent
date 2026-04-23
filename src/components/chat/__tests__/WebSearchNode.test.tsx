import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { WebSearchNode } from '../WebSearchNode';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const translations: Record<string, string> = {
        'chat.search.searching': '正在联网搜索...',
        'chat.search.done': '联网搜索完成',
        'chat.search.error': '搜索失败',
      };
      return translations[key] ?? key;
    },
  }),
}));

describe('WebSearchNode', () => {
  it('shows the searching state when attrs come from tuple pairs', () => {
    render(
      <WebSearchNode
        node={{
          type: 'web-search',
          attrs: [['status', 'searching']],
          content: '',
          loading: false,
        }}
      />,
    );

    expect(screen.getByText('正在联网搜索...')).toBeInTheDocument();
  });
});
