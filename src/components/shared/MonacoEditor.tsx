// SPDX-License-Identifier: AGPL-3.0-only

import { loadMonaco } from "@/lib/monaco";
import type { ArtifactLanguage } from "@/types";
import { useEffect, useRef } from "react";

const LANGUAGE_MAP: Record<ArtifactLanguage, string> = {
  javascript: "javascript",
  typescript: "typescript",
  jsx: "javascript",
  tsx: "typescript",
  html: "html",
  css: "css",
  python: "python",
  markdown: "markdown",
  text: "plaintext",
  json: "json",
  svg: "xml",
  mermaid: "markdown",
  d2: "markdown",
};

interface MonacoEditorProps {
  value: string;
  language: ArtifactLanguage;
  onChange?: (value: string) => void;
  readOnly?: boolean;
  height?: string | number;
}

export function MonacoEditor({
  value,
  language,
  onChange,
  readOnly = false,
  height = "100%",
}: MonacoEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<
    import("monaco-editor").editor.IStandaloneCodeEditor | null
  >(null);

  // monaco 动态加载是异步的，编辑器创建时须使用最新的 value/language，
  // 不能依赖挂载时闭包捕获的初始值（加载完成前 props 可能已更新）。
  const valueRef = useRef(value);
  valueRef.current = value;
  const languageRef = useRef(language);
  languageRef.current = language;

  useEffect(() => {
    let disposed = false;
    let editor: import("monaco-editor").editor.IStandaloneCodeEditor | null = null;

    loadMonaco()
      .then((monaco) => {
        if (disposed || !containerRef.current) {
          return;
        }

        editor = monaco.editor.create(containerRef.current, {
          value: valueRef.current,
          language: LANGUAGE_MAP[languageRef.current] || "plaintext",
          readOnly,
          theme: "vs-dark",
          minimap: { enabled: false },
          fontSize: 13,
          lineNumbers: "on",
          scrollBeyondLastLine: false,
          automaticLayout: true,
          wordWrap: "on",
          padding: { top: 8 },
        });

        editorRef.current = editor;

        if (onChange) {
          editor.onDidChangeModelContent(() => {
            const newValue = editor?.getValue() ?? "";
            onChange(newValue);
          });
        }
      })
      .catch((e) => {
        console.error("[MonacoEditor] 加载 monaco-editor 失败:", e);
      });

    return () => {
      disposed = true;
      editor?.dispose();
      editorRef.current = null;
    };
    // 仅挂载时初始化 Monaco editor 实例，后续通过另一个 effect 更新内容。
    // 加入 value/language/onChange 会导致每次变更销毁并重建编辑器，性能极差。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (editorRef.current) {
      const model = editorRef.current.getModel();
      if (model && model.getValue() !== value) {
        editorRef.current.setValue(value);
      }
    }
  }, [value]);

  useEffect(() => {
    if (editorRef.current) {
      const model = editorRef.current.getModel();
      if (model) {
        loadMonaco().then((monaco) => {
          monaco.editor.setModelLanguage(
            model,
            LANGUAGE_MAP[language] || "plaintext",
          );
        });
      }
    }
  }, [language]);

  return <div ref={containerRef} style={{ height, width: "100%" }} />;
}
