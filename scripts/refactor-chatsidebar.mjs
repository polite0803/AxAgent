import { readFileSync, writeFileSync } from "node:fs";

let f = readFileSync("src/components/chat/ChatSidebar.tsx", "utf-8");

// ── 1. conversationItems type: ConversationItemType[] → ConvItem[] ──
f = f.replace(/const conversationItems: ConversationItemType\[]/g, "const conversationItems: ConvItem[]");

// ── 2. buildConvItem return type ──
f = f.replace(/: ConversationItemType =>/g, ": ConvItem =>");

// ── 3. renderGroupLabel and handleRename/handleDelete param types ──
f = f.replace(/\(item: ConversationItemType\)/g, "(item: ConvItem)");

// ── 4. Replace menuConfig entirely ──
const menuConfigStart = f.indexOf("  const menuConfig = useCallback(");
const menuConfigEnd = f.indexOf("  const handleConversationClick", menuConfigStart);
if (menuConfigStart === -1 || menuConfigEnd === -1) {
  console.error("Cannot find menuConfig boundaries");
  process.exit(1);
}

const buildMenuItems = `  const buildMenuItems = useCallback(
    (convId: string): DropdownItem[] => {
      const conv = conversations.find((c) => c.id === convId);
      if (!conv) return [];
      const isPinned = conv.is_pinned ?? false;
      const title = conv.title ?? "";
      const hasParent = !!conv.parent_conversation_id;
      const parentId = conv.parent_conversation_id;

      const wsChildren = wsDirs.flatMap((d) =>
        d !== conv.workspace_dir
          ? [{
            key: \`move-ws:\${d}\`,
            label: (
              <span className="truncate" style={{ maxWidth: 180, display: "inline-block" }}>
                {d}
              </span>
            ),
            onClick: () => {
              void invoke("agent_update_session", {
                conversationId: conv.id,
                cwd: d,
              });
            },
          }]
          : []
      );
      if (conv.workspace_dir) {
        wsChildren.unshift({
          key: "remove-ws",
          label: (
            <span style={{ fontStyle: "italic", opacity: 0.6 }}>
              {t("chat.removeFromWorkspace")}
            </span>
          ),
          onClick: () => {
            void invoke("agent_update_session", {
              conversationId: conv.id,
              cwd: null,
            });
          },
        });
      }

      return [
        {
          key: "pin",
          label: isPinned ? t("chat.unpin") : t("chat.pin"),
          icon: isPinned ? <PinOff size={14} /> : <Pin size={14} />,
          onClick: () => togglePin(conv.id),
        },
        {
          key: "archive",
          label: t("chat.archive"),
          icon: <Archive size={14} />,
          onClick: () => { void handleArchiveSingle(conv.id); },
        },
        {
          key: "ai-title",
          label: t("chat.aiGenerateTitle"),
          icon: <Sparkles size={14} />,
          onClick: () => { void regenerateTitle(conv.id); },
        },
        {
          key: "fork",
          label: t("chat.forkConversation"),
          icon: <GitFork size={14} />,
          onClick: () => { void forkConversation(conv.id); },
        },
        {
          key: "copy-id",
          label: t("chat.copyConversationId"),
          icon: <Copy size={14} />,
          onClick: () => {
            void navigator.clipboard
              .writeText(conv.id)
              .then(() => messageApi.success(t("chat.copied")));
          },
        },
        {
          key: "copy-transcript",
          label: (
            <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
              <Copy size={14} />
              {t("chat.copyTranscript")}
            </span>
          ),
          children: [
            {
              key: "copy-md",
              label: "Markdown",
              icon: <FileCode size={14} />,
              onClick: () => {
                (async () => {
                  try {
                    const msgs = await invoke<Message[]>("list_messages", { conversationId: conv.id });
                    if (msgs.length === 0) { messageApi.warning(t("chat.noMessages")); return; }
                    await copyTranscript(msgs, title, "markdown");
                    messageApi.success(t("chat.copied"));
                  } catch (_e) { messageApi.error(t("chat.copyFailed")); }
                })();
              },
            },
            {
              key: "copy-txt",
              label: t("chat.exportTxt"),
              icon: <FileType size={14} />,
              onClick: () => {
                (async () => {
                  try {
                    const msgs = await invoke<Message[]>("list_messages", { conversationId: conv.id });
                    if (msgs.length === 0) { messageApi.warning(t("chat.noMessages")); return; }
                    await copyTranscript(msgs, title, "text");
                    messageApi.success(t("chat.copied"));
                  } catch (_e) { messageApi.error(t("chat.copyFailed")); }
                })();
              },
            },
          ],
        },
        ...(wsChildren.length > 0
          ? [{
            key: "move-workspace",
            label: (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
                <FolderOpen size={14} />
                {t("chat.moveToWorkspace")}
              </span>
            ),
            children: wsChildren.slice(0, 15),
          }]
          : []),
        ...(hasParent
          ? [{
            key: "detach-parent",
            label: t("chat.detachFromParent"),
            icon: <Link2 size={14} style={{ transform: "rotate(45deg)" }} />,
            onClick: () => { void updateConversation(conv.id, { parent_conversation_id: null }); },
          }]
          : []),
        ...(hasParent && parentId
          ? [{
            key: "go-parent",
            label: t("chat.goToParent"),
            icon: <ChevronRight size={14} style={{ transform: "rotate(180deg)" }} />,
            onClick: () => setActiveConversation(parentId),
          }]
          : []),
        {
          key: "rename",
          label: t("chat.rename"),
          icon: <Pencil size={14} />,
          onClick: () => {
            const item: ConvItem = { key: conv.id, label: title, icon: null, group: "" };
            handleRename(item);
          },
        },
        {
          key: "export",
          label: (
            <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
              <Share size={14} />
              {t("chat.export")}
            </span>
          ),
          children: buildExportChildren(conv.id, title),
        },
        {
          key: "delete",
          label: t("chat.delete"),
          icon: <Trash2 size={14} />,
          danger: true,
          onClick: () => {
            const item: ConvItem = { key: conv.id, label: title, icon: null, group: "" };
            handleDelete(item);
          },
        },
      ];
    },
    [
      t, conversations, wsDirs, regenerateTitle, forkConversation,
      updateConversation, handleRename, handleDelete, togglePin, toggleArchive,
      buildExportChildren, handleArchiveSingle,
      setActiveConversation, messageApi,
    ],
  );`;

f = f.slice(0, menuConfigStart) + buildMenuItems + "\n\n" + f.slice(menuConfigEnd);

// ── 5. Remove rightClickMenuConfig ──
const rcmStart = f.indexOf("  const rightClickMenuConfig = useMemo(() => {");
const rcmEnd = f.indexOf("  if (isCollapsed)", rcmStart);
if (rcmStart !== -1 && rcmEnd !== -1) {
  // Find the end of the rightClickMenuConfig (look for the closing "  }, [" pattern after it)
  const afterRcm = f.indexOf("\n  if (isCollapsed)", rcmStart);
  if (afterRcm !== -1) {
    f = f.slice(0, rcmStart) + f.slice(afterRcm);
  }
}

// ── 6. Replace the JSX rendering block ──
// Find and replace the entire <Dropdown> + <Conversations> block
const dropdownStart = f.indexOf("          <Dropdown");
if (dropdownStart === -1) {
  console.error("Cannot find Dropdown in JSX");
  process.exit(1);
}

// Find the matching closing tag
const convBlockEnd = f.indexOf("      <Modal", dropdownStart);
if (convBlockEnd === -1) {
  console.error("Cannot find end of Dropdown+Conversations block");
  process.exit(1);
}

const nativeList = `          <div className="flex-1 overflow-y-auto">
              <div
                onContextMenu={(e) => {
                  if (multiSelectMode) {
                    e.preventDefault();
                    e.stopPropagation();
                    return;
                  }
                  const listItem = (e.target as HTMLElement).closest(
                    "[data-conv-id]",
                  ) as HTMLElement;
                  if (!listItem) {
                    e.preventDefault();
                    e.stopPropagation();
                    return;
                  }
                  const convId = listItem.getAttribute("data-conv-id");
                  if (!convId) {
                    e.preventDefault();
                    e.stopPropagation();
                    return;
                  }
                  e.preventDefault();
                  setContextMenuState({ x: e.clientX, y: e.clientY, convId });
                }}
              >
                {conversationsLoading && conversations.length === 0
                  ? (
                    <div style={{ padding: "8px 12px" }}>
                      {Array.from({ length: 6 }).map((_, i) => (
                        <div
                          key={i}
                          className="conv-skeleton"
                          style={{ opacity: 1 - i * 0.12 }}
                        />
                      ))}
                    </div>
                  )
                  : conversationItems.length > 0
                  ? (
                    <div className="conv-list">
                      {(function () {
                        const grouped = new Map<string, ConvItem[]>();
                        conversationItems.forEach((item) => {
                          const key = item.group ?? "__nogroup__";
                          if (!grouped.has(key)) grouped.set(key, []);
                          grouped.get(key)!.push(item);
                        });

                        return Array.from(grouped.entries()).map(([group, items]) => {
                          const isWsGroup = group.startsWith("ws:");
                          const isExpanded = !isWsGroup || expandedKeys.includes(group);

                          return (
                            <div key={group}>
                              {group !== "__nogroup__" && (
                                <div
                                  className="conv-group-header"
                                  onClick={() => {
                                    if (isWsGroup) {
                                      handleGroupExpand(
                                        isExpanded
                                          ? expandedKeys.filter((k) => k !== group)
                                          : [...expandedKeys, group],
                                      );
                                    }
                                  }}
                                >
                                  {isWsGroup && (
                                    <span className={\`conv-group-chevron\${isExpanded ? " expanded" : ""}\`}>
                                      <ChevronRight size={12} />
                                    </span>
                                  )}
                                  {renderGroupLabel(group)}
                                </div>
                              )}
                              {isExpanded && items.map((item) => (
                                <div
                                  key={item.key}
                                  className={\`conv-item\${activeConversationId === item.key ? " active" : ""}\`}
                                  data-conv-id={item["data-conv-id"]}
                                  style={item.style}
                                  onClick={() => handleConversationClick(item.key)}
                                  onKeyDown={(e) => {
                                    if (e.key === "Enter" || e.key === " ") {
                                      e.preventDefault();
                                      handleConversationClick(item.key);
                                    }
                                  }}
                                  role="button"
                                  tabIndex={0}
                                >
                                  <span className="conv-item-icon">{item.icon}</span>
                                  <span className="conv-item-label">{item.label}</span>
                                  {!multiSelectMode && (
                                    <DropdownMenu items={buildMenuItems(item.key)}>
                                      <button
                                        className="conv-item-menu-btn"
                                        onClick={(e) => e.stopPropagation()}
                                        aria-label="Menu"
                                      >
                                        <Pencil size={12} />
                                      </button>
                                    </DropdownMenu>
                                  )}
                                </div>
                              ))}
                            </div>
                          );
                        });
                      })()}
                    </div>
                  )
                  : (
                    <div className="conv-empty">
                      <Empty
                        description={t("chat.noConversations")}
                        image={Empty.PRESENTED_IMAGE_SIMPLE}
                      />
                    </div>
                  )}
              </div>
            </div>

            {/* Context menu overlay */}
            {contextMenuState && (
              <div
                className="conv-context-menu"
                style={{ left: contextMenuState.x, top: contextMenuState.y }}
                role="menu"
                onClick={() => setContextMenuState(null)}
              >
                {buildMenuItems(contextMenuState.convId).map((child) =>
                  child.divider
                    ? <div key={child.key} className="dropdown-divider" />
                    : child.children
                    ? (
                      <div key={child.key} className="dropdown-submenu">
                        <div className="dropdown-group-label">{child.label}</div>
                        {child.children.map((gchild) => (
                          <button
                            key={gchild.key}
                            className={\`dropdown-item\${gchild.danger ? " dropdown-item-danger" : ""}\${
                              gchild.disabled ? " dropdown-item-disabled" : ""
                            }\`}
                            disabled={gchild.disabled}
                            onClick={() => {
                              gchild.onClick?.();
                              setContextMenuState(null);
                            }}
                          >
                            {gchild.icon && <span className="dropdown-item-icon">{gchild.icon}</span>}
                            <span className="dropdown-item-label">{gchild.label}</span>
                          </button>
                        ))}
                      </div>
                    )
                    : (
                      <button
                        key={child.key}
                        className={\`dropdown-item\${child.danger ? " dropdown-item-danger" : ""}\${
                          child.disabled ? " dropdown-item-disabled" : ""
                        }\`}
                        disabled={child.disabled}
                        onClick={() => {
                          child.onClick?.();
                          setContextMenuState(null);
                        }}
                      >
                        {child.icon && <span className="dropdown-item-icon">{child.icon}</span>}
                        <span className="dropdown-item-label">{child.label}</span>
                      </button>
                    )
                )}
              </div>
            )}
          </div>`;`;

// Find closing div of the Conversations block
// The pattern is: </div></div></Dropdown>
// We need to find the matching closing tags
// Let's find "            </div>" after the Conversations closing tag
const convCloseParen = f.indexOf("\n            </div>\n          </div>\n        </Dropdown>\n      )", dropdownStart);
if (convCloseParen !== -1) {
  f = f.slice(0, convCloseParen) + "\n" + nativeList + "\n" + f.slice(convCloseParen);
} else {
  // Try alternate pattern with just 2 closing divs
  const altEnd = f.indexOf("\n            </div>\n          </Dropdown>\n        )", dropdownStart);
  if (altEnd !== -1) {
    f = f.slice(0, altEnd) + "\n" + nativeList + "\n" + f.slice(altEnd);
  } else {
    console.error("Cannot find closing tags for Dropdown block");
    process.exit(1);
  }
}

writeFileSync("src/components/chat/ChatSidebar.tsx", f, "utf-8");
console.log("Done — ChatSidebar.tsx rewritten");
