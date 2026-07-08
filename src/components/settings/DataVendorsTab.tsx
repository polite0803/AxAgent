/**
 * 数据源 Tab — Vendor 开关 + 健康检测 + 固定工具依赖融合展示。
 */
import { invoke } from "@/lib/invoke";
import { App, Button, Card, Input, Select, Space, Spin, Switch, Tag } from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

interface VendorDef {
  key: string;
  enName: string;
  name: string;
  desc: string;
  capabilities: string[];
  requiresKey: boolean;
  helpUrl?: string;
  helpText?: string;
}

const VENDORS: VendorDef[] = [
  {
    key: "vendor_tencent",
    enName: "tencent",
    name: "腾讯财经",
    desc: "报价/K线",
    capabilities: ["quote", "klines"],
    requiresKey: false,
  },
  {
    key: "vendor_eastmoney",
    enName: "eastmoney",
    name: "东方财富",
    desc: "综合数据",
    capabilities: [
      "quote",
      "klines",
      "financials",
      "money_flow",
      "dragon_tiger",
      "lockup",
      "search",
      "margin",
      "north_bound",
      "sector",
      "shareholder_trades",
      "dividend",
      "research_reports",
      "market_dragon_tiger",
      "cls_flash",
      "announcements",
      "block_trades",
      "institutional_visits",
      "index_quotes",
      "peers",
      "option_pcr",
    ],
    requiresKey: false,
  },
  {
    key: "vendor_sina",
    enName: "sina",
    name: "新浪财经",
    desc: "报价/新闻",
    capabilities: ["quote", "news"],
    requiresKey: false,
  },
  {
    key: "vendor_ths",
    enName: "ths",
    name: "同花顺",
    desc: "综合数据",
    capabilities: ["consensus_eps", "concept_blocks", "hot_stocks", "industry_ranking", "north_bound_flow"],
    requiresKey: false,
  },
  {
    key: "vendor_cninfo",
    enName: "cninfo",
    name: "巨潮资讯",
    desc: "信息披露",
    capabilities: ["announcements"],
    requiresKey: false,
  },
  {
    key: "vendor_baidu_stock",
    enName: "baidu_stock",
    name: "百度股市通",
    desc: "全维度",
    capabilities: [
      "quote",
      "klines",
      "financials",
      "news",
      "money_flow",
      "dragon_tiger",
      "lockup",
      "search",
      "margin",
      "north_bound",
      "sector",
      "shareholder_trades",
      "dividend",
      "research_reports",
      "concept_blocks",
      "hot_stocks",
      "industry_ranking",
      "north_bound_flow",
    ],
    requiresKey: false,
  },
  {
    key: "vendor_iwencai",
    enName: "iwencai",
    name: "问财",
    desc: "选股/研报",
    capabilities: ["search", "sector", "consensus_eps", "concept_blocks", "hot_stocks"],
    requiresKey: true,
    helpUrl: "https://www.iwencai.com/",
    helpText: "注册登录后获取 API token",
  },
  {
    key: "vendor_akshare",
    enName: "akshare",
    name: "AKShare",
    desc: "开源数据",
    capabilities: ["financials", "news", "consensus_eps", "cls_flash"],
    requiresKey: false,
  },
  {
    key: "vendor_mootdx",
    enName: "mootdx",
    name: "Mootdx",
    desc: "通达信本地",
    capabilities: ["quote", "klines"],
    requiresKey: false,
    helpText: "需本地安装通达信客户端，启动后 Mootdx 自动通过 127.0.0.1:7709 连接行情服务",
  },
  {
    key: "vendor_xueqiu",
    enName: "xueqiu",
    name: "雪球",
    desc: "新闻/财务",
    capabilities: ["news", "financials", "quote", "klines"],
    requiresKey: true,
    helpUrl: "https://xueqiu.com/",
    helpText: "登录雪球后 F12 → Cookie 中取 xq_a_token",
  },
  {
    key: "vendor_neodata",
    enName: "neodata",
    name: "NeoData",
    desc: "美股/宏观/外汇",
    capabilities: [
      "quote",
      "financials",
      "news",
      "search",
      "sector",
      "cls_flash",
      "industry_ranking",
      "index_quotes",
      "hot_stocks",
    ],
    requiresKey: true,
    helpText: "点击「通过 WorkBuddy 连接」按钮，然后在 WorkBuddy 中输入「刷新 NeoData token」即可自动获取",
  },
];

/** 工具 → Vendor 路由映射（固定 + 暴露） */
interface ToolRoute {
  tool: string;
  label: string;
  kind: "fixed" | "exposed"; // 固定 ToolNode / LLM 暴露工具
  vendors: string[];
}

/** 固定 ToolNode（DAG 确定性执行）*/
const FIXED_TOOLS: ToolRoute[] = [
  { tool: "get_stock_kline", label: "K线", kind: "fixed", vendors: ["eastmoney", "tencent", "sina", "mootdx"] },
  { tool: "get_hot_stocks", label: "热门股", kind: "fixed", vendors: ["ths", "baidu_stock", "iwencai"] },
  { tool: "get_announcements", label: "公告", kind: "fixed", vendors: ["cninfo", "eastmoney"] },
  { tool: "get_consensus_eps", label: "一致预期", kind: "fixed", vendors: ["ths", "akshare", "iwencai"] },
  { tool: "get_stock_money_flow", label: "资金流向", kind: "fixed", vendors: ["eastmoney", "baidu_stock"] },
  { tool: "get_industry_ranking", label: "行业排名", kind: "fixed", vendors: ["ths", "baidu_stock"] },
];

/** LLM 暴露工具（Agent 自主调用，均走 VendorRouting 降级链）*/
const EXPOSED_TOOLS: ToolRoute[] = [
  { tool: "get_stock_quote", label: "实时行情", kind: "exposed", vendors: ["tencent", "mootdx", "eastmoney"] },
  {
    tool: "get_stock_news",
    label: "新闻",
    kind: "exposed",
    vendors: ["xueqiu", "sina", "eastmoney", "baidu_stock", "akshare"],
  },
  {
    tool: "get_stock_financials",
    label: "财务",
    kind: "exposed",
    vendors: ["eastmoney", "xueqiu", "baidu_stock", "akshare", "sina"],
  },
  { tool: "search_stock", label: "搜索", kind: "exposed", vendors: ["eastmoney", "iwencai", "baidu_stock"] },
  { tool: "get_research_reports", label: "研报", kind: "exposed", vendors: ["eastmoney", "baidu_stock"] },
  { tool: "get_concept_blocks", label: "概念板块", kind: "exposed", vendors: ["ths", "baidu_stock", "iwencai"] },
  { tool: "get_market_dragon_tiger", label: "龙虎榜", kind: "exposed", vendors: ["eastmoney", "baidu_stock"] },
  { tool: "get_cls_flash", label: "快讯", kind: "exposed", vendors: ["eastmoney", "akshare"] },
  { tool: "get_north_bound_flow", label: "北向资金", kind: "exposed", vendors: ["ths", "baidu_stock"] },
  { tool: "get_block_trades", label: "大宗交易", kind: "exposed", vendors: ["eastmoney", "baidu_stock"] },
  { tool: "get_institutional_visits", label: "机构调研", kind: "exposed", vendors: ["eastmoney"] },
  { tool: "get_index_quotes", label: "大盘指数", kind: "exposed", vendors: ["eastmoney"] },
  { tool: "get_stock_peers", label: "同行对比", kind: "exposed", vendors: ["eastmoney"] },
  { tool: "get_stock_option_pcr", label: "期权PCR", kind: "exposed", vendors: ["eastmoney"] },
  {
    tool: "get_stock_quote",
    label: "实时行情",
    kind: "exposed",
    vendors: ["tencent", "mootdx", "sina", "xueqiu", "eastmoney"],
  },
];

const ALL_TOOLS = [...FIXED_TOOLS, ...EXPOSED_TOOLS];

/** 本地计算工具 */
const LOCAL_TOOLS = [
  { tool: "compute_scoring", label: "技术评分" },
  { tool: "compute_valuation", label: "估值计算" },
  { tool: "compute_portfolio_risk", label: "组合风险" },
];

const CAP_LABELS: Record<string, string> = {
  quote: "行情",
  klines: "K线",
  financials: "财务",
  news: "新闻",
  money_flow: "资金流向",
  dragon_tiger: "龙虎榜",
  lockup: "解禁",
  search: "搜索",
  margin: "融资融券",
  north_bound: "北向持仓",
  sector: "行业板块",
  shareholder_trades: "增减持",
  dividend: "分红",
  research_reports: "研报",
  consensus_eps: "一致预期",
  concept_blocks: "概念板块",
  announcements: "公告",
  market_dragon_tiger: "大盘龙虎",
  hot_stocks: "热门股",
  industry_ranking: "行业排名",
  cls_flash: "快讯",
  north_bound_flow: "北向资金",
  block_trades: "大宗交易",
  institutional_visits: "机构调研",
  index_quotes: "大盘指数",
  peers: "同行对比",
  option_pcr: "期权PCR",
};

type HealthStatus = "ok" | "fail" | "pending" | "idle";

export function DataVendorsTab() {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const [vendorValues, setVendorValues] = useState<Record<string, boolean>>({});
  const [iwencaiKey, setIwencaiKey] = useState("");
  const [xueqiuToken, setXueqiuToken] = useState("");
  const [neodataToken, setNeodataToken] = useState("");
  const [health, setHealth] = useState<Record<string, HealthStatus>>({});
  const [checkingAll, setCheckingAll] = useState(false);
  const [saving, setSaving] = useState(false);
  const [loaded, setLoaded] = useState(false);

  // vendor 英文名 → 依赖的工具列表（固定 + 暴露）
  const vendorTools = useMemo(() => {
    const map: Record<string, ToolRoute[]> = {};
    for (const tr of ALL_TOOLS) {
      for (const vn of tr.vendors) {
        (map[vn] ??= []).push(tr);
      }
    }
    return map;
  }, []);

  useEffect(() => {
    let cancelled = false;
    invoke<Record<string, unknown>>("get_workflow_template", { id: "stock-analysis" })
      .then((tmpl) => {
        if (cancelled) { return; }
        const vars: { name: string; value: unknown }[] = (tmpl?.variables as { name: string; value: unknown }[]) ?? [];
        const vals: Record<string, boolean> = {};
        let key = "";
        let xqToken = "";
        let ndToken = "";
        for (const v of vars) {
          if (
            v.name.startsWith("vendor_") && v.name !== "vendor_iwencai_key" && v.name !== "vendor_xueqiu_token"
            && v.name !== "vendor_neodata_token"
          ) {
            vals[v.name] = !!v.value;
          }
          if (v.name === "vendor_iwencai_key") { key = typeof v.value === "string" ? v.value : ""; }
          if (v.name === "vendor_xueqiu_token") { xqToken = typeof v.value === "string" ? v.value : ""; }
          if (v.name === "vendor_neodata_token") { ndToken = typeof v.value === "string" ? v.value : ""; }
        }
        setVendorValues(vals);
        setIwencaiKey(key);
        setXueqiuToken(xqToken);
        setNeodataToken(ndToken);
        setLoaded(true);
      })
      .catch(() => {
        if (!cancelled) { setLoaded(true); }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      // 先加载全量模板，只更新 vendor_ 变量值，保留 var_type/is_secret 等字段
      const tmpl = await invoke("get_workflow_template", { id: "stock-analysis" }) as Record<string, unknown>;
      const allVars = (tmpl?.variables as Record<string, unknown>[]) ?? [];
      const varMap = new Map<string, Record<string, unknown>>();
      for (const v of allVars) { varMap.set(v.name as string, v); }
      for (const [k, v] of Object.entries(vendorValues)) {
        const existing = varMap.get(k);
        if (existing && typeof existing === "object" && "name" in existing) {
          varMap.set(k, { ...existing, value: v });
        } else {
          varMap.set(k, { name: k, var_type: "boolean", value: v, is_secret: false });
        }
      }
      const iwencaiExisting = varMap.get("vendor_iwencai_key");
      varMap.set("vendor_iwencai_key", {
        ...(iwencaiExisting && typeof iwencaiExisting === "object" ? iwencaiExisting : {}),
        name: "vendor_iwencai_key",
        var_type: "string",
        value: iwencaiKey,
        is_secret: true,
      });
      const xueqiuExisting = varMap.get("vendor_xueqiu_token");
      varMap.set("vendor_xueqiu_token", {
        ...(xueqiuExisting && typeof xueqiuExisting === "object" ? xueqiuExisting : {}),
        name: "vendor_xueqiu_token",
        var_type: "string",
        value: xueqiuToken,
        is_secret: true,
      });
      const neodataExisting = varMap.get("vendor_neodata_token");
      varMap.set("vendor_neodata_token", {
        ...(neodataExisting && typeof neodataExisting === "object" ? neodataExisting : {}),
        name: "vendor_neodata_token",
        var_type: "string",
        value: neodataToken,
        is_secret: true,
      });
      const merged = Array.from(varMap.values());
      await invoke("update_workflow_template", {
        id: "stock-analysis",
        input: {
          name: tmpl.name,
          icon: tmpl.icon,
          nodes: tmpl.nodes,
          edges: tmpl.edges,
          tags: tmpl.tags,
          trigger_config: tmpl.triggerConfig,
          input_schema: tmpl.inputSchema,
          output_schema: tmpl.outputSchema,
          error_config: tmpl.errorConfig,
          variables: merged,
        },
      });
      // 通知侧栏面板的 vendor 缓存失效，下次进入时按新值重新检查
      try {
        const { clearVendorCheckCache } = await import("@/components/stock-analysis/vendorCheck");
        clearVendorCheckCache();
      } catch { /* 浏览器模式或路径不存在时跳过 */ }
      message.success(t("stockAnalysis.settings.saveSuccess"));
    } catch {
      message.error(t("stockAnalysis.settings.saveFailed"));
    } finally {
      setSaving(false);
    }
  }, [vendorValues, iwencaiKey, xueqiuToken, neodataToken, t, message]);

  const checkOne = useCallback(async (vendorName: string) => {
    setHealth((prev) => ({ ...prev, [vendorName]: "pending" }));
    try {
      await invoke("check_vendor_health", { vendor: vendorName });
      setHealth((prev) => ({ ...prev, [vendorName]: "ok" }));
    } catch {
      setHealth((prev) => ({ ...prev, [vendorName]: "fail" }));
    }
  }, []);

  const checkAll = useCallback(async () => {
    setCheckingAll(true);
    const names = [
      "tencent",
      "eastmoney",
      "sina",
      "ths",
      "cninfo",
      "baidu_stock",
      "iwencai",
      "akshare",
      "mootdx",
      "xueqiu",
      "neodata",
    ];
    for (const n of names) { setHealth((prev) => ({ ...prev, [n]: "pending" })); }
    for (const n of names) { await checkOne(n); }
    setCheckingAll(false);
    // 延迟一帧读取 health 状态，自动关闭失败的 vendor
    setTimeout(() => {
      setHealth((prev) => {
        const toDisable: Record<string, boolean> = {};
        for (const n of names) {
          if (prev[n] === "fail") {
            const key = `vendor_${n}`;
            toDisable[key] = false;
          }
        }
        if (Object.keys(toDisable).length > 0) {
          setVendorValues((prev) => ({ ...prev, ...toDisable }));
          message.warning(t("stockAnalysis.settings.vendor.autoDisabled", { count: Object.keys(toDisable).length }));
        }
        return prev;
      });
    }, 100);
  }, [checkOne, t, message]);

  if (!loaded) {
    return (
      <div className="flex justify-center py-12">
        <Spin />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3">
      {/* 顶部操作栏 */}
      <div className="flex items-center justify-between">
        <Space size={4}>
          <span className="text-sm text-gray-500">{t("stockAnalysis.settings.vendors.summary")}</span>
        </Space>
        <Space>
          <Button size="small" loading={checkingAll} onClick={checkAll}>
            {t("stockAnalysis.settings.checkHealth")}
          </Button>
          <Button size="small" type="primary" loading={saving} onClick={handleSave}>
            {t("stockAnalysis.settings.saveConfig")}
          </Button>
        </Space>
      </div>

      {/* Vendor 卡片（含依赖的固定工具） */}
      {VENDORS.map((v) => {
        const enabled = vendorValues[v.key] ?? false;
        const status = health[v.enName] ?? "idle";
        const deps = vendorTools[v.enName] ?? [];
        const fixedDeps = deps.filter((d) => d.kind === "fixed");
        const exposedDeps = deps.filter((d) => d.kind === "exposed");

        return (
          <Card
            key={v.key}
            size="small"
            className={enabled ? "" : "opacity-60"}
            title={
              <Space size={8}>
                <Switch
                  size="small"
                  checked={enabled}
                  onChange={(checked) => setVendorValues((prev) => ({ ...prev, [v.key]: checked }))}
                />
                <span className="font-medium text-sm">{v.name}</span>
                <span className="text-xs text-gray-400">{v.desc}</span>
                <Tag
                  className="text-xs m-0"
                  color={status === "ok"
                    ? "green"
                    : status === "fail"
                    ? "red"
                    : status === "pending"
                    ? "blue"
                    : "default"}
                >
                  {status === "ok"
                    ? t("stockAnalysis.settings.connected")
                    : status === "fail"
                    ? t("stockAnalysis.settings.disconnected")
                    : status === "pending"
                    ? t("stockAnalysis.settings.vendors.checking")
                    : t("stockAnalysis.settings.vendors.notChecked")}
                </Tag>
              </Space>
            }
            extra={
              <Space size={4}>
                {v.key === "vendor_iwencai" && (
                  <Select
                    style={{ width: 180 }}
                    size="small"
                    mode="tags"
                    maxCount={1}
                    placeholder={t("stockAnalysis.settings.vendors.apiKey")}
                    value={iwencaiKey ? [iwencaiKey] : []}
                    onChange={(vals) => setIwencaiKey(vals[0] ?? "")}
                  />
                )}
                {v.key === "vendor_xueqiu" && (
                  <Input.Password
                    style={{ width: 180 }}
                    size="small"
                    placeholder={t("stockAnalysis.settings.vendors.apiKey")}
                    value={xueqiuToken}
                    onChange={(e) => setXueqiuToken(e.target.value)}
                  />
                )}
                {v.key === "vendor_neodata" && (
                  <>
                    <Input.Password
                      style={{ width: 180 }}
                      size="small"
                      placeholder={t("stockAnalysis.settings.vendors.apiKey")}
                      value={neodataToken}
                      onChange={(e) => setNeodataToken(e.target.value)}
                    />
                    <Button
                      size="small"
                      type="primary"
                      ghost
                      onClick={() => {
                        message.info("请在 WorkBuddy 对话中输入「刷新 NeoData token」自动完成连接");
                      }}
                    >
                      🔗 通过 WorkBuddy 连接
                    </Button>
                  </>
                )}
                <Button size="small" onClick={() => checkOne(v.enName)}>
                  {t("stockAnalysis.settings.check")}
                </Button>
              </Space>
            }
          >
            {/* 能力标签 */}
            <div className="mb-2">
              <Space wrap size={[2, 4]}>
                {v.capabilities.map((cap) => (
                  <Tag key={cap} color="blue" className="text-xs m-0">{CAP_LABELS[cap] ?? cap}</Tag>
                ))}
              </Space>
            </div>
            {/* 帮助信息：API Key 获取地址 / 使用说明 */}
            {v.helpUrl && (
              <div className="text-xs text-blue-500 mb-2">
                🔑{" "}
                <a href={v.helpUrl} target="_blank" rel="noopener noreferrer" className="underline">
                  {v.helpUrl}
                </a>
                {v.helpText && <span className="text-gray-400 ml-1">{v.helpText}</span>}
              </div>
            )}
            {v.helpText && !v.helpUrl && <div className="text-xs text-gray-400 mb-2">💡 {v.helpText}</div>}
            {/* 固定工具 (🔧) + LLM 暴露工具 (🤖) */}
            {deps.length > 0 && (
              <div className="border-t border-gray-100 pt-2 mt-1">
                {fixedDeps.length > 0 && (
                  <div className="mb-1">
                    <div className="text-xs text-gray-400 mb-0.5">
                      🔧 {t("stockAnalysis.settings.vendors.fixedToolHeader")}
                    </div>
                    {fixedDeps.map((tr) => (
                      <div key={tr.tool} className="flex items-center gap-1 text-xs py-0.5">
                        <Tag color="default" className="text-xs m-0">{tr.tool}</Tag>
                        <span className="text-gray-400">{tr.label}</span>
                        <span className="text-gray-300">
                          {tr.vendors.map((vn, i) => (
                            <span key={vn}>
                              {i > 0 ? " → " : ""}
                              <span
                                className={vn === v.enName
                                  ? "font-medium text-gray-600"
                                  : health[vn] === "ok"
                                  ? "text-green-600"
                                  : "text-gray-400"}
                              >
                                {vn}
                              </span>
                            </span>
                          ))}
                        </span>
                      </div>
                    ))}
                  </div>
                )}
                {exposedDeps.length > 0 && (
                  <div>
                    <div className="text-xs text-gray-400 mb-0.5">
                      🤖 {t("stockAnalysis.settings.vendors.exposedToolHeader")}
                    </div>
                    {exposedDeps.map((tr) => (
                      <div key={tr.tool} className="flex items-center gap-1 text-xs py-0.5">
                        <Tag color="green" className="text-xs m-0">{tr.tool}</Tag>
                        <span className="text-gray-400">{tr.label}</span>
                        <span className="text-gray-300">
                          {tr.vendors.map((vn, i) => (
                            <span key={vn}>
                              {i > 0 ? " → " : ""}
                              <span
                                className={vn === v.enName
                                  ? "font-medium text-gray-600"
                                  : health[vn] === "ok"
                                  ? "text-green-600"
                                  : "text-gray-400"}
                              >
                                {vn}
                              </span>
                            </span>
                          ))}
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </Card>
        );
      })}

      {/* 本地计算工具 */}
      <div className="pt-2">
        <div className="text-xs text-gray-400 mb-2">{t("stockAnalysis.settings.vendors.localCompute")}</div>
        <Space wrap size={[4, 8]}>
          {LOCAL_TOOLS.map((lt) => <Tag key={lt.tool} color="purple" className="text-xs m-0">💻 {lt.tool}</Tag>)}
        </Space>
      </div>
    </div>
  );
}
