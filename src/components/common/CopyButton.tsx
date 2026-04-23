import React, { useCallback } from 'react';
import { Check, Copy } from 'lucide-react';
import { Button, message, theme } from 'antd';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';

export interface CopyButtonProps {
  /** Text to copy, or async function returning text to copy */
  text: string | (() => string | Promise<string>);
  /** Icon size in px (default: 14) */
  size?: number;
  /** Duration of success state in ms (default: 2000) */
  timeout?: number;
  /** If provided, shows message.success() with this string on copy */
  successMessage?: string;
  /** Called after successful copy */
  onSuccess?: () => void;
  /** Called on copy failure (clipboard error or text getter error) */
  onError?: (error: unknown) => void;
  /** Additional inline style */
  style?: React.CSSProperties;
  /** Additional className */
  className?: string;
}

export const CopyButton = React.forwardRef<HTMLElement, CopyButtonProps>(
  function CopyButton(
    { text, size = 14, timeout = 2000, successMessage, onSuccess, onError, style, className },
    ref,
  ) {
    const { token } = theme.useToken();
    const { copy, isCopied } = useCopyToClipboard({ timeout });

    const handleClick = useCallback(async () => {
      try {
        const value = typeof text === 'function' ? await text() : text;
        const ok = await copy(value);
        if (ok) {
          if (successMessage) message.success(successMessage);
          onSuccess?.();
        }
      } catch (e) {
        onError?.(e);
      }
    }, [text, copy, successMessage, onSuccess, onError]);

    return (
      <Button
        ref={ref as React.Ref<HTMLButtonElement>}
        type="text"
        size="small"
        icon={isCopied ? <Check size={size} style={{ color: token.colorSuccess }} /> : <Copy size={size} />}
        onClick={handleClick}
        style={style}
        className={className}
      />
    );
  },
);
