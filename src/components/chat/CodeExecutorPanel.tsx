import { MonacoEditor } from "@/components/shared/MonacoEditor";
import { codeExecutor, type ExecutionResult } from "@/lib/codeExecutor";
import type { ArtifactLanguage } from "@/types/artifact";
import { Button, Select, Spin } from "antd";
import { Play, Square } from "lucide-react";
import { useState } from "react";

const LANGUAGES = [
  { value: "javascript", label: "JavaScript" },
  { value: "typescript", label: "TypeScript" },
  { value: "python", label: "Python" },
] as const;

interface CodeExecutorPanelProps {
  initialCode?: string;
  language?: "javascript" | "typescript" | "python";
}

export function CodeExecutorPanel({
  initialCode = "",
  language = "javascript",
}: CodeExecutorPanelProps) {
  const [code, setCode] = useState(initialCode);
  const [execLanguage, setExecLanguage] = useState(language);
  const [result, setResult] = useState<ExecutionResult | null>(null);
  const [loading, setLoading] = useState(false);

  const handleExecute = async () => {
    setLoading(true);
    setResult(null);
    try {
      const execResult = await codeExecutor.execute({
        code,
        language: execLanguage,
      });
      setResult(execResult);
    } finally {
      setLoading(false);
    }
  };

  const handleStop = () => {
    setLoading(false);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div
        style={{
          padding: "8px 16px",
          borderBottom: "1px solid #eee",
          display: "flex",
          alignItems: "center",
          gap: 12,
        }}
      >
        <Select
          value={execLanguage}
          onChange={setExecLanguage}
          options={[...LANGUAGES]}
          style={{ width: 140 }}
          size="small"
        />
        {loading
          ? (
            <Button
              danger
              icon={<Square size={14} />}
              onClick={handleStop}
              size="small"
            >
              Stop
            </Button>
          )
          : (
            <Button
              type="primary"
              icon={<Play size={14} />}
              onClick={handleExecute}
              size="small"
            >
              Run
            </Button>
          )}
        {loading && <Spin size="small" />}
      </div>

      <div style={{ flex: 1, minHeight: 200 }}>
        <MonacoEditor
          value={code}
          language={execLanguage as ArtifactLanguage}
          onChange={setCode}
          height="100%"
        />
      </div>

      {result && (
        <div
          style={{
            borderTop: "1px solid #eee",
            padding: 12,
            background: "#1e1e1e",
            color: "#d4d4d4",
            fontFamily: "monospace",
            fontSize: 13,
            maxHeight: 200,
            overflow: "auto",
          }}
        >
          <div style={{ marginBottom: 8, color: "#888" }}>
            Exit code: {result.exit_code} | Duration: {result.duration_ms?.toFixed(2)}ms
          </div>

          {result.stdout && (
            <div style={{ marginBottom: 8, whiteSpace: "pre-wrap" }}>
              <span style={{ color: "#4ec9b0" }}>stdout:</span>
              <pre style={{ margin: "4px 0 0 0" }}>{result.stdout}</pre>
            </div>
          )}

          {result.stderr && (
            <div style={{ color: "#f48771" }}>
              <span style={{ color: "#f48771" }}>stderr:</span>
              <pre style={{ margin: "4px 0 0 0" }}>{result.stderr}</pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
