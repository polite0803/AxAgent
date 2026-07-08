import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import globals from "globals";
import tseslint from "typescript-eslint";

// m8: 合并逐文件豁免为按模式分组，减少豁免块数量并添加 TECH-DEBT 注释。
// 后续应逐个修复底层问题并移除对应豁免。

export default tseslint.config(
  { ignores: ["src/i18n/compare_locales.js"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    languageOptions: {
      ecmaVersion: 2020,
      globals: globals.browser,
    },
    plugins: {
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "react-refresh/only-export-components": ["warn", { allowConstantExport: true }],
      "no-unused-vars": "off",
      "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_" }],
      "react-hooks/exhaustive-deps": "error",
      "@typescript-eslint/ban-ts-comment": "error",
    },
  },
  // ── 测试文件：放宽严格类型与未使用变量检查 ──
  {
    files: ["**/__tests__/**", "**/test/**", "**/*.test.ts", "**/*.test.tsx", "**/*.spec.ts", "**/*.spec.tsx"],
    rules: {
      "@typescript-eslint/no-explicit-any": "off",
      "@typescript-eslint/no-unused-vars": "off",
      "@typescript-eslint/ban-ts-comment": "off",
    },
  },
  // ── 声明文件：关闭未使用变量限制（导出类型即视为使用） ──
  {
    files: ["**/*.d.ts"],
    rules: {
      "@typescript-eslint/no-unused-vars": "off",
      "@typescript-eslint/ban-ts-comment": "off",
    },
  },
  // ── TECH-DEBT: 以下豁免应逐步修复底层代码问题后移除 ──
  //   - Workflow Nodes 组件的 props 解构未使用变量 → 需重构 Props 类型
  //   - ChatViewMessages 的 setState-in-effect → 需重构状态管理
  //   - 多文件缺少 named export → 需添加 displayName 或 named export
  {
    files: [
      "src/components/workflow/Nodes/*.tsx",
    ],
    rules: {
      "@typescript-eslint/no-unused-vars": "off",
    },
  },
  {
    files: ["src/components/chat/ChatViewMessages.tsx"],
    rules: {
      "react-hooks/set-state-in-effect": "off",
      "react-hooks/preserve-manual-memoization": "off",
    },
  },
  {
    files: [
      "src/components/dynamicUI/DynamicUIRenderer.tsx",
      "src/components/layout/CommandPalette.tsx",
      "src/components/notification/NotificationCenter.tsx",
      "src/components/settings/HookExecutionLog.tsx",
      "src/components/chat/ChatViewMessages.tsx",
    ],
    rules: {
      "react-refresh/only-export-components": "off",
    },
  },
);
