// SPDX-License-Identifier: AGPL-3.0-only

import { Button } from "antd";
import type { GlobalToken } from "antd";
import { ArrowUp, Square } from "lucide-react";
import { useTranslation } from "react-i18next";
import { CommandSuggest } from "./CommandSuggest";

export function InputAreaTextarea(props: {
  value: string;
  cursorPosition: number;
  showSuggest: boolean;
  textareaRef: React.RefObject<HTMLTextAreaElement | null>;
  userMinHeight: number;
  ABSOLUTE_MAX_HEIGHT: number;
  streaming: boolean;
  token: GlobalToken;
  onInput: (e: React.ChangeEvent<HTMLTextAreaElement>) => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  onPaste: (e: React.ClipboardEvent<HTMLTextAreaElement>) => void;
  onCancel: () => void;
  onSend: () => void;
  onCursorChange: (pos: number) => void;
  onShowSuggestChange: (v: boolean) => void;
  onCommandSelect: (replacement: string) => void;
}) {
  const { t } = useTranslation();

  const {
    value,
    cursorPosition,
    showSuggest,
    textareaRef,
    userMinHeight,
    ABSOLUTE_MAX_HEIGHT,
    streaming,
    token,
    onInput,
    onKeyDown,
    onPaste,
    onCancel,
    onSend,
    onCursorChange,
    onShowSuggestChange,
    onCommandSelect,
  } = props;

  return (
    <div className="chat-input-box">
      <CommandSuggest
        value={value}
        cursorPosition={cursorPosition}
        onSelect={onCommandSelect}
        visible={showSuggest}
      />
      <textarea
        className="axagent-input-textarea"
        ref={textareaRef}
        data-testid="message-input"
        value={value}
        onChange={onInput}
        onKeyDown={onKeyDown}
        onPaste={onPaste}
        placeholder={t("chat.inputPlaceholder")}
        aria-label={t("chat.inputPlaceholder")}
        rows={1}
        style={{
          color: token.colorText,
          minHeight: userMinHeight,
          maxHeight: ABSOLUTE_MAX_HEIGHT,
        }}
        onKeyUp={() => {
          if (textareaRef.current) {
            onCursorChange(textareaRef.current.selectionStart);
            const textBefore = value.slice(0, textareaRef.current.selectionStart);
            const atLineStart = textBefore === ""
              || textBefore.endsWith(" ")
              || textBefore.endsWith("\n");
            const hasActiveSlash = atLineStart && /\/\S{1,}$/.test(textBefore);
            const hasActiveAt = atLineStart && /@\S{1,}$/.test(textBefore);
            onShowSuggestChange(hasActiveSlash || hasActiveAt);
          }
        }}
        onClick={() => {
          if (textareaRef.current) {
            onCursorChange(textareaRef.current.selectionStart);
          }
        }}
      />
      {streaming
        ? (
          <Button
            shape="circle"
            size="small"
            danger
            data-testid="stop-generation-btn"
            icon={<Square size={14} />}
            onClick={onCancel}
            style={{ flexShrink: 0, alignSelf: "flex-end" }}
          />
        )
        : (
          <Button
            type="primary"
            shape="circle"
            size="small"
            data-testid="send-btn"
            aria-label={t("chat.sendMessage")}
            icon={<ArrowUp size={16} />}
            onClick={onSend}
            disabled={!value.trim() || streaming}
            style={{ flexShrink: 0, alignSelf: "flex-end", width: 36, height: 36 }}
            className={value.trim() && !streaming
              ? "ax-glow-shadow"
              : ""}
          />
        )}
    </div>
  );
}
