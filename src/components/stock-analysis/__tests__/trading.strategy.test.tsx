// 静态保证测试: trading.rs:343 写入交易时必须设置 strategy 字段
// 触发: 之前 trades::ActiveModel 初始化缺 strategy 字段,cargo check 失败
// (E0063 missing field `strategy`)。此测试保证修复后该字段始终被显式赋值。

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

describe("trading.rs — trades::ActiveModel 必须设置 strategy 字段", () => {
  it("execute_trade 函数体的 trades::ActiveModel 初始化必须含 strategy 字段", () => {
    const projectRoot = process.cwd();
    const srcPath = resolve(projectRoot, "src-tauri/crates/stock-analysis/src/trading.rs");
    const src = readFileSync(srcPath, "utf8");

    // 定位到 execute_trade 函数体的 trades::ActiveModel 块
    const startMarker = "let trade = trades::ActiveModel {";
    const startIdx = src.indexOf(startMarker);
    expect(startIdx).toBeGreaterThan(0);
    const endIdx = src.indexOf("};", startIdx);
    const block = src.slice(startIdx, endIdx);

    // 必须有 strategy: Set(...)
    expect(block).toMatch(/strategy:\s*Set\(/);
  });
});
