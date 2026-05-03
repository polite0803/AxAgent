import type { GraphData } from "@/components/wiki/GraphView";
import type { Note } from "@/types";
import { SearchOutlined } from "@ant-design/icons";
import { Empty, Input, Space, Spin, theme, Tree, Typography } from "antd";
import type { DataNode } from "antd/es/tree";
import { FileText, FolderTree, Hash } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

const typeColorMap: Record<string, string> = {
  note: "#1890ff",
  concept: "#52c41a",
  entity: "#fa8c16",
  source: "#eb2f96",
};

function getTypeColor(type: string): string {
  return typeColorMap[type] || "#888";
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

  // 按路径构建树形结构
  const treeData = useMemo(() => {
    if (!notes || notes.length === 0) { return []; }

    const root: Record<string, { name: string; children: Record<string, unknown>; notes: Note[] }> = {};

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

    const buildTreeNode = (
      dirs: typeof root,
      depth: number,
    ): DataNode[] => {
      return Object.entries(dirs).map(([key, val]) => ({
        key: `dir:${key}`,
        title: (
          <Space size={4}>
            <FolderTree size={12} style={{ color: token.colorWarning }} />
            <Text style={{ fontSize: 13 }}>{key}</Text>
            <Text type="secondary" style={{ fontSize: 11 }}>
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
              <div
                className="flex items-center gap-1"
                style={{
                  color: selectedNodeId === note.id ? token.colorPrimary : undefined,
                  fontWeight: selectedNodeId === note.id ? 600 : undefined,
                }}
              >
                <FileText size={11} />
                <span className="truncate text-sm">{note.title}</span>
                {note.author === "llm" && (
                  <span
                    className="text-[9px] px-1 py-px rounded-full font-medium"
                    style={{ backgroundColor: `${token.colorPrimary}18`, color: token.colorPrimary }}
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
    const rootNotes = notes.filter((n) =>
      !n.filePath.includes("/") || n.filePath.split("/").filter(Boolean).length === 1
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
                <Text type="secondary" style={{ fontSize: 11 }}>
                  ({rootNotes.length})
                </Text>
              </Space>
            ),
            selectable: false,
            children: rootNotes.map((note) => ({
              key: note.id,
              title: (
                <div
                  className="flex items-center gap-1"
                  style={{
                    color: selectedNodeId === note.id ? token.colorPrimary : undefined,
                    fontWeight: selectedNodeId === note.id ? 600 : undefined,
                  }}
                >
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
            if (parts.length <= 1) { return; }
            const dirName = parts[0];
            if (!dirs[dirName]) {
              dirs[dirName] = { name: dirName, children: {}, notes: [] };
            }
            // Recurse for nested dirs
            let current = root;
            for (let i = 0; i < parts.length - 1; i++) {
              const p = parts[i];
              if (!current[p]) { current[p] = { name: p, children: {}, notes: [] }; }
              current = current[p].children as typeof root;
            }
            current[parts[parts.length - 2]]?.notes.push(note);
          });
          return dirs;
        })(),
        0,
      ),
    ];
  }, [notes, selectedNodeId, token]);

  // 标签提取
  const allTags = useMemo(() => {
    if (!graphData) { return []; }
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
      if (n.title.toLowerCase().includes(q) || n.tags.some((t) => t.toLowerCase().includes(q))) {
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
    if (!graphData) { return; }
    const ids = new Set(
      graphData.nodes.filter((n) => n.tags.includes(tag)).map((n) => n.id),
    );
    onSearchHighlight(ids);
  };

  return (
    <div className="h-full flex flex-col" style={{ backgroundColor: token.colorBgContainer }}>
      {/* 搜索 — 玻璃态 */}
      <div
        className="px-3 pt-3 pb-2 shrink-0"
        style={{ borderBottom: `1px solid ${token.colorBorderSecondary}10` }}
      >
        <Input
          prefix={<SearchOutlined style={{ color: token.colorTextQuaternary }} />}
          placeholder={t("wiki.searchPlaceholder", "Search notes...")}
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

      {/* 文件树 */}
      <div className="flex-1 overflow-y-auto px-2 py-1">
        {loading
          ? (
            <div className="flex justify-center mt-8">
              <Spin size="small" />
            </div>
          )
          : notes.length === 0
          ? (
            <Empty
              description={t("wiki.emptyNotes", "No notes")}
              image={Empty.PRESENTED_IMAGE_SIMPLE}
            />
          )
          : (
            <Tree
              treeData={treeData}
              onSelect={handleTreeSelect}
              selectedKeys={selectedNodeId ? [selectedNodeId] : []}
              defaultExpandAll={notes.length < 50}
              showIcon={false}
              blockNode
              className="wiki-file-tree"
              style={{ fontSize: 13 }}
            />
          )}
      </div>

      {/* 底部：标签云 + 类型统计 */}
      <div
        className="shrink-0 px-3 py-3"
        style={{ borderTop: `1px solid ${token.colorBorderSecondary}10` }}
      >
        <div className="flex items-center gap-1.5 mb-2.5">
          <Hash size={10} style={{ color: token.colorTextQuaternary }} />
          <Text type="secondary" className="text-[11px] font-medium tracking-wide uppercase">
            {t("wiki.tags", "Tags")}
          </Text>
        </div>
        {allTags.length === 0
          ? (
            <Text type="secondary" className="text-[11px]">
              {t("wiki.noTags", "No tags")}
            </Text>
          )
          : (
            <div className="flex flex-wrap gap-1 mb-3">
              {allTags.slice(0, 20).map((tag) => (
                <span
                  key={tag}
                  className="text-[11px] px-2 py-0.5 rounded-full cursor-pointer transition-all duration-200 hover:scale-105"
                  style={{
                    backgroundColor: `${token.colorPrimary}10`,
                    color: token.colorPrimary,
                    border: `1px solid ${token.colorPrimary}20`,
                  }}
                  onClick={() => handleTagClick(tag)}
                >
                  {tag}
                </span>
              ))}
              {allTags.length > 20 && (
                <Text type="secondary" className="text-[10px] self-center">
                  +{allTags.length - 20}
                </Text>
              )}
            </div>
          )}

        {Object.keys(nodeTypes).length > 0 && (
          <div className="flex flex-wrap gap-2">
            {Object.entries(nodeTypes).map(([type, count]) => (
              <div
                key={type}
                className="flex items-center gap-1 text-[10px]"
              >
                <span
                  className="w-2 h-2 rounded-full inline-block"
                  style={{ backgroundColor: getTypeColor(type) }}
                />
                <span style={{ color: token.colorTextSecondary }}>
                  {type}: {count}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
