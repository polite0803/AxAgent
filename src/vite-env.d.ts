/// <reference types="vite/client" />
/// <reference types="node" />
/// <reference types="vitest" />

declare namespace JSX {
  interface IntrinsicElements {
    "emoji-picker": React.DetailedHTMLProps<React.HTMLAttributes<HTMLElement>, HTMLElement>;
  }
}
