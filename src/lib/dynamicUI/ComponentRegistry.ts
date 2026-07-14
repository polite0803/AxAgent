// SPDX-License-Identifier: AGPL-3.0-only

import type { ComponentRegistryEntry, DynamicComponentType } from "@/types";

interface NamespacedComponentEntry extends ComponentRegistryEntry {
  namespace?: string;
}

class ComponentRegistry {
  private registry = new Map<string, NamespacedComponentEntry>();
  /** Reverse index mapping bare type -> full key for O(1) fallback lookup. */
  private typeIndex = new Map<string, string>();

  register(entry: ComponentRegistryEntry, namespace?: string): void {
    const fullKey = namespace ? `${namespace}:${entry.type}` : entry.type;
    this.registry.set(fullKey, { ...entry, namespace });
    // Maintain reverse index for bare type lookup
    if (!this.typeIndex.has(entry.type)) {
      this.typeIndex.set(entry.type, fullKey);
    }
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
    // O(1) fallback via reverse index
    const fullKey = this.typeIndex.get(type);
    if (fullKey) {
      return this.registry.get(fullKey);
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
    const entry = this.registry.get(globalKey);
    if (entry) {
      return entry;
    }
    // O(1) fallback via reverse index
    const fullKey = this.typeIndex.get(type);
    if (fullKey) {
      return this.registry.get(fullKey);
    }
    return undefined;
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
    const entry = this.registry.get(fullKey);
    if (entry) {
      // Remove from reverse index if no other entry uses the same bare type
      let hasDup = false;
      for (const [key, e] of this.registry) {
        if (key !== fullKey && e.type === entry.type) {
          hasDup = true;
          break;
        }
      }
      if (!hasDup) {
        this.typeIndex.delete(entry.type);
      }
    }
    this.registry.delete(fullKey);
  }

  unregisterNamespace(namespace: string): void {
    for (const [key] of this.registry) {
      if (key.startsWith(`${namespace}:`)) {
        this.unregister(key.replace(`${namespace}:`, ""), namespace);
      }
    }
  }

  getAllTypes(): DynamicComponentType[] {
    return [...this.registry.keys()]
      .filter((key) => !key.includes(":"))
      .map((key) => key as DynamicComponentType);
  }

  clear(): void {
    this.registry.clear();
    this.typeIndex.clear();
  }
}

export const componentRegistry = new ComponentRegistry();
