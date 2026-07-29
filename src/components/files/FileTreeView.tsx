// SPDX-License-Identifier: AGPL-3.0-only

import { showBackendError } from "@/lib/errorI18n";
import {
  createDirectory,
  deleteEntry,
  type DirEntry,
  listDirectory,
  moveEntry,
  renameEntry,
} from "@/lib/fileBrowserApi";
import { App, Button, Dropdown, Empty, Input, Modal, Spin, theme, Tree } from "antd";
import type { MenuProps } from "antd";
import type { DataNode } from "antd/es/tree";
import { File as FileIcon, Folder, FolderOpen, FolderPlus, Pencil, Trash2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

/** 扩展 DataNode，携带后端返回的路径与类型信息 */
interface TreeNodeData extends DataNode {
  path: string;
  isDir: boolean;
  name: string;
}

interface FileTreeViewProps {
  /** 初始根目录的绝对路径 */
  rootPath: string;
  /** 选中文件时触发预览回调 */
  onSelectFile?: (path: string) => void;
}

/** 把后端 DirEntry 转为 Tree 节点 */
function toTreeNode(e: DirEntry): TreeNodeData {
  return {
    key: e.path,
    path: e.path,
    isDir: e.isDir,
    name: e.name,
    isLeaf: !e.isDir,
    title: e.name,
  };
}

/** 递归更新指定路径节点的 children */
function updateChildren(
  nodes: TreeNodeData[],
  targetPath: string,
  children: TreeNodeData[],
): TreeNodeData[] {
  return nodes.map((n) => {
    if (n.path === targetPath) {
      return { ...n, children };
    }
    if (n.children && (n.children as TreeNodeData[]).length > 0) {
      return { ...n, children: updateChildren(n.children as TreeNodeData[], targetPath, children) };
    }
    return n;
  });
}

export function FileTreeView({ rootPath, onSelectFile }: FileTreeViewProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message } = App.useApp();

  const [treeData, setTreeData] = useState<TreeNodeData[]>([]);
  const [loadedKeys, setLoadedKeys] = useState<React.Key[]>([]);
  const [expandedKeys, setExpandedKeys] = useState<React.Key[]>([]);
  const [selectedKeys, setSelectedKeys] = useState<React.Key[]>([]);
  const [loading, setLoading] = useState(false);
  const [refreshTrigger, setRefreshTrigger] = useState(0);

  // 重命名 Modal
  const [renameState, setRenameState] = useState<{ node: TreeNodeData } | null>(null);
  const [renameValue, setRenameValue] = useState("");
  // 新建文件夹 Modal
  const [mkdirState, setMkdirState] = useState<{ parentPath: string } | null>(null);
  const [mkdirValue, setMkdirValue] = useState("");
  // 移动 Modal
  const [moveState, setMoveState] = useState<{ srcPath: string } | null>(null);
  const [moveValue, setMoveValue] = useState("");

  // 加载根目录
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    listDirectory(rootPath)
      .then((entries) => {
        if (!cancelled) {
          setTreeData(entries.map(toTreeNode));
          setLoadedKeys([]);
          setSelectedKeys([]);
        }
      })
      .catch((e: unknown) => {
        if (!cancelled) { showBackendError(message, e, { context: "listDirectory(root)" }); }
      })
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, [rootPath, refreshTrigger, message]);

  // 懒加载子目录
  const onLoadData = useCallback(
    ({ key }: { key: React.Key }) => {
      const path = String(key);
      return listDirectory(path)
        .then((entries) => {
          setTreeData((prev) => updateChildren(prev, path, entries.map(toTreeNode)));
        })
        .catch((e: unknown) => {
          showBackendError(message, e, { context: "listDirectory" });
        });
    },
    [message],
  );

  const refresh = useCallback(() => {
    setRefreshTrigger((x) => x + 1);
  }, []);

  const handleSelect = useCallback(
    (keys: React.Key[], info: { node: TreeNodeData }) => {
      setSelectedKeys(keys);
      const node = info.node as TreeNodeData;
      if (!node.isDir && onSelectFile) {
        onSelectFile(node.path);
      }
    },
    [onSelectFile],
  );

  // ── 右键菜单操作 ──

  const openRename = useCallback((node: TreeNodeData) => {
    setRenameState({ node });
    setRenameValue(node.name);
  }, []);

  const confirmRename = useCallback(async () => {
    if (!renameState) { return; }
    const newName = renameValue.trim();
    if (!newName || newName === renameState.node.name) {
      setRenameState(null);
      return;
    }
    try {
      await renameEntry(renameState.node.path, newName);
      message.success(t("files.renameSuccess"));
      setRenameState(null);
      refresh();
    } catch (e) {
      showBackendError(message, e, { context: "renameEntry" });
    }
  }, [renameState, renameValue, message, t, refresh]);

  const openMkdir = useCallback((parentPath: string) => {
    setMkdirState({ parentPath });
    setMkdirValue("");
  }, []);

  const confirmMkdir = useCallback(async () => {
    if (!mkdirState) { return; }
    const name = mkdirValue.trim();
    if (!name) {
      setMkdirState(null);
      return;
    }
    // F-P1-4: 检测父路径使用的分隔符，避免 Windows 上拼出混合分隔符路径
    const sep = /[\\/]/.test(mkdirState.parentPath)
      ? (mkdirState.parentPath.includes("\\") ? "\\" : "/")
      : "/";
    const fullPath = `${mkdirState.parentPath}${sep}${name}`;
    try {
      await createDirectory(fullPath);
      message.success(t("files.mkdirSuccess"));
      setMkdirState(null);
      refresh();
    } catch (e) {
      showBackendError(message, e, { context: "createDirectory" });
    }
  }, [mkdirState, mkdirValue, message, t, refresh]);

  const openMove = useCallback((srcPath: string) => {
    setMoveState({ srcPath });
    setMoveValue("");
  }, []);

  const confirmMove = useCallback(async () => {
    if (!moveState) { return; }
    const dstDir = moveValue.trim();
    if (!dstDir) {
      setMoveState(null);
      return;
    }
    try {
      await moveEntry(moveState.srcPath, dstDir);
      message.success(t("files.moveSuccess"));
      setMoveState(null);
      refresh();
    } catch (e) {
      showBackendError(message, e, { context: "moveEntry" });
    }
  }, [moveState, moveValue, message, t, refresh]);

  const confirmDelete = useCallback(
    (node: TreeNodeData) => {
      Modal.confirm({
        title: t("files.deleteConfirmTitle"),
        content: t("files.deleteConfirmContent", { name: node.name }),
        okText: t("files.confirmYes"),
        cancelText: t("files.confirmNo"),
        okButtonProps: { danger: true },
        onOk: async () => {
          try {
            await deleteEntry(node.path, node.isDir);
            message.success(t("files.deleteSuccess"));
            refresh();
          } catch (e) {
            showBackendError(message, e, { context: "deleteEntry" });
          }
        },
      });
    },
    [message, t, refresh],
  );

  const buildMenuItems = useCallback(
    (node: TreeNodeData): MenuProps["items"] => {
      const items: MenuProps["items"] = [];
      if (node.isDir) {
        items.push({
          key: "mkdir",
          label: t("files.contextMkdir"),
          icon: <FolderPlus size={14} />,
          onClick: () => openMkdir(node.path),
        });
      }
      items.push(
        {
          key: "rename",
          label: t("files.contextRename"),
          icon: <Pencil size={14} />,
          onClick: () => openRename(node),
        },
        {
          key: "move",
          label: t("files.contextMove"),
          icon: <FolderOpen size={14} />,
          onClick: () => openMove(node.path),
        },
        { type: "divider" },
        {
          key: "delete",
          label: t("files.contextDelete"),
          icon: <Trash2 size={14} />,
          danger: true,
          onClick: () => confirmDelete(node),
        },
      );
      return items;
    },
    [t, openMkdir, openRename, openMove, confirmDelete],
  );

  const titleRender = useCallback(
    (node: DataNode) => {
      const data = node as TreeNodeData;
      const Icon = data.isDir ? Folder : FileIcon;
      const iconColor = data.isDir ? token.colorWarning : token.colorTextSecondary;
      return (
        <Dropdown menu={{ items: buildMenuItems(data) }} trigger={["contextMenu"]}>
          <div
            className="inline-flex items-center gap-1.5 px-1 py-0.5 rounded"
            style={{ minWidth: 0 }}
          >
            <Icon size={13} style={{ color: iconColor, flexShrink: 0 }} />
            <span
              className="truncate text-[13px]"
              style={{ color: token.colorText }}
              title={data.name}
            >
              {data.name}
            </span>
          </div>
        </Dropdown>
      );
    },
    [buildMenuItems, token],
  );

  const tree = useMemo(() => {
    return (
      <Tree<TreeNodeData>
        treeData={treeData}
        loadData={onLoadData}
        loadedKeys={loadedKeys}
        onLoad={(keys) => setLoadedKeys(keys)}
        expandedKeys={expandedKeys}
        onExpand={(keys) => setExpandedKeys(keys)}
        selectedKeys={selectedKeys}
        onSelect={handleSelect}
        titleRender={titleRender}
        showIcon
        blockNode
        style={{ fontSize: 13 }}
      />
    );
  }, [
    treeData,
    onLoadData,
    loadedKeys,
    expandedKeys,
    selectedKeys,
    handleSelect,
    titleRender,
  ]);

  return (
    <div
      className="h-full flex flex-col"
      style={{ backgroundColor: token.colorBgContainer }}
      data-testid="file-tree-view"
    >
      {/* 顶部：根路径 + 新建文件夹按钮 */}
      <div
        className="px-2 py-1.5 shrink-0 flex items-center justify-between gap-2"
        style={{ borderBottom: `1px solid ${token.colorBorderSecondary}` }}
      >
        <span
          className="truncate text-[11px]"
          style={{ color: token.colorTextTertiary }}
          title={rootPath}
        >
          {rootPath}
        </span>
        <Button
          size="small"
          type="text"
          icon={<FolderPlus size={14} />}
          onClick={() => openMkdir(rootPath)}
          aria-label={t("files.contextMkdir")}
        />
      </div>

      {/* 树体 */}
      <div className="flex-1 overflow-y-auto px-1 py-1">
        {loading
          ? (
            <div className="flex justify-center mt-6">
              <Spin size="small" />
            </div>
          )
          : treeData.length === 0
          ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={t("files.treeEmpty")}
            />
          )
          : tree}
      </div>

      {/* 重命名 Modal */}
      <Modal
        title={t("files.renameTitle")}
        open={renameState !== null}
        onOk={() => void confirmRename()}
        onCancel={() => setRenameState(null)}
        okText={t("files.confirmYes")}
        cancelText={t("files.confirmNo")}
        destroyOnHidden
      >
        <Input
          id="file-tree-view-rename-input"
          value={renameValue}
          onChange={(e) => setRenameValue(e.target.value)}
          onPressEnter={() => void confirmRename()}
          autoFocus
        />
      </Modal>

      {/* 新建文件夹 Modal */}
      <Modal
        title={t("files.mkdirTitle")}
        open={mkdirState !== null}
        onOk={() => void confirmMkdir()}
        onCancel={() => setMkdirState(null)}
        okText={t("files.confirmYes")}
        cancelText={t("files.confirmNo")}
        destroyOnHidden
      >
        <Input
          id="file-tree-view-mkdir-input"
          value={mkdirValue}
          onChange={(e) => setMkdirValue(e.target.value)}
          onPressEnter={() => void confirmMkdir()}
          placeholder={t("files.mkdirPlaceholder")}
          autoFocus
        />
      </Modal>

      {/* 移动 Modal */}
      <Modal
        title={t("files.moveTitle")}
        open={moveState !== null}
        onOk={() => void confirmMove()}
        onCancel={() => setMoveState(null)}
        okText={t("files.confirmYes")}
        cancelText={t("files.confirmNo")}
        destroyOnHidden
      >
        <Input
          id="file-tree-view-move-input"
          value={moveValue}
          onChange={(e) => setMoveValue(e.target.value)}
          onPressEnter={() => void confirmMove()}
          placeholder={t("files.movePlaceholder")}
          autoFocus
        />
      </Modal>
    </div>
  );
}
