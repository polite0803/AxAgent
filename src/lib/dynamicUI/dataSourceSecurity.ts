// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 动态 UI 数据源安全边界（共享模块）。
 *
 * 供 DataBindingEngine 运行时校验与 SchemaValidator 结构校验复用，
 * 避免白名单逻辑在多处重复定义造成漂移。
 */

/**
 * 允许从动态 UI 数据源调用的 Tauri 命令前缀白名单。
 *
 * 安全约束：dataSource.type === "api" 的 endpoint 来自 schema（Agent 生成或用户导入，
 * 属不可信输入）。为避免构造任意 schema 绕过前端权限调用任意已注册 Tauri 命令
 * （未鉴权 IPC / SSRF），仅放行动态 UI 渲染自身的命令前缀。
 * 新增可调用的命令时，请在此显式登记其前缀。
 */
export const ALLOWED_INVOKE_PREFIXES = ["dynamic_ui_"];

/** 允许的 fetch 协议白名单（仅 http/https，禁止 file://, javascript:, data: 等）。 */
export const ALLOWED_FETCH_PROTOCOLS = ["https:", "http:"];

/**
 * 校验 Tauri 命令名是否在白名单前缀内。
 */
export function isAllowedInvokeEndpoint(endpoint: string): boolean {
  return ALLOWED_INVOKE_PREFIXES.some((prefix) => endpoint.startsWith(prefix));
}

/**
 * 校验 fetch URL 是否使用受控协议且可解析。
 */
export function isAllowedFetchUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    return ALLOWED_FETCH_PROTOCOLS.includes(parsed.protocol);
  } catch {
    return false;
  }
}
