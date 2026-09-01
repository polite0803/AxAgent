// SPDX-License-Identifier: AGPL-3.0-only

// wikilink 片段高亮共享实现。
// F5 去重：原 BacklinkPanel / WikiDetailPanel 各持一份完全相同的拷贝，统一收编至此。
import { theme, Typography } from "antd";
import type { ReactNode } from "react";

type AntdToken = ReturnType<typeof theme.useToken>["token"];

export function highlightWikilink(
  snippet: string,
  linkText: string,
  token: AntdToken,
): ReactNode {
  const linkPattern = `[[${linkText}]]`;
  const parts = snippet.split(linkPattern);
  if (parts.length === 1) {
    return <span>{snippet}</span>;
  }

  const { Text } = Typography;
  return (
    <span>
      {parts.map((part, i) => (
        // 静态文本分割列表，基于索引的 key 安全
        <span key={i}>
          {part}
          {i < parts.length - 1 && (
            <Text
              strong
              style={{
                backgroundColor: `${token.colorPrimary}1F`,
                borderRadius: 3,
                padding: "0 2px",
              }}
            >
              {linkPattern}
            </Text>
          )}
        </span>
      ))}
    </span>
  );
}
