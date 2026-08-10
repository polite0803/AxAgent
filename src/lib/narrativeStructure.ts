// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type { NarrativeStructure } from "@/types/narrative";

/// 叙事结构响应（与后端 Rust DTO 对应）
export interface NarrativeStructureRecord {
  id: string;
  name: string;
  description?: string;
  genre: string;
  structure: NarrativeStructure;
  isTemplate: boolean;
  version: number;
  createdAt: number;
  updatedAt: number;
}

/// 创建叙事结构请求
export interface CreateNarrativeRequest {
  id: string;
  name: string;
  description?: string;
  genre: string;
  structure: NarrativeStructure;
  isTemplate?: boolean;
}

/// 更新叙事结构请求
export interface UpdateNarrativeRequest {
  id: string;
  name?: string;
  description?: string;
  genre?: string;
  structure?: NarrativeStructure;
}

/// 列出叙事结构
export async function listNarrativeStructures(
  isTemplate?: boolean,
  genre?: string,
): Promise<NarrativeStructureRecord[]> {
  return invoke<NarrativeStructureRecord[]>("list_narrative_structures", {
    isTemplate,
    genre,
  });
}

/// 获取单个叙事结构
export async function getNarrativeStructure(
  id: string,
): Promise<NarrativeStructureRecord | null> {
  return invoke<NarrativeStructureRecord | null>("get_narrative_structure", { id });
}

/// 创建叙事结构
export async function createNarrativeStructure(
  input: CreateNarrativeRequest,
): Promise<NarrativeStructureRecord> {
  return invoke<NarrativeStructureRecord>("create_narrative_structure", { input });
}

/// 更新叙事结构
export async function updateNarrativeStructure(
  input: UpdateNarrativeRequest,
): Promise<NarrativeStructureRecord> {
  return invoke<NarrativeStructureRecord>("update_narrative_structure", { input });
}

/// 删除叙事结构
export async function deleteNarrativeStructure(id: string): Promise<void> {
  return invoke<void>("delete_narrative_structure", { id });
}
