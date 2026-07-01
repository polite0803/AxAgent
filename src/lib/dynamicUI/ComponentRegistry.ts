// SPDX-License-Identifier: AGPL-3.0-only

import type { ComponentRegistryEntry, DynamicComponentType } from "@/types";

interface NamespacedComponentEntry extends ComponentRegistryEntry {
  namespace?: string;
}

class ComponentRegistry {
  private registry = new Map<string, NamespacedComponentEntry>();

  register(entry: ComponentRegistryEntry, namespace?: string): void {
    const fullKey = namespace ? `${namespace}:${entry.type}` : entry.type;
    this.registry.set(fullKey, { ...entry, namespace });
  }

  registerBatch(entries: ComponentRegistryEntry[], namespace?: string): void {
    for (const entry of entries) {
      this.register(entry, namespace);
    }
  }

  get(type: string): ComponentRegistryEntry | undefined {
    if (this.registry.has(type)) {
      return this.registry.get(type);
    }
    for (const [, entry] of this.registry) {
      const entryType = entry.namespace ? `${entry.namespace}:${entry.type}` : entry.type;
      if (entryType === type) {
        return entry;
      }
      if (!entry.namespace && entry.type === type) {
        return entry;
      }
    }
    return undefined;
  }

  resolve(type: string, namespace?: string): ComponentRegistryEntry | undefined {
    if (namespace) {
      const namespacedKey = `${namespace}:${type}`;
      if (this.registry.has(namespacedKey)) {
        return this.registry.get(namespacedKey);
      }
    }
    const globalKey = type;
    return this.registry.get(globalKey);
  }

  getByCategory(category: string): ComponentRegistryEntry[] {
    const result: ComponentRegistryEntry[] = [];
    for (const entry of this.registry.values()) {
      if (entry.category === category) {
        result.push(entry);
      }
    }
    return result;
  }

  has(type: string): boolean {
    return this.get(type) !== undefined;
  }

  unregister(type: string, namespace?: string): void {
    const fullKey = namespace ? `${namespace}:${type}` : type;
    this.registry.delete(fullKey);
  }

  unregisterNamespace(namespace: string): void {
    for (const [key] of this.registry) {
      if (key.startsWith(`${namespace}:`)) {
        this.registry.delete(key);
      }
    }
  }

  getAllTypes(): DynamicComponentType[] {
    return [...this.registry.keys()] as DynamicComponentType[];
  }

  clear(): void {
    this.registry.clear();
  }
}

export const componentRegistry = new ComponentRegistry();
