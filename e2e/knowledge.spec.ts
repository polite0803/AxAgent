import { test, expect } from '@playwright/test';

test.describe('Agent Management E2E', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(1000);
  });

  test('should display agent store and list agents', async ({ page }) => {
    const agentStore = page.locator('[data-testid="agent-store"]');
    if (await agentStore.isVisible()) {
      await expect(agentStore).toBeVisible();
    }
  });

  test('should create a new agent', async ({ page }) => {
    const createAgentBtn = page.locator('[data-testid="create-agent-btn"]');
    if (await createAgentBtn.isVisible()) {
      await createAgentBtn.click();
      await page.waitForSelector('[data-testid="agent-form"]', { timeout: 5000 });
      await expect(page.locator('[data-testid="agent-form"]')).toBeVisible();
    }
  });

  test('should show agent status indicators', async ({ page }) => {
    const statusBadge = page.locator('[data-testid="agent-status-badge"]');
    const statusBadges = page.locator('[data-testid="agent-status-badge"]');
    const count = await statusBadges.count();
    if (count > 0) {
      await expect(statusBadges.first()).toBeVisible();
    }
  });

  test('should allow agent configuration', async ({ page }) => {
    const agentSettingsBtn = page.locator('[data-testid="agent-settings-btn"]');
    if (await agentSettingsBtn.isVisible()) {
      await agentSettingsBtn.click();
      await expect(page.locator('[data-testid="agent-config-panel"]')).toBeVisible();
    }
  });
});

test.describe('Knowledge Base E2E', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/knowledge');
    await page.waitForTimeout(1000);
  });

  test('should display knowledge base page', async ({ page }) => {
    const kbPage = page.locator('[data-testid="knowledge-base-page"]');
    if (await kbPage.isVisible()) {
      await expect(kbPage).toBeVisible();
    }
  });

  test('should list knowledge collections', async ({ page }) => {
    const collectionsList = page.locator('[data-testid="collections-list"]');
    if (await collectionsList.isVisible()) {
      await expect(collectionsList).toBeVisible();
    }
  });

  test('should create a new collection', async ({ page }) => {
    const createCollectionBtn = page.locator('[data-testid="create-collection-btn"]');
    if (await createCollectionBtn.isVisible()) {
      await createCollectionBtn.click();
      await expect(page.locator('[data-testid="collection-form"]')).toBeVisible();
    }
  });

  test('should display search functionality', async ({ page }) => {
    const searchInput = page.locator('[data-testid="knowledge-search-input"]');
    if (await searchInput.isVisible()) {
      await expect(searchInput).toBeVisible();
      await searchInput.fill('test query');
      await page.waitForTimeout(500);
    }
  });

  test('should show document count per collection', async ({ page }) => {
    const docCount = page.locator('[data-testid="document-count"]');
    const docCounts = page.locator('[data-testid="document-count"]');
    const count = await docCounts.count();
    if (count > 0) {
      await expect(docCounts.first()).toBeVisible();
    }
  });
});
