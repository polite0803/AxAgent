export type ArtifactKind = "draft" | "note" | "report" | "snippet" | "checklist";
export type ArtifactFormat =
  | "markdown"
  | "text"
  | "json"
  | "html"
  | "css"
  | "javascript"
  | "typescript"
  | "jsx"
  | "tsx"
  | "python"
  | "svg"
  | "mermaid"
  | "d2";

export type ArtifactLanguage = ArtifactFormat;

export type ArtifactPreviewMode = "split" | "preview" | "code";

export type Artifact = {
  id: string;
  conversationId: string;
  kind: ArtifactKind;
  title: string;
  content: string;
  format: ArtifactFormat;
  language?: ArtifactLanguage;
  previewMode?: ArtifactPreviewMode;
  metadata?: {
    lineCount?: number;
    lastExecuted?: string;
    executionOutput?: string;
  };
  pinned: boolean;
  updatedAt: string;
};

export type CreateArtifactInput = {
  conversationId: string;
  sourceMessageId?: string;
  kind: ArtifactKind;
  title: string;
  content: string;
  format: ArtifactFormat;
};

export type UpdateArtifactInput = {
  title?: string;
  content?: string;
  format?: ArtifactFormat;
  language?: ArtifactLanguage;
  previewMode?: ArtifactPreviewMode;
  metadata?: Artifact["metadata"];
  pinned?: boolean;
};
