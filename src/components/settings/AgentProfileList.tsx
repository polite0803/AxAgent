/**
 * 股票分析 AgentProfile 列表编辑器。
 * 编辑模板节点 exposed_tools（暴露给 LLM 的工具），持久化到 workflow_templates 表。
 *
 * 工具分两种：
 *   固定工具 (⚙️) — DAG 中 WorkflowNode::Tool 节点确定性执行，结果注入 context_sources
 *   暴露工具 (🤖) — 模板节点 exposed_tools，LLM 自主决定调用
 */
import { invoke } from "@/lib/invoke";
import { App, Button, Input, Popover, Select, Space, Spin, Tag, Tooltip } from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

interface AgentNodeRow {
  id: string;
  profileId: string;
  expertId: string;
  expertName: string;
  roleId: string;
  roleName: string;
  tools: string[];
  fixedTools: string[];
  systemPrompt: string;
  temperature: number;
  maxTokens: number;
  maxToolRounds: number;
}

/** 固定 ToolNode → agent 节点的工具映射 */
const FIXED_TOOL_MAP: Record<string, string[]> = {
  "a-market-analyst": ["get_stock_kline"],
  "a-sentiment": ["get_hot_stocks"],
  "a-news": ["get_announcements"],
  "a-fundamentals": ["get_consensus_eps"],
  "a-policy": ["get_announcements"],
  "a-hot-money": ["get_stock_money_flow"],
  "a-lockup": ["get_announcements"],
  "a-research": ["get_consensus_eps"],
  "a-sector": ["get_industry_ranking"],
};

const FIXED_ALGO_TOOLS: Record<string, string[]> = {
  "research-mgr": ["compute_scoring", "compute_valuation", "compute_portfolio_risk"],
};

const PROFILE_NAMES: Record<string, string> = {
  "stock-market-analyst": "市场技术分析师",
  "stock-sentiment-analyst": "情绪面分析师",
  "stock-news-analyst": "消息面分析师",
  "stock-fundamentals-analyst": "基本面分析师",
  "stock-policy-analyst": "政策面分析师",
  "stock-hot-money-tracker": "资金面追踪",
  "stock-lockup-watcher": "筹码限售观察",
  "stock-research-analyst": "研报分析师",
  "stock-sector-analyst": "板块题材分析师",
  "stock-bull-researcher": "多方研究员",
  "stock-bear-researcher": "空方研究员",
  "stock-aggressive-debator": "激进风险评估",
  "stock-conservative-debator": "保守风险评估",
  "stock-neutral-debator": "中性风险评估",
  "stock-research-manager": "研究经理",
  "stock-trader": "交易员",
  "stock-portfolio-manager": "投资组合经理",
};

const PROFILE_ROLES: Record<string, string> = {
  "stock-market-analyst": "股票分析师",
  "stock-sentiment-analyst": "股票分析师",
  "stock-news-analyst": "股票分析师",
  "stock-fundamentals-analyst": "股票分析师",
  "stock-policy-analyst": "股票分析师",
  "stock-hot-money-tracker": "股票分析师",
  "stock-lockup-watcher": "股票分析师",
  "stock-research-analyst": "股票分析师",
  "stock-sector-analyst": "股票分析师",
  "stock-bull-researcher": "辩论研究员",
  "stock-bear-researcher": "辩论研究员",
  "stock-aggressive-debator": "风险评估师",
  "stock-conservative-debator": "风险评估师",
  "stock-neutral-debator": "风险评估师",
  "stock-research-manager": "决策者",
  "stock-trader": "交易员",
  "stock-portfolio-manager": "决策者",
};

const PROFILE_ROLE_IDS: Record<string, string> = {
  "stock-market-analyst": "stock-analyst",
  "stock-sentiment-analyst": "stock-analyst",
  "stock-news-analyst": "stock-analyst",
  "stock-fundamentals-analyst": "stock-analyst",
  "stock-policy-analyst": "stock-analyst",
  "stock-hot-money-tracker": "stock-analyst",
  "stock-lockup-watcher": "stock-analyst",
  "stock-research-analyst": "stock-analyst",
  "stock-sector-analyst": "stock-analyst",
  "stock-bull-researcher": "debater",
  "stock-bear-researcher": "debater",
  "stock-aggressive-debator": "risk-evaluator",
  "stock-conservative-debator": "risk-evaluator",
  "stock-neutral-debator": "risk-evaluator",
  "stock-research-manager": "decision-maker",
  "stock-trader": "trader",
  "stock-portfolio-manager": "decision-maker",
};

export function AgentProfileList() {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const [rows, setRows] = useState<AgentNodeRow[]>([]);
  const [allTools, setAllTools] = useState<string[]>([]);
  const [expertMap, setExpertMap] = useState<Record<string, { name: string; prompt: string }>>({});
  const [roleMap, setRoleMap] = useState<Record<string, { name: string; prompt: string }>>({});
  const [loading, setLoading] = useState(true);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [saving, setSaving] = useState<string | null>(null);
  const [editPrompt, setEditPrompt] = useState<{ type: "expert" | "role"; id: string; text: string } | null>(null);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [editRow, setEditRow] = useState<AgentNodeRow | null>(null);

  const toolOptions = useMemo(
    () => allTools.map((t) => ({ label: t, value: t })),
    [allTools],
  );

  const loadAll = useCallback(async () => {
    setLoading(true);
    try {
      const [template, tools, expertsRaw, rolesRaw] = await Promise.all([
        invoke<{ nodes: unknown[] }>("get_workflow_template", { id: "stock-analysis" }),
        invoke<string[]>("list_stock_tools"),
        invoke<{ id: string; name: string; system_prompt: string }[]>("list_agency_experts"),
        invoke<{ id: string; name: string; system_prompt: string }[]>("list_agent_roles", { source: "stock-analysis" }),
      ]);
      setAllTools(Array.isArray(tools) ? tools : []);
      const em: Record<string, { name: string; prompt: string }> = {};
      for (const e of (Array.isArray(expertsRaw) ? expertsRaw : [])) {
        em[e.id] = { name: e.name, prompt: e.system_prompt };
      }
      setExpertMap(em);
      const rm: Record<string, { name: string; prompt: string }> = {};
      for (const r of (Array.isArray(rolesRaw) ? rolesRaw : [])) {
        rm[r.id] = { name: r.name, prompt: r.system_prompt };
      }
      setRoleMap(rm);

      if (template?.nodes) {
        const nodes = Array.isArray(template.nodes) ? template.nodes : [];
        const parsed = nodes
          .map((n) => {
            const node = n as Record<string, unknown>;
            const config = node?.config as Record<string, unknown> | undefined;
            const base = node?.base as Record<string, unknown> | undefined;
            const pid: string = (config?.agent_profile_id as string) ?? "";
            if (!pid.startsWith("stock-")) { return null; }
            const exposed: string[] = (config?.exposed_tools as string[]) ?? [];
            const nodeId: string = (base?.id as string) ?? (node?.id as string) ?? "";
            return {
              id: nodeId,
              profileId: pid,
              expertId: `agency-${pid}`,
              expertName: PROFILE_NAMES[pid] ?? pid,
              roleId: PROFILE_ROLE_IDS[pid] ?? "",
              roleName: PROFILE_ROLES[pid] ?? "-",
              tools: exposed,
              fixedTools: FIXED_TOOL_MAP[nodeId] ?? FIXED_ALGO_TOOLS[nodeId] ?? [],
              systemPrompt: config?.system_prompt as string ?? "",
              temperature: config?.temperature as number ?? 0.3,
              maxTokens: config?.max_tokens as number ?? 4096,
              maxToolRounds: config?.max_tool_rounds as number ?? 2,
            } as AgentNodeRow;
          })
          .filter(Boolean) as AgentNodeRow[];
        if (parsed.length > 0) {
          setRows(parsed);
          return;
        }
      }
      // 无模板数据时用静态映射回退（预览/离线模式）
      const profileToNode: Record<string, string> = {
        "stock-market-analyst": "a-market-analyst",
        "stock-sentiment-analyst": "a-sentiment",
        "stock-news-analyst": "a-news",
        "stock-fundamentals-analyst": "a-fundamentals",
        "stock-policy-analyst": "a-policy",
        "stock-hot-money-tracker": "a-hot-money",
        "stock-lockup-watcher": "a-lockup",
        "stock-research-analyst": "a-research",
        "stock-sector-analyst": "a-sector",
        "stock-research-manager": "research-mgr",
      };
      const fallbackRows: AgentNodeRow[] = Object.keys(PROFILE_NAMES).map((pid) => {
        const nid = profileToNode[pid] ?? pid.replace("stock-", "");
        return {
          id: nid,
          profileId: pid,
          expertId: `agency-${pid}`,
          expertName: PROFILE_NAMES[pid],
          roleId: PROFILE_ROLE_IDS[pid] ?? "",
          roleName: PROFILE_ROLES[pid] ?? "-",
          tools: [],
          fixedTools: FIXED_TOOL_MAP[nid] ?? FIXED_ALGO_TOOLS[nid] ?? [],
          systemPrompt: "",
          temperature: 0.3,
          maxTokens: 4096,
          maxToolRounds: 2,
        };
      });
      setRows(fallbackRows);
    } catch (err) {
      console.error("加载模板节点失败", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      setLoading(true);
      return Promise.all([
        invoke<{ nodes: unknown[] }>("get_workflow_template", { id: "stock-analysis" }),
        invoke<string[]>("list_stock_tools"),
        invoke<{ id: string; name: string; system_prompt: string }[]>("list_agency_experts"),
        invoke<{ id: string; name: string; system_prompt: string }[]>("list_agent_roles", { source: "stock-analysis" }),
      ]);
    })
      .then((result) => {
        if (!result || cancelled) { return; }
        const [template, tools, expertsRaw, rolesRaw] = result;
        setAllTools(Array.isArray(tools) ? tools : []);
        const em: Record<string, { name: string; prompt: string }> = {};
        for (const e of (Array.isArray(expertsRaw) ? expertsRaw : [])) {
          em[e.id] = { name: e.name, prompt: e.system_prompt };
        }
        setExpertMap(em);
        const rm: Record<string, { name: string; prompt: string }> = {};
        for (const r of (Array.isArray(rolesRaw) ? rolesRaw : [])) {
          rm[r.id] = { name: r.name, prompt: r.system_prompt };
        }
        setRoleMap(rm);

        if (template?.nodes) {
          const nodes = Array.isArray(template.nodes) ? template.nodes : [];
          const parsed = nodes
            .map((rawN) => {
              const n = rawN as Record<string, unknown>;
              const config = n?.config as Record<string, unknown> | undefined;
              const base = n?.base as Record<string, unknown> | undefined;
              const pid: string = (config?.agent_profile_id as string) ?? "";
              if (!pid.startsWith("stock-")) { return null; }
              const exposed: string[] = (config?.exposed_tools as string[]) ?? [];
              const nodeId: string = (base?.id as string) ?? (n?.id as string) ?? "";
              return {
                id: nodeId,
                profileId: pid,
                expertId: `agency-${pid}`,
                expertName: PROFILE_NAMES[pid] ?? pid,
                roleId: PROFILE_ROLE_IDS[pid] ?? "",
                roleName: PROFILE_ROLES[pid] ?? "-",
                tools: exposed,
                fixedTools: FIXED_TOOL_MAP[nodeId] ?? FIXED_ALGO_TOOLS[nodeId] ?? [],
                systemPrompt: config?.system_prompt as string ?? "",
                temperature: config?.temperature as number ?? 0.3,
                maxTokens: config?.max_tokens as number ?? 4096,
                maxToolRounds: config?.max_tool_rounds as number ?? 2,
              } as AgentNodeRow;
            })
            .filter(Boolean) as AgentNodeRow[];
          if (parsed.length > 0) {
            setRows(parsed);
            if (!cancelled) { setLoading(false); }
            return;
          }
        }
        // 无模板数据时用静态映射回退（预览/离线模式）
        const profileToNode: Record<string, string> = {
          "stock-market-analyst": "a-market-analyst",
          "stock-sentiment-analyst": "a-sentiment",
          "stock-news-analyst": "a-news",
          "stock-fundamentals-analyst": "a-fundamentals",
          "stock-policy-analyst": "a-policy",
          "stock-hot-money-tracker": "a-hot-money",
          "stock-lockup-watcher": "a-lockup",
          "stock-research-analyst": "a-research",
          "stock-sector-analyst": "a-sector",
          "stock-research-manager": "research-mgr",
        };
        const fallbackRows: AgentNodeRow[] = Object.keys(PROFILE_NAMES).map((pid) => {
          const nid = profileToNode[pid] ?? pid.replace("stock-", "");
          return {
            id: nid,
            profileId: pid,
            expertId: `agency-${pid}`,
            expertName: PROFILE_NAMES[pid],
            roleId: PROFILE_ROLE_IDS[pid] ?? "",
            roleName: PROFILE_ROLES[pid] ?? "-",
            tools: [],
            fixedTools: FIXED_TOOL_MAP[nid] ?? FIXED_ALGO_TOOLS[nid] ?? [],
            systemPrompt: "",
            temperature: 0.3,
            maxTokens: 4096,
            maxToolRounds: 2,
          };
        });
        setRows(fallbackRows);
      })
      .catch((err) => {
        console.error("加载模板节点失败", err);
      })
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const handleSavePrompt = useCallback(async () => {
    if (!editPrompt) { return; }
    setSaving(`prompt-${editPrompt.id}`);
    try {
      if (editPrompt.type === "expert") {
        await invoke("update_agency_expert", { request: { id: editPrompt.id, systemPrompt: editPrompt.text } });
        setExpertMap((prev) => ({ ...prev, [editPrompt.id]: { ...prev[editPrompt.id], prompt: editPrompt.text } }));
      } else {
        await invoke("update_agent_role", { id: editPrompt.id, systemPrompt: editPrompt.text });
        setRoleMap((prev) => ({ ...prev, [editPrompt.id]: { ...prev[editPrompt.id], prompt: editPrompt.text } }));
      }
      message.success(t("stockAnalysis.settings.profile.saved"));
      setEditPrompt(null);
    } catch (err) {
      message.error(t("stockAnalysis.settings.profile.saveError", { error: String(err) }));
    } finally {
      setSaving(null);
    }
  }, [editPrompt, t, message]);

  const handleSaveNodeConfig = useCallback(async () => {
    if (!editRow) { return; }
    setSaving(`node-${editRow.id}`);
    try {
      await invoke("update_workflow_template_node", {
        templateId: "stock-analysis",
        nodeId: editRow.id,
        input: {
          systemPrompt: editRow.systemPrompt || undefined,
          temperature: editRow.temperature,
          maxTokens: editRow.maxTokens,
          maxToolRounds: editRow.maxToolRounds,
        },
      });
      message.success(t("stockAnalysis.settings.profile.saved"));
      setExpandedId(null);
      setEditRow(null);
      loadAll();
    } catch (err) {
      message.error(t("stockAnalysis.settings.profile.saveError", { error: String(err) }));
    } finally {
      setSaving(null);
    }
  }, [editRow, t, loadAll, message]);

  const handleSave = useCallback(
    async (nodeId: string, tools: string[]) => {
      setSaving(nodeId);
      try {
        await invoke("update_workflow_template_node", {
          templateId: "stock-analysis",
          nodeId,
          input: { exposedTools: tools },
        });
        message.success(t("stockAnalysis.settings.profile.saved"));
        setEditingId(null);
        loadAll();
      } catch (err) {
        message.error(t("stockAnalysis.settings.profile.saveError", { error: String(err) }));
      } finally {
        setSaving(null);
      }
    },
    [t, loadAll, message],
  );

  if (loading) {
    return (
      <div className="flex justify-center py-12">
        <Spin />
      </div>
    );
  }

  return (
    <div>
      {/* 卡片列表 */}
      <div className="flex flex-col gap-2">
        {rows.map((row) => {
          const isEditing = editingId === row.id;
          const fixedSet = new Set(row.fixedTools);
          // 分类：纯固定 / 重叠 / 纯暴露
          const both = row.tools.filter((t) => fixedSet.has(t));
          const exposed = row.tools.filter((t) => !fixedSet.has(t));

          return (
            <div
              key={row.id}
              className="rounded-lg border border-gray-200 p-3 transition-colors hover:border-blue-300"
            >
              {/* 标题行 */}
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2 min-w-0">
                  <Popover
                    trigger="click"
                    title={
                      <Space>
                        <span>{expertMap[row.expertId]?.name ?? row.expertName}</span>
                        <Button
                          size="small"
                          type="link"
                          onClick={(e) => {
                            e.stopPropagation();
                            const p = expertMap[row.expertId]?.prompt ?? "";
                            setEditPrompt(
                              editPrompt?.id === row.expertId ? null : { type: "expert", id: row.expertId, text: p },
                            );
                          }}
                        >
                          {editPrompt?.id === row.expertId
                            ? t("stockAnalysis.settings.profile.cancel")
                            : t("stockAnalysis.settings.profile.edit")}
                        </Button>
                      </Space>
                    }
                    content={editPrompt?.id === row.expertId && editPrompt.type === "expert"
                      ? (
                        <div className="flex flex-col gap-2" style={{ width: 420 }}>
                          <Input.TextArea
                            autoSize={{ minRows: 4, maxRows: 16 }}
                            value={editPrompt.text}
                            onChange={(e) => setEditPrompt({ ...editPrompt, text: e.target.value })}
                          />
                          <Button
                            size="small"
                            type="primary"
                            loading={saving === `prompt-${row.expertId}`}
                            onClick={handleSavePrompt}
                          >
                            {t("stockAnalysis.settings.profile.save")}
                          </Button>
                        </div>
                      )
                      : (
                        <div className="max-h-60 max-w-96 overflow-auto whitespace-pre-wrap text-xs leading-relaxed">
                          {expertMap[row.expertId]?.prompt ?? t("stockAnalysis.settings.profile.noPrompt")}
                        </div>
                      )}
                  >
                    <span className="font-medium text-sm truncate cursor-pointer hover:text-blue-500 transition-colors">
                      {row.expertName}
                    </span>
                  </Popover>
                  <Popover
                    trigger="click"
                    title={
                      <Space>
                        <span>{roleMap[row.roleId]?.name ?? row.roleName}</span>
                        <Button
                          size="small"
                          type="link"
                          onClick={(e) => {
                            e.stopPropagation();
                            const p = roleMap[row.roleId]?.prompt ?? "";
                            setEditPrompt(
                              editPrompt?.id === row.roleId ? null : { type: "role", id: row.roleId, text: p },
                            );
                          }}
                        >
                          {editPrompt?.id === row.roleId
                            ? t("stockAnalysis.settings.profile.cancel")
                            : t("stockAnalysis.settings.profile.edit")}
                        </Button>
                      </Space>
                    }
                    content={editPrompt?.id === row.roleId && editPrompt.type === "role"
                      ? (
                        <div className="flex flex-col gap-2" style={{ width: 420 }}>
                          <Input.TextArea
                            autoSize={{ minRows: 4, maxRows: 16 }}
                            value={editPrompt.text}
                            onChange={(e) => setEditPrompt({ ...editPrompt, text: e.target.value })}
                          />
                          <Button
                            size="small"
                            type="primary"
                            loading={saving === `prompt-${row.roleId}`}
                            onClick={handleSavePrompt}
                          >
                            {t("stockAnalysis.settings.profile.save")}
                          </Button>
                        </div>
                      )
                      : (
                        <div className="max-h-60 max-w-96 overflow-auto whitespace-pre-wrap text-xs leading-relaxed">
                          {roleMap[row.roleId]?.prompt ?? t("stockAnalysis.settings.profile.noPrompt")}
                        </div>
                      )}
                  >
                    <Tag color="blue" className="text-xs m-0 shrink-0 cursor-pointer">{row.roleName}</Tag>
                  </Popover>
                </div>
                <Space size={4} className="shrink-0">
                  <Button
                    size="small"
                    type={expandedId === row.id ? "primary" : "text"}
                    onClick={() => {
                      if (expandedId === row.id) {
                        setExpandedId(null);
                        setEditRow(null);
                      } else {
                        setExpandedId(row.id);
                        setEditRow({ ...row });
                      }
                    }}
                  >
                    {expandedId === row.id ? t("common.collapse") : t("stockAnalysis.settings.profile.advanced")}
                  </Button>
                  {!isEditing && (
                    <Button size="small" type="dashed" onClick={() => setEditingId(row.id)}>
                      {t("stockAnalysis.settings.profile.editTools")}
                    </Button>
                  )}
                </Space>
              </div>

              {/* 工具行 */}
              {isEditing
                ? (
                  <div className="flex flex-col gap-2">
                    {row.fixedTools.length > 0 && (
                      <div className="text-xs text-gray-400">
                        {t("stockAnalysis.settings.profile.fixedHint", { tools: row.fixedTools.join(", ") })}
                      </div>
                    )}
                    <Select
                      mode="multiple"
                      style={{ width: "100%" }}
                      value={row.tools}
                      options={toolOptions}
                      onChange={(vals) => {
                        setRows((prev) => prev.map((r) => (r.id === row.id ? { ...r, tools: vals } : r)));
                      }}
                      placeholder={t("stockAnalysis.settings.profile.selectTools")}
                      maxTagCount="responsive"
                    />
                    <Space>
                      <Button
                        size="small"
                        type="primary"
                        loading={saving === row.id}
                        onClick={() => handleSave(row.id, row.tools)}
                      >
                        {t("stockAnalysis.settings.profile.save")}
                      </Button>
                      <Button
                        size="small"
                        onClick={() => {
                          loadAll();
                          setEditingId(null);
                        }}
                      >
                        {t("stockAnalysis.settings.profile.cancel")}
                      </Button>
                    </Space>
                  </div>
                )
                : (
                  <div className="flex flex-wrap items-start gap-x-3 gap-y-1">
                    {/* 固定工具 */}
                    {row.fixedTools.length > 0 && (
                      <span className="inline-flex flex-wrap items-center gap-1">
                        {row.fixedTools.map((tn) => (
                          <Tooltip
                            key={tn}
                            title={both.includes(tn) ? t("stockAnalysis.settings.profile.bothHint") : undefined}
                          >
                            <Tag color={both.includes(tn) ? "cyan" : "default"} className="text-xs m-0">
                              ⚙️ {tn}
                            </Tag>
                          </Tooltip>
                        ))}
                      </span>
                    )}
                    {/* 纯暴露工具 */}
                    {exposed.length > 0 && (
                      <span className="inline-flex flex-wrap items-center gap-1">
                        {exposed.map((tn) => <Tag key={tn} color="green" className="text-xs m-0">🤖 {tn}</Tag>)}
                      </span>
                    )}
                    {row.fixedTools.length === 0 && exposed.length === 0 && (
                      <span className="text-gray-400 text-xs italic">
                        {t("stockAnalysis.settings.profile.noTools")}
                      </span>
                    )}
                  </div>
                )}

              {/* LLM 配置展开区 */}
              {expandedId === row.id && editRow && (
                <div className="mt-3 pt-3 border-t border-gray-100">
                  <div className="text-xs text-gray-400 mb-2">{t("stockAnalysis.settings.profile.promptSection")}</div>
                  <div className="flex flex-col gap-2">
                    <Input.TextArea
                      size="small"
                      autoSize={{ minRows: 2, maxRows: 6 }}
                      placeholder={t("stockAnalysis.settings.profile.promptPlaceholder")}
                      value={editRow.systemPrompt}
                      onChange={(e) => setEditRow({ ...editRow, systemPrompt: e.target.value })}
                    />
                    <div className="flex flex-wrap gap-3">
                      <span className="flex items-center gap-1 text-xs">
                        <span className="text-gray-400">{t("stockAnalysis.settings.profile.temperature")}</span>
                        <Input
                          size="small"
                          style={{ width: 70 }}
                          value={editRow.temperature}
                          onChange={(e) => setEditRow({ ...editRow, temperature: Number(e.target.value) || 0 })}
                        />
                      </span>
                      <span className="flex items-center gap-1 text-xs">
                        <span className="text-gray-400">Max Tokens</span>
                        <Input
                          size="small"
                          style={{ width: 80 }}
                          value={editRow.maxTokens}
                          onChange={(e) => setEditRow({ ...editRow, maxTokens: Number(e.target.value) || 0 })}
                        />
                      </span>
                      <span className="flex items-center gap-1 text-xs">
                        <span className="text-gray-400">{t("stockAnalysis.settings.profile.maxToolRounds")}</span>
                        <Input
                          size="small"
                          style={{ width: 60 }}
                          value={editRow.maxToolRounds}
                          onChange={(e) => setEditRow({ ...editRow, maxToolRounds: Number(e.target.value) || 0 })}
                        />
                      </span>
                    </div>
                    <div>
                      <Button
                        size="small"
                        type="primary"
                        loading={saving === `node-${row.id}`}
                        onClick={handleSaveNodeConfig}
                      >
                        {t("stockAnalysis.settings.profile.save")}
                      </Button>
                    </div>
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
