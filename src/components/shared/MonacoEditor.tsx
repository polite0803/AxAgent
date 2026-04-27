import type { ArtifactLanguage } from "@/types/artifact";
import { useEffect, useRef } from "react";

declare global {
  interface Window {
    monaco: typeof import("monaco-editor");
  }
}

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
  const editorRef = useRef<import("monaco-editor").editor.IStandaloneCodeEditor | null>(null);

  useEffect(() => {
    if (!containerRef.current) { return; }

    const editor = window.monaco.editor.create(containerRef.current, {
      value,
      language: LANGUAGE_MAP[language] || "plaintext",
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
        const newValue = editor.getValue();
        onChange(newValue);
      });
    }

    return () => {
      editor.dispose();
    };
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
        window.monaco.editor.setModelLanguage(model, LANGUAGE_MAP[language] || "plaintext");
      }
    }
  }, [language]);

  return <div ref={containerRef} style={{ height, width: "100%" }} />;
}
