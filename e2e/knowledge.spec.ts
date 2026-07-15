import { test } from "@playwright/test";

/**
 * 知识库 E2E —— 目前为「显式待办」状态（test.fixme）。
 *
 * 背景（2026-07-14 缺陷核查 L18）：原有用例全部把断言包在
 * `if (locator.isVisible().catch(() => false))` 里，且所用的 data-testid
 * （knowledge-base-page / collections-list / create-collection-btn / …）
 * 在整个代码库中并不存在。结果是无论功能是否正常，所有用例都 PASS，
 * 提供零真实覆盖，属于「假阳性测试」。
 *
 * 为避免继续给出虚假的绿色信号，这里统一标记为 test.fixme：
 * 它们会被报告为「待修复/跳过」，而不是「通过」。
 *
 * 恢复真实覆盖前需完成：
 *   1. 在知识库页面组件（KnowledgeBasePage / 集合列表 / 新建按钮 /
 *      搜索框 / 文档计数）上补齐稳定的 data-testid。
 *   2. 去掉 isVisible() 条件保护，改为无条件断言真实 DOM。
 *   3. 补充建库、导入文档、检索命中等关键路径的断言。
 *
 * 注：原文件还混入了一段与知识库无关的「Agent Management E2E」拷贝块，
 * 已移除（Agent 相关用例应放在各自的 spec 中）。
 */
test.describe("Knowledge Base E2E", () => {
  test.fixme("should display knowledge base page", async () => {
    // 待补齐 data-testid="knowledge-base-page" 后实现真实断言
  });

  test.fixme("should list knowledge collections", async () => {
    // 待补齐 data-testid="collections-list" 后实现真实断言
  });

  test.fixme("should create a new collection", async () => {
    // 待补齐建库按钮/表单的 data-testid 后实现真实断言
  });

  test.fixme("should display search functionality", async () => {
    // 待补齐 data-testid="knowledge-search-input" 后实现真实断言
  });

  test.fixme("should show document count per collection", async () => {
    // 待补齐 data-testid="document-count" 后实现真实断言
  });
});
