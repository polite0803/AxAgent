import { invoke } from "@/lib/invoke";
import { message } from "antd";
import i18next from "i18next";

/** 面板 → 必需 vendor 列表（前端 UI 直接使用）*/
export const PANEL_VENDORS: Record<string, string[]> = {
  limitup: ["ths", "baidu_stock", "iwencai"],
  dragontiger: ["eastmoney", "baidu_stock"],
  sectors: ["ths", "baidu_stock"],
  north: ["ths", "baidu_stock"],
  screener: ["eastmoney", "tencent", "ths", "baidu_stock", "iwencai", "akshare"],
  events: ["cninfo", "eastmoney", "baidu_stock"],
};

export type VendorCheckResult =
  | { status: "ok" }
  | { status: "disabled"; panelName: string; vendors: string[] }
  | { status: "backend_offline" };

/** 缓存当前已启用的 vendor 集合，避免每个面板重复 RPC */
let enabledCache: { set: Set<string>; ts: number } | null = null;
const CACHE_TTL_MS = 30_000;

async function getEnabledVendors(): Promise<Set<string> | null> {
  if (enabledCache && Date.now() - enabledCache.ts < CACHE_TTL_MS) {
    return enabledCache.set;
  }
  try {
    const tmpl = await invoke("get_workflow_template", { id: "stock-analysis" }) as Record<string, unknown>;
    const vars: { name: string; value: unknown }[] = (tmpl?.variables as { name: string; value: unknown }[]) ?? [];
    const enabledSet = new Set<string>();
    for (const v of vars) {
      // 白名单：只认 vendor_enabled_ 前缀或显式布尔值，避免将字符串 key/token 误判为启用
      if (v.name.startsWith("vendor_enabled_") && v.value === true) {
        enabledSet.add(v.name.replace("vendor_", ""));
      } else if (
        v.name.startsWith("vendor_") && !v.name.includes("_key") && !v.name.includes("_token")
        && !v.name.includes("_secret")
        && !v.name.includes("_password") && !v.name.includes("_appid")
        && (v.value === true || v.value === "true")
      ) {
        enabledSet.add(v.name.replace("vendor_", ""));
      }
    }
    enabledCache = { set: enabledSet, ts: Date.now() };
    return enabledSet;
  } catch {
    return null;
  }
}

/** 清除缓存（在设置页保存 vendor 后由调用方主动调用）*/
export function clearVendorCheckCache() {
  enabledCache = null;
}

/**
 * 检查指定面板的数据源是否已启用。
 *
 * - 未知面板：返回 "ok"（无 vendoring 需求）
 * - 没有已启用的 vendor：toast 警告并返回 "disabled"
 * - 后端取不到 workflow template：toast 错误并返回 "backend_offline"
 */
export async function checkVendorEnabled(
  panelKey: string,
  opts: { silent?: boolean } = {},
): Promise<VendorCheckResult> {
  const names = PANEL_VENDORS[panelKey];
  if (!names) { return { status: "ok" }; }
  const enabledSet = await getEnabledVendors();
  if (enabledSet === null) {
    if (!opts.silent) {
      message.error(i18next.t("stockAnalysis.settings.vendor.backendOffline"));
    }
    return { status: "backend_offline" };
  }
  if (!names.some((n) => enabledSet.has(n))) {
    if (!opts.silent) {
      message.warning(i18next.t("stockAnalysis.settings.vendor.disabled", { names: names.join(" / ") }));
    }
    return { status: "disabled", panelName: panelKey, vendors: names };
  }
  return { status: "ok" };
}
