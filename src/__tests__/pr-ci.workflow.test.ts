// 集成测试: pr-ci.yml E2E 配置检查
//
// macOS runner 上使用系统 Chrome 而非下载 Playwright 专用 Chromium，
// 因为 cdn.playwright.dev 在 macOS CI runner 上下载 165MB 后卡在解压阶段。
// 用 system Chrome (channel: chrome) 跳过下载，依赖 macOS runner 预装的 Chrome。

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

function loadPrCi(): string {
  const projectRoot = process.cwd();
  const ymlPath = resolve(projectRoot, ".github/workflows/pr-ci.yml");
  return readFileSync(ymlPath, "utf8");
}

describe(".github/workflows/pr-ci.yml — Playwright 配置", () => {
  it("E2E job 使用 system Chrome（无 playwright install 步骤）", () => {
    const yml = loadPrCi();
    // 不应有 playwright install 命令（使用系统 Chrome）
    const installLines = [...yml.matchAll(/playwright\s+install/g)];
    expect(installLines.length).toBe(0);
    // 应有 PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD 环境变量
    expect(yml).toContain("PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD");
  });

  it("E2E job 运行在 macOS，无 timeout-minutes", () => {
    const yml = loadPrCi();
    const jobIdx = yml.indexOf("test-e2e:");
    expect(jobIdx).toBeGreaterThan(0);
    const after = yml.slice(jobIdx);
    const nextJobMatch = after.slice(1).search(/\n {2}[a-zA-Z][a-zA-Z0-9_-]*:/);
    const end = nextJobMatch > 0 ? jobIdx + 1 + nextJobMatch : yml.length;
    const jobBlock = yml.slice(jobIdx, end);
    expect(jobBlock).toMatch(/macOS/);
    expect(jobBlock).not.toMatch(/HOMEBREW_NO_AUTO_UPDATE/);
    expect(jobBlock).not.toMatch(/timeout-minutes/);
  });
});
