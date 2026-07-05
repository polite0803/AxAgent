// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 创建一个防抖函数，在最后一次调用后等待 `delay` 毫秒后执行。
 * @param fn 需要防抖的函数
 * @param delay 延迟毫秒数（默认 500）
 */
export function debounce<T extends (...args: never[]) => void>(
  fn: T,
  delay = 500,
): (...args: Parameters<T>) => void {
  let timer: ReturnType<typeof setTimeout> | null = null;
  return (...args: Parameters<T>) => {
    if (timer !== null) {
      clearTimeout(timer);
    }
    timer = setTimeout(() => {
      fn(...args);
      timer = null;
    }, delay);
  };
}
