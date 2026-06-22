// SPDX-License-Identifier: AGPL-3.0-only

/// <reference types="vite/client" />
/// <reference types="node" />
/// <reference types="vitest" />

interface FileSystemDirectoryHandle {
  name: string;
}

interface Window {
  showDirectoryPicker(): Promise<FileSystemDirectoryHandle>;
}

declare namespace JSX {
  interface IntrinsicElements {
    "emoji-picker": React.DetailedHTMLProps<
      React.HTMLAttributes<HTMLElement>,
      HTMLElement
    >;
  }
}
