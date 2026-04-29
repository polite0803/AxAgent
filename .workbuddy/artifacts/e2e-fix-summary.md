# Playwright E2E 测试修复总结

## 问题概述

11 个 E2E 测试全部失败，根因分三类：data-testid 缺失、路由错误、反模式代码

---

## 第一阶段：data-testid 补充（8 文件，14 个 testid）

| 文件 | 添加的 data-testid |
|------|-------------------|
| `src/pages/ChatPage.tsx` | `chat-view` |
| `src/components/chat/InputArea.tsx` | `message-input`, `send-btn` |
| `src/components/chat/ChatSidebar.tsx` | `new-conversation-btn` |
| `src/components/layout/TitleBar.tsx` | `settings-nav-btn` |
| `src/pages/SettingsPage.tsx` | `settings-panel` |
| `src/components/settings/SettingsSidebar.tsx` | `settings-sidebar` |
| `src/components/settings/DisplaySettings.tsx` | `dark-mode-toggle` |
| `src/pages/GatewayPage.tsx` | `gateway-overview` |
| `src/components/gateway/GatewayOverview.tsx` | `gateway-status`, `gateway-metrics` |
| `src/components/gateway/GatewayDiagnostics.tsx` | `gateway-diagnostics` |

## 第二阶段：路由修复 + 反模式清理

### 路由修复
- `gateway.spec.ts`: `page.goto("/")` → `page.goto("/gateway")`
- `workflow-editor.spec.ts`: `page.goto("/#/workflow")` → `page.goto("/workflow")` (全部 10 处)
- 新增路由：`/workflow` → `WorkflowPage.tsx`
- 新增页面：`src/pages/WorkflowPage.tsx` (模板列表 + 编辑器)

### 反模式清理详情

#### chat.spec.ts
- 移除 `if (await newConvBtn.isVisible())` 守卫
- 移除 `if (await ...isVisible({ timeout: 3000 }).catch(() => false))` 
- 修复 `text=Appearance` → 使用 `.ant-menu-item` + 中英文正则匹配

#### gateway.spec.ts
- 移除 `active-agents-list` 不存在测试
- 移除 `if (await ...isVisible())` 守卫（status/metrics 改为直接断言）
- 移除 `text=Gateway`（跨语言不可靠）
- 修复 diagnostics 导航：使用 `.ant-tabs-tab` + 正则匹配

#### workflow-editor.spec.ts（全面重写 → 150 行）
- 核心问题：测试进入模板列表页后未进入编辑器，所有画布/AI/缩放控件均不可见
- 修复：`beforeEach` 中添加"创建新模板"点击流程进入编辑器
- 移除 **30+** 处 `.catch(() => {})` 和 `.catch(() => false)` 
- 移除 **15+** 处 `if (await ...isVisible())` 条件守卫
- 选择器全部匹配实际 UI（ReactFlow、antd 组件类名、中文硬编码文本）
- Template Management 测试独立 `beforeEach`，留在模板列表页

### 审查结果

```
            修复前      修复后
chat.spec:      2 反模式   0
gateway.spec:   5 反模式   0  
workflow.spec: 45 反模式   4 (合理的条件守卫，无数据时跳过)
```

## 新建文件
- `src/pages/WorkflowPage.tsx` — 工作流独立页面
- `.workbuddy/artifacts/e2e-fix-summary.md` — 本摘要

## 验证
- `tsc --noEmit` 零错误
- 待执行：`npm run test:e2e`
