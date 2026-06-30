import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: "html",
  timeout: 120 * 1000,
  expect: {
    timeout: 30000,
  },

  use: {
    baseURL: "http://localhost:1420",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },

  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"], channel: "chrome" },
    },
  ],

  webServer: {
    // 使用 npm run dev（仅 Vite）而非 tauri dev，因为：
    // 1. E2E 测试为纯前端测试，使用 browserMock（localStorage）模拟后端
    // 2. macOS CI runner 无图形界面，Tauri WebView 无法初始化
    // 3. 如未来需要测试 Tauri API，请改用 webDriver 方案
    command: "npm run dev",
    url: "http://localhost:1420",
    reuseExistingServer: !process.env.CI,
    timeout: 120 * 1000,
  },
});
