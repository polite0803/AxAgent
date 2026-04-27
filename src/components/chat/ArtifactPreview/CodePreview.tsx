import { MonacoEditor } from "@/components/shared/MonacoEditor";
import type { ArtifactLanguage } from "@/types/artifact";
import { memo } from "react";

interface CodePreviewProps {
  code: string;
  language: ArtifactLanguage;
  readOnly?: boolean;
  onChange?: (code: string) => void;
}

export const CodePreview = memo(function CodePreview({
  code,
  language,
  readOnly = false,
  onChange,
}: CodePreviewProps) {
  return (
    <MonacoEditor
      value={code}
      language={language}
      readOnly={readOnly}
      onChange={onChange}
    />
  );
});
