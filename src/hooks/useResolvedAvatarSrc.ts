// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, isTauri } from "@/lib/invoke";
import type { AvatarType } from "@/stores";
import { useEffect, useState } from "react";

/**
 * Resolves a file-type avatar value to a renderable src string.
 * - Relative paths are resolved via `read_attachment_preview`.
 */
export function useResolvedAvatarSrc(
  avatarType: AvatarType,
  avatarValue: string,
): string | undefined {
  const [resolved, setResolved] = useState<string | undefined>(undefined);

  useEffect(() => {
    // 合并两个 undefined 分支：非 file 类型、非 Tauri 环境均走同一 setResolved 路径
    if (avatarType !== "file" || !avatarValue || !isTauri()) {
      setTimeout(() => setResolved(undefined), 0);
      return;
    }
    let cancelled = false;
    invoke<string>("read_attachment_preview", { filePath: avatarValue })
      .then((dataUrl) => {
        if (!cancelled) {
          setResolved(dataUrl);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setResolved(undefined);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [avatarType, avatarValue]);

  return resolved;
}
