/** Zustand Store 注册表，供声明式 Action 的 store 类型访问 */

type StoreAccessor = {
  get: (payload?: unknown) => unknown;
  set: (payload?: unknown) => void;
  update: (payload?: unknown) => void;
};

const storeRegistry = new Map<string, StoreAccessor>();

export function getStoreRegistry(): Map<string, StoreAccessor> {
  return storeRegistry;
}

export function registerStore(name: string, accessor: StoreAccessor): void {
  storeRegistry.set(name, accessor);
}

export function unregisterStore(name: string): void {
  storeRegistry.delete(name);
}
