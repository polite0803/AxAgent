export type Note = {
  id: string;
  vaultId: string;
  title: string;
  filePath: string;
  content: string;
  contentHash: string;
  author: string;
  pageType?: string;
  sourceRefs?: string[];
  relatedPages?: string[];
  qualityScore?: number;
  lastLintedAt?: number;
  lastCompiledAt?: number;
  compiledSourceHash?: string;
  userEdited: boolean;
  userEditedAt?: number;
  createdAt: number;
  updatedAt: number;
  isDeleted: boolean;
};

export type CreateNoteInput = {
  vaultId: string;
  title: string;
  filePath: string;
  content: string;
  author: string;
  pageType?: string;
  sourceRefs?: string[];
};

export type UpdateNoteInput = {
  title?: string;
  content?: string;
  pageType?: string;
  relatedPages?: string[];
};

export type NoteLink = {
  id: number;
  vaultId: string;
  sourceNoteId: string;
  targetNoteId: string;
  linkText: string;
  linkType: string;
  createdAt: number;
};

export type NoteSearchResult = {
  note: Note;
  snippet: string;
  score: number;
};