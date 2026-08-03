/**
 * 开发模式下抑制 antd 弃用警告。
 * antd v6 仍支持 Input.TextArea、Tag color 等旧 API，但会在 console.error 中输出弃用警告。
 * 等 antd 提供稳定的迁移路径（如 TextArea 顶层导出）后可移除此文件。
 */
if (import.meta.env.DEV) {
  const antdWarningPattern = /^Warning:\s*\[antd:/;
  const originalError = console.error.bind(console);
  const originalWarn = console.warn.bind(console);

  console.error = ((...args: unknown[]) => {
    const firstArg = typeof args[0] === "string" ? args[0] : "";
    if (antdWarningPattern.test(firstArg)) { return; }
    originalError(...args);
  }) as typeof console.error;

  console.warn = ((...args: unknown[]) => {
    const firstArg = typeof args[0] === "string" ? args[0] : "";
    if (antdWarningPattern.test(firstArg)) { return; }
    originalWarn(...args);
  }) as typeof console.warn;
}
