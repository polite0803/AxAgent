// SPDX-License-Identifier: AGPL-3.0-only

import type { GraphData } from "@/components/wiki/GraphView";
import type { Note } from "@/types";
import { SearchOutlined } from "@ant-design/icons";
import { Empty, Input, Space, Spin, theme, Tooltip, Tree, Typography } from "antd";
import type { DataNode } from "antd/es/tree";
import { ChevronDown, ChevronRight, FileText, FolderTree, Hash } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

const getTypeColorMap = (token: ReturnType<typeof theme.useToken>["token"]): Record<string, string> => ({
  note: token.colorPrimary,
  concept: token.colorSuccess,
  entity: "var(--orange, #fa8c16)",
  source: "var(--magenta, #eb2f96)",
});

function getTypeColor(type: string, token: ReturnType<typeof theme.useToken>["token"]): string {
  return getTypeColorMap(token)[type] || token.colorTextTertiary;
}

interface WikiFilePanelProps {
  notes: Note[];
  graphData: GraphData | null;
  loading: boolean;
  selectedNodeId: string | null;
  highlightedNodeIds: Set<string>;
  onSelectNode: (nodeId: string) => void;
  onSearchHighlight: (nodeIds: Set<string>) => void;
}

export function WikiFilePanel({
  notes,
  graphData,
  loading,
  selectedNodeId,
  onSelectNode,
  onSearchHighlight,
}: WikiFilePanelProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState("");
  const [expandedKeys, setExpandedKeys] = useState<React.Key[]>([]);
  const [manualExpand, setManualExpand] = useState(false);

  // 收集所有目录 key
  const getAllDirKeys = useCallback((nodes: DataNode[]): React.Key[] => {
    const keys: React.Key[] = [];
    const traverse = (nodeList: DataNode[]) => {
      for (const node of nodeList) {
        if (String(node.key).startsWith("dir:")) {
          keys.push(node.key);
        }
        if (node.children && node.children.length > 0) {
          traverse(node.children);
        }
      }
    };
    traverse(nodes);
    return keys;
  }, []);

  // 按路径构建树形结构
  const treeData = useMemo(() => {
    if (!notes || notes.length === 0) {
      return [];
    }

    const root: Record<
      string,
      { name: string; children: Record<string, unknown>; notes: Note[] }
    > = {};

    notes.forEach((note) => {
      const parts = note.filePath.split("/").filter(Boolean);
      let current = root;
      for (let i = 0; i < parts.length - 1; i++) {
        const part = parts[i];
        if (!current[part]) {
          current[part] = { name: part, children: {}, notes: [] };
        }
        current = current[part].children as typeof root;
      }
    });

    notes.forEach((note) => {
      const parts = note.filePath.split("/").filter(Boolean);
      let current = root;
      for (let i = 0; i < parts.length - 1; i++) {
        current = current[parts[i]].children as typeof root;
      }
      const lastDir = parts.length > 1 ? parts[parts.length - 2] : null;
      if (lastDir && current[lastDir]) {
        current[lastDir].notes.push(note);
      }
    });

    const buildTreeNode = (dirs: typeof root, depth: number): DataNode[] => {
      return Object.entries(dirs).map(([key, val]) => ({
        key: `dir:${key}`,
        title: (
          <Space size={4}>
            <FolderTree size={12} style={{ color: token.colorWarning }} />
            <Text style={{ fontSize: 13 }}>{key}</Text>
            <Text type="secondary" style={{ fontSize: 12 }}>
              ({val.notes.length})
            </Text>
          </Space>
        ),
        selectable: false,
        children: [
          ...buildTreeNode(val.children as typeof root, depth + 1),
          ...val.notes.map((note) => ({
            key: note.id,
            title: (
              <div className="flex items-center gap-1">
                <FileText size={11} />
                <span className="truncate text-sm">{note.title}</span>
                {note.author === "llm" && (
                  <span
                    className="text-[9px] px-1 py-px rounded-full font-medium"
                    style={{
                      backgroundColor: `${token.colorPrimary}18`,
                      color: token.colorPrimary,
                    }}
                  >
                    AI
                  </span>
                )}
              </div>
            ),
            isLeaf: true,
            selectable: true,
          })),
        ],
      }));
    };

    // 收集根目录的直接笔记
    const rootNotes = notes.filter(
      (n) =>
        !n.filePath.includes("/")
        || n.filePath.split("/").filter(Boolean).length === 1,
    );

    return [
      ...(rootNotes.length > 0
        ? [
          {
            key: "dir:root",
            title: (
              <Space size={4}>
                <FolderTree size={12} style={{ color: token.colorWarning }} />
                <Text style={{ fontSize: 13 }}>/</Text>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  ({rootNotes.length})
                </Text>
              </Space>
            ),
            selectable: false,
            children: rootNotes.map((note) => ({
              key: note.id,
              title: (
                <div className="flex items-center gap-1">
                  <FileText size={11} />
                  <span className="truncate text-sm">{note.title}</span>
                </div>
              ),
              isLeaf: true,
            })),
          },
        ]
        : []),
      ...buildTreeNode(
        (() => {
          const dirs: typeof root = {};
          notes.forEach((note) => {
            const parts = note.filePath.split("/").filter(Boolean);
            if (parts.length <= 1) {
              return;
            }
            const dirName = parts[0];
            if (!dirs[dirName]) {
              dirs[dirName] = { name: dirName, children: {}, notes: [] };
            }
            // Recurse for nested dirs
            let current = root;
            for (let i = 0; i < parts.length - 1; i++) {
              const p = parts[i];
              if (!current[p]) {
                current[p] = { name: p, children: {}, notes: [] };
              }
              current = current[p].children as typeof root;
            }
            current[parts[parts.length - 2]]?.notes.push(note);
          });
          return dirs;
        })(),
        0,
      ),
    ];
  }, [notes, token]);

  // 展开全部
  const handleExpandAll = useCallback(() => {
    setManualExpand(true);
    setExpandedKeys(getAllDirKeys(treeData));
  }, [getAllDirKeys, treeData]);

  // 折叠全部
  const handleCollapseAll = useCallback(() => {
    setManualExpand(true);
    setExpandedKeys([]);
  }, []);

  // 同步：首次加载或数据变化时自动展开（笔记数少于 50）
  useEffect(() => {
    if (!manualExpand && notes.length < 50 && treeData.length > 0) {
      setExpandedKeys(getAllDirKeys(treeData));
    }
  }, [treeData, notes.length, manualExpand, getAllDirKeys]);

  // 标签提取
  const allTags = useMemo(() => {
    if (!graphData) {
      return [];
    }
    const tagSet = new Set<string>();
    graphData.nodes.forEach((n) => n.tags.forEach((t) => tagSet.add(t)));
    return Array.from(tagSet).sort();
  }, [graphData]);

  const nodeTypes = useMemo(() => {
    const counts: Record<string, number> = {};
    graphData?.nodes.forEach((n) => {
      counts[n.type] = (counts[n.type] || 0) + 1;
    });
    return counts;
  }, [graphData]);

  const handleSearch = (value: string) => {
    setSearchQuery(value);
    if (!value.trim() || !graphData) {
      onSearchHighlight(new Set());
      return;
    }
    const q = value.toLowerCase();
    const matchedIds = new Set<string>();
    graphData.nodes.forEach((n) => {
      if (
        n.title.toLowerCase().includes(q)
        || n.tags.some((t) => t.toLowerCase().includes(q))
      ) {
        matchedIds.add(n.id);
      }
    });
    onSearchHighlight(matchedIds);
  };

  const handleTreeSelect = (keys: React.Key[]) => {
    if (keys.length > 0) {
      const key = String(keys[0]);
      if (!key.startsWith("dir:")) {
        onSelectNode(key);
      }
    }
  };

  const handleTagClick = (tag: string) => {
    if (!graphData) {
      return;
    }
    const ids = new Set(
      graphData.nodes.flatMap((n) => (n.tags.includes(tag) ? [n.id] : [])),
    );
    onSearchHighlight(ids);
  };

  return (
    <div
      className="h-full flex flex-col"
      style={{ backgroundColor: token.colorBgContainer }}
    >
      {/* 搜索 — 极致紧凑 */}
      <div
        className="px-2 pt-2 pb-1 shrink-0"
        style={{ borderBottom: `1px solid ${token.colorBorderSecondary}20` }}
      >
        <Input
          id="wiki-file-panel-input-69"
          prefix={<SearchOutlined style={{ color: token.colorTextQuaternary }} />}
          placeholder={t("wiki.searchPlaceholder")}
          value={searchQuery}
          onChange={(e) => handleSearch(e.target.value)}
          allowClear
          size="small"
          className="rounded-xl"
          style={{
            backgroundColor: `${token.colorBgElevated}80`,
            borderColor: `${token.colorBorderSecondary}40`,
          }}
        />
      </div>

      {/* 文件树控制栏 */}
      {!loading && notes.length > 0 && (
        <div className="flex items-center justify-between px-2 py-1 shrink-0">
          <span className="text-xs" style={{ color: token.colorTextTertiary }}>
            {notes.length} {t("wiki.graph.nodes")}
          </span>
          <div className="flex items-center gap-1">
            <Tooltip title={t("wiki.expandAll")}>
              <button
                type="button"
                onClick={handleExpandAll}
                className="flex items-center justify-center rounded hover:opacity-70"
                style={{
                  width: 22,
                  height: 22,
                  padding: 0,
                  border: `1px solid ${token.colorBorderSecondary}40`,
                  background: "transparent",
                  cursor: "pointer",
                  color: token.colorTextSecondary,
                }}
              >
                <ChevronDown size={12} />
              </button>
            </Tooltip>
            <Tooltip title={t("wiki.collapseAll")}>
              <button
                type="button"
                onClick={handleCollapseAll}
                className="flex items-center justify-center rounded hover:opacity-70"
                style={{
                  width: 22,
                  height: 22,
                  padding: 0,
                  border: `1px solid ${token.colorBorderSecondary}40`,
                  background: "transparent",
                  cursor: "pointer",
                  color: token.colorTextSecondary,
                }}
              >
                <ChevronRight size={12} />
              </button>
            </Tooltip>
          </div>
        </div>
      )}

      {/* 文件树 */}
      <div className="flex-1 overflow-y-auto px-1 py-0">
        {loading
          ? (
            <div className="flex justify-center mt-8">
              <Spin size="small" />
            </div>
          )
          : notes.length === 0
          ? (
            <Empty
              description={t("wiki.emptyNotes")}
              image={Empty.PRESENTED_IMAGE_SIMPLE}
            />
          )
          : (
            <Tree
              treeData={treeData}
              onSelect={handleTreeSelect}
              selectedKeys={selectedNodeId ? [selectedNodeId] : []}
              expandedKeys={expandedKeys}
              onExpand={(keys) => {
                setManualExpand(true);
                setExpandedKeys(keys);
              }}
              showIcon={false}
              blockNode
              className="wiki-file-tree"
              style={{ fontSize: 13 }}
            />
          )}
      </div>

      {/* 底部：标签云 + 类型统计（极致紧凑） */}
      <div
        className="shrink-0 px-2 py-1"
        style={{ borderTop: `1px solid ${token.colorBorderSecondary}20` }}
      >
        {allTags.length > 0 && (
          <div className="flex items-center gap-1.5 mb-1.5">
            <Hash size={10} style={{ color: token.colorTextQuaternary }} />
            <div className="flex flex-wrap gap-1 flex-1">
              {allTags.slice(0, 10).map((tag) => (
                <span
                  key={tag}
                  role="button"
                  tabIndex={0}
                  className="text-[10px] px-1.5 py-0.5 rounded-full cursor-pointer hover:opacity-80"
                  style={{
                    backgroundColor: `${token.colorPrimary}10`,
                    color: token.colorPrimary,
                    border: `1px solid ${token.colorPrimary}20`,
                  }}
                  onClick={() => handleTagClick(tag)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      handleTagClick(tag);
                    }
                  }}
                >
                  {tag}
                </span>
              ))}
              {allTags.length > 10 && (
                <Text type="secondary" className="text-[9px] self-center">
                  +{allTags.length - 10}
                </Text>
              )}
            </div>
          </div>
        )}

        {Object.keys(nodeTypes).length > 0 && (
          <div className="flex flex-wrap gap-1.5">
            {Object.entries(nodeTypes).slice(0, 5).map(([type, count]) => (
              <span key={type} className="flex items-center gap-1 text-[10px]">
                <span
                  className="size-1.5 rounded-full inline-block"
                  style={{ backgroundColor: getTypeColor(type, token) }}
                />
                <span style={{ color: token.colorTextTertiary }}>
                  {type} {count}
                </span>
              </span>
            ))}
            {Object.keys(nodeTypes).length > 5 && (
              <Text type="secondary" className="text-[9px]">
                +{Object.keys(nodeTypes).length - 5}
              </Text>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
