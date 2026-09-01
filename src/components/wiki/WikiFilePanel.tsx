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

  // 按目录路径构建树形结构（F5：原骨架构建 / 笔记填充 / 二次重建三段重复逻辑收敛为 dirMap + 递归构建，
  // 并修复单层目录笔记（如 docs/note.md）被遗漏的 bug）
  const treeData = useMemo(() => {
    if (!notes || notes.length === 0) {
      return [];
    }

    // 目录完整路径 → 直接位于该目录的笔记（根目录路径为 ""）
    const dirMap = new Map<string, Note[]>();
    notes.forEach((note) => {
      const parts = note.filePath.split("/").filter(Boolean);
      const dirPath = parts.slice(0, -1).join("/");
      const bucket = dirMap.get(dirPath);
      if (bucket) {
        bucket.push(note);
      } else {
        dirMap.set(dirPath, [note]);
      }
    });

    const renderNoteNode = (note: Note) => ({
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
    });

    // 递归构建目录节点：key 用完整目录路径，避免不同父目录下同名子目录 key 冲突
    const buildDirNodes = (parent: string): DataNode[] => {
      const prefix = parent === "" ? "" : `${parent}/`;
      const childDirs = [...dirMap.keys()]
        .filter(
          (d) =>
            d !== ""
            && d.startsWith(prefix)
            && !d.slice(prefix.length).includes("/"),
        )
        .sort();
      return childDirs.map((dirPath) => {
        const dirNotes = dirMap.get(dirPath) ?? [];
        return {
          key: `dir:${dirPath}`,
          title: (
            <Space size={4}>
              <FolderTree size={12} style={{ color: token.colorWarning }} />
              <Text style={{ fontSize: 13 }}>{dirPath.slice(prefix.length)}</Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                ({dirNotes.length})
              </Text>
            </Space>
          ),
          selectable: false,
          children: [
            ...buildDirNodes(dirPath),
            ...dirNotes.map(renderNoteNode),
          ],
        };
      });
    };

    // 根目录直接笔记收纳在 "/" 节点下
    const rootNotes = dirMap.get("") ?? [];
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
            children: rootNotes.map(renderNoteNode),
          },
        ]
        : []),
      ...buildDirNodes(""),
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
                  {t(`wiki.graph.nodeType.${type}`, { defaultValue: type })} {count}
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
