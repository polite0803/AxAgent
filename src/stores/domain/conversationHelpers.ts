// 单调递增计数器，与 Date.now() 组合防止同毫秒 ID 重复
let _idSeq = 0;
export function tempId(prefix: string): string {
  return `${prefix}${Date.now()}-${++_idSeq}`;
}
