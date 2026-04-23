import { Card, theme } from 'antd';
import type { CSSProperties, ReactNode } from 'react';

interface SettingsGroupProps {
  title?: ReactNode;
  children: ReactNode;
  extra?: ReactNode;
  style?: CSSProperties;
}

export function SettingsGroup({ title, children, extra, style }: SettingsGroupProps) {
  const { token } = theme.useToken();
  const isLight = parseInt(token.colorBgBase.replace('#', '').substring(0, 2), 16) > 200;
  const cardBg = isLight ? '#fcfcfc' : token.colorBgContainer;

  return (
    <div style={{ marginBottom: 20, ...style }}>
      {(title || extra) && (
        <div style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 6,
          padding: '0 4px',
        }}>
          {title && (
            <div style={{
              fontSize: 13,
              fontWeight: 600,
              color: token.colorTextSecondary,
            }}>
              {title}
            </div>
          )}
          {extra}
        </div>
      )}
      <Card
        size="small"
        style={{
          borderRadius: 10,
          border: 'none',
          backgroundColor: cardBg,
          boxShadow: `0 0 0 0.5px ${token.colorBorderSecondary}`,
        }}
      >
        {children}
      </Card>
    </div>
  );
}
