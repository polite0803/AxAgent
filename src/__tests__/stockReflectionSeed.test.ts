// 静态测试: seed_reflection_workflow_template 必须用 Rust 类型构造节点,
// 不能用 serde_json::json!() 裸写(否则反序列化时因缺 title/retry/enabled
// 等必填字段而静默失败,工作流编辑器只能看到空白画布)。
//
// 触发链:
//  1. 启动时 seed_reflection_workflow_template 写入 stock-reflection 模板
//  2. 用户在工作流编辑器打开 stock-reflection
//  3. 后端 model_to_response 调 serde_json::from_str(&model.nodes)
//  4. 若 JSON 缺 title/retry/enabled 等必填字段 → 反序列化失败
//  5. unwrap_or_default() 吞错,返回空 Vec → 前端画布无节点
//
// 修复: 改用 stock-analysis 同款 Rust 类型构造路径(WorkflowNode::Trigger /
// SubWorkflow / Agent / Storage + WorkflowNodeBase 全字段),编译器会强制要求
// 所有必填字段,根除此类 schema 漂移。

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

function loadSource(): string {
  const projectRoot = process.cwd();
  const srcPath = resolve(
    projectRoot,
    "src-tauri/src/commands/stock_analysis_setup.rs",
  );
  return readFileSync(srcPath, "utf8");
}

/** 提取 `seed_reflection_workflow_template` 函数体(从 fn 声明到匹配的 } 结束)。 */
function extractSeedFnBody(src: string): string {
  const fnStart = src.indexOf("async fn seed_reflection_workflow_template");
  expect(fnStart, "seed_reflection_workflow_template 函数未找到").toBeGreaterThan(0);
  // 从 fn 声明的 { 开始扫描,用嵌套花括号匹配函数体结束位置
  const braceStart = src.indexOf("{", fnStart);
  let depth = 1;
  let i = braceStart + 1;
  for (; i < src.length; i++) {
    const c = src[i];
    if (c === "{") { depth++; }
    else if (c === "}") {
      depth--;
      if (depth === 0) { break; }
    }
  }
  return src.slice(braceStart, i);
}

describe("stock_analysis_setup.rs — seed_reflection_workflow_template", () => {
  const body = (() => {
    try {
      return extractSeedFnBody(loadSource());
    } catch {
      return "";
    }
  })();

  it("应使用 Rust 类型构造 4 个节点(Trigger / SubWorkflow / Agent / Storage),不用 serde_json::json!", () => {
    expect(body).toMatch(/WorkflowNode::Trigger\s*\(/);
    expect(body).toMatch(/WorkflowNode::SubWorkflow\s*\(/);
    expect(body).toMatch(/WorkflowNode::Agent\s*\(/);
    expect(body).toMatch(/WorkflowNode::Storage\s*\(/);
    // 主节点数组不应再裸写 JSON(变量/trigger_config 仍可用 json! 但节点本身必须用类型)
    expect(body).not.toMatch(/let\s+nodes\s*=\s*serde_json::json!\s*\(\s*\[/);
  });

  it("每个 WorkflowNodeBase 必须包含 id / title / position / retry / enabled 字段(否则反序列化会失败)", () => {
    // 至少有 4 处 `id: "...".into()` 风格的 base.id
    const idMatches = body.match(/id:\s*".+?"\.into\(\)/g) ?? [];
    expect(idMatches.length, "期望至少 4 个 id 字段").toBeGreaterThanOrEqual(4);
    // 至少 4 处 title
    const titleMatches = body.match(/title:\s*".+?"\.into\(\)/g) ?? [];
    expect(titleMatches.length, "期望至少 4 个 title 字段").toBeGreaterThanOrEqual(4);
    // 至少 4 处 position
    const positionMatches = body.match(/position:\s*Position\s*\{/g) ?? [];
    expect(positionMatches.length, "期望至少 4 个 position 字段").toBeGreaterThanOrEqual(4);
    // 至少 4 处 retry
    const retryMatches = body.match(/retry:\s*RetryConfig/g) ?? [];
    expect(retryMatches.length, "期望至少 4 个 retry 字段").toBeGreaterThanOrEqual(4);
    // 至少 4 处 enabled
    const enabledMatches = body.match(/enabled:\s*(true|false)/g) ?? [];
    expect(enabledMatches.length, "期望至少 4 个 enabled 字段").toBeGreaterThanOrEqual(4);
  });

  it("应通过 serde_json::to_string(&nodes) 序列化为字符串(走和 stock-analysis 一样的路径)", () => {
    // 模拟 stock-analysis 的写法:先 to_string(&nodes) 再存 DB
    expect(body).toMatch(/serde_json::to_string\s*\(\s*&nodes\s*\)/);
  });
});
