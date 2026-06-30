// SPDX-License-Identifier: AGPL-3.0-only

import type { ComponentRegistryEntry, DynamicComponentType } from "@/types";

/**
 * 全局组件注册表。
 * 使用 Map<string, ComponentRegistryEntry> 存储注册的组件，
 * 支持按类型、分类查询，及动态注册/注销。
 */
class ComponentRegistry {
  private registry = new Map<string, ComponentRegistryEntry>();

  /**
   * 注册单个组件。
   * 如果同类型已存在，将覆盖旧组件。
   */
  register(entry: ComponentRegistryEntry): void {
    this.registry.set(entry.type, entry);
  }

  /**
   * 批量注册组件。
   */
  registerBatch(entries: ComponentRegistryEntry[]): void {
    for (const entry of entries) {
      this.registry.set(entry.type, entry);
    }
  }

  /**
   * 根据组件类型获取注册项。
   */
  get(type: string): ComponentRegistryEntry | undefined {
    return this.registry.get(type);
  }

  /**
   * 按分类获取所有注册项。
   */
  getByCategory(category: string): ComponentRegistryEntry[] {
    const result: ComponentRegistryEntry[] = [];
    for (const entry of this.registry.values()) {
      if (entry.category === category) {
        result.push(entry);
      }
    }
    return result;
  }

  /**
   * 检查指定类型是否已注册。
   */
  has(type: string): boolean {
    return this.registry.has(type);
  }

  /**
   * 注销指定类型的组件（用于技能卸载时清理自定义组件）。
   */
  unregister(type: string): void {
    this.registry.delete(type);
  }

  /**
   * 获取所有已注册的组件类型列表。
   */
  getAllTypes(): DynamicComponentType[] {
    return [...this.registry.keys()] as DynamicComponentType[];
  }

  /**
   * 清空所有注册（仅用于测试/重置）。
   */
  clear(): void {
    this.registry.clear();
  }
}

/** 全局单例 */
export const componentRegistry = new ComponentRegistry();
