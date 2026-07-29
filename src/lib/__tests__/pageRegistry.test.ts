// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it } from "vitest";

import { BUILTIN_PAGE_PATH, DEFAULT_HOME } from "../pageRegistry";

describe("pageRegistry", () => {
  describe("DEFAULT_HOME", () => {
    it("应为 /chat（仪表盘已合并到对话页工作台 Tab）", () => {
      expect(DEFAULT_HOME).toBe("/chat");
    });
  });

  describe("BUILTIN_PAGE_PATH", () => {
    it("应包含所有内置页面", () => {
      expect(BUILTIN_PAGE_PATH.chat).toBe("/chat");
      expect(BUILTIN_PAGE_PATH.dashboard).toBe("/dashboard");
      expect(BUILTIN_PAGE_PATH.settings).toBe("/settings");
      expect(BUILTIN_PAGE_PATH.workflow).toBe("/workflow");
      expect(BUILTIN_PAGE_PATH.files).toBe("/files");
      expect(BUILTIN_PAGE_PATH.knowledge).toBe("/knowledge");
      expect(BUILTIN_PAGE_PATH.memory).toBe("/memory");
      expect(BUILTIN_PAGE_PATH.link).toBe("/link");
      expect(BUILTIN_PAGE_PATH.gateway).toBe("/gateway");
      expect(BUILTIN_PAGE_PATH.terminal).toBe("/terminal");
    });

    it("应包含 devtools 路由", () => {
      expect(BUILTIN_PAGE_PATH.devtools).toBe("/devtools");
      expect(BUILTIN_PAGE_PATH.devtoolsTraceExplorer).toBe("/devtools/trace-explorer");
      expect(BUILTIN_PAGE_PATH.devtoolsBenchmark).toBe("/devtools/benchmark");
      expect(BUILTIN_PAGE_PATH.devtoolsFineTune).toBe("/devtools/fine-tune");
    });

    it("所有路径应以 / 开头", () => {
      for (const path of Object.values(BUILTIN_PAGE_PATH)) {
        expect(path).toMatch(/^\//);
      }
    });

    it("不应有重复路径", () => {
      const paths = Object.values(BUILTIN_PAGE_PATH);
      const uniquePaths = new Set(paths);
      expect(uniquePaths.size).toBe(paths.length);
    });
  });
});
