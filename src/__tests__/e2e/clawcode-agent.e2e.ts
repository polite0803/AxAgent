import { expect, test } from "@playwright/test";

test("ClawCode Agent End-to-End Test", async ({ page }) => {
  // Navigate to the chat page
  await page.goto("http://localhost:5173");

  // Wait for the page to load
  await page.waitForSelector('[data-testid="chat-input"]');

  // Select a provider and model that supports ClawCode
  await page.click('[data-testid="model-selector"]');
  await page.click('[data-testid="model-option-clawcode"]');

  // Enter a simple query that should trigger tool use
  await page.fill('[data-testid="chat-input"]', "What is 123 + 456?");
  await page.click('[data-testid="send-button"]');

  // Wait for the agent to respond
  await page.waitForSelector('[data-testid="assistant-message"]');

  // Check if the response contains the correct answer
  const assistantMessage = await page.textContent('[data-testid="assistant-message"]');
  expect(assistantMessage).toContain("579");

  // Test a more complex query that uses multiple tools
  await page.fill('[data-testid="chat-input"]', 'Echo "Hello World" and then add 10 + 20');
  await page.click('[data-testid="send-button"]');

  // Wait for the agent to respond
  await page.waitForSelector('[data-testid="assistant-message"]');

  // Check if the response contains both tool results
  const assistantMessage2 = await page.textContent('[data-testid="assistant-message"]');
  expect(assistantMessage2).toContain("Hello World");
  expect(assistantMessage2).toContain("30");

  // Test streaming functionality
  await page.fill('[data-testid="chat-input"]', "Write a short story about a robot");
  await page.click('[data-testid="send-button"]');

  // Wait for streaming to start and complete
  await page.waitForSelector('[data-testid="streaming-indicator"]');
  await page.waitForSelector('[data-testid="streaming-indicator"]', { state: "hidden" });

  // Check if the story was generated
  const storyMessage = await page.textContent('[data-testid="assistant-message"]');
  expect(storyMessage).toContain("robot");
  expect(storyMessage?.length).toBeGreaterThan(100);
});

test("ClawCode Agent Error Handling", async ({ page }) => {
  // Navigate to the chat page
  await page.goto("http://localhost:5173");

  // Wait for the page to load
  await page.waitForSelector('[data-testid="chat-input"]');

  // Select ClawCode model
  await page.click('[data-testid="model-selector"]');
  await page.click('[data-testid="model-option-clawcode"]');

  // Enter a query that should cause an error (invalid tool input)
  await page.fill('[data-testid="chat-input"]', 'Add "hello" + "world"');
  await page.click('[data-testid="send-button"]');

  // Wait for the agent to respond with an error
  await page.waitForSelector('[data-testid="error-message"]');

  // Check if the error message is displayed
  const errorMessage = await page.textContent('[data-testid="error-message"]');
  expect(errorMessage).toContain("error");
  expect(errorMessage).toContain("invalid");
});
