---
name: fix-mcp-store-test-errors
overview: Replace incorrect `McpStore` and `awaitMcpStore` references with `useMcpStore` in test file
todos:
  - id: rename-mcpstore-refs
    content: 将 mcpStore.test.ts 中 `McpStore` 和 `awaitMcpStore` 全部替换为 `useMcpStore`（共16处）
    status: completed
  - id: verify-fix
    content: 运行 `npm run typecheck` 确认零 TS 错误，运行测试确认通过并使用 [subagent:code-reviewer] 审查
    status: completed
    dependencies:
      - rename-mcpstore-refs
---

修复 `src/stores/feature/__tests__/mcpStore.test.ts` 中的 TypeScript 编译错误：

- `McpStore` 未定义（3处）——测试中错误引用了一个不存在的变量名
- `awaitMcpStore` 未定义（13处）——同样是不存在的变量名

实际测试文件已导入 `useMcpStore`（Zustand store），而 `useMcpStore` 本身就是 StoreApi 对象，自带了 `getState()` 和 `setState()` 方法。因此只需将两处错误名称统一替换为 `useMcpStore` 即可。

## 技术方案

### 修复策略

单一文件 `src/stores/feature/__tests__/mcpStore.test.ts` 全局字符串替换，无需新增/删除 import。

### 替换规则

| 原文本                     | 替换为                   | 出现位置（行号）                                                          |
| -------------------------- | ------------------------ | ------------------------------------------------------------------------- |
| `McpStore.setState(`       | `useMcpStore.setState(`  | L57, L124, L141                                                           |
| `awaitMcpStore.getState()` | `useMcpStore.getState()` | L71, L81, L93, L109, L128, L148, L160, L169, L180, L192, L205, L226, L235 |

共 16 处替换，全部在同一个文件中完成。无需修改其他文件。

### 验证方式

替换后运行 `npm run typecheck`（即 `tsc --noEmit`）确认零类型错误，再运行 `npm run test:run` 确认测试通过。

## Agent 扩展使用

### SubAgent

- **code-reviewer**: 替换完成后，使用 [subagent:code-reviewer] 审查修改的正确性，确保替换无误且无副作用。
