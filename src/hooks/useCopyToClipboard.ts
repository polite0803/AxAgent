import { useCallback, useEffect, useRef, useState } from 'react';
import { writeText as tauriWriteText } from '@tauri-apps/plugin-clipboard-manager';

async function writeToClipboard(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    // Fallback to Tauri native clipboard (survives expired user gesture)
    await tauriWriteText(text);
  }
}

export interface UseCopyToClipboardOptions {
  /** Duration of success state in ms (default: 2000) */
  timeout?: number;
}

export function useCopyToClipboard(options?: UseCopyToClipboardOptions) {
  const timeout = options?.timeout ?? 2000;
  const [copiedValue, setCopiedValue] = useState<string | null>(null);
  const timeoutRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  const copy = useCallback(
    async (text: string): Promise<boolean> => {
      try {
        await writeToClipboard(text);
        setCopiedValue(text);
        if (timeoutRef.current) clearTimeout(timeoutRef.current);
        timeoutRef.current = setTimeout(() => setCopiedValue(null), timeout);
        return true;
      } catch {
        return false;
      }
    },
    [timeout],
  );

  useEffect(() => {
    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, []);

  return {
    copy,
    copiedValue,
    isCopied: copiedValue !== null,
    isCopiedFor: useCallback((text: string) => copiedValue === text, [copiedValue]),
  };
}
