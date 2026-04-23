import { defineConfig, type Plugin } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import monacoEditorPluginModule from "vite-plugin-monaco-editor";
import path from "path";

const monacoEditorPlugin = (monacoEditorPluginModule as any).default || monacoEditorPluginModule;

const host = process.env.TAURI_DEV_HOST;

// Only bundle commonly-used Shiki language grammars (saves ~8 MB in build).
// Languages not listed here will gracefully degrade (no syntax highlighting).
const SHIKI_ALLOWED_LANGS = new Set([
  "angular-html", "angular-ts", "astro", "bash", "c", "cpp", "csharp",
  "css", "dart", "dockerfile", "go", "graphql", "html", "html-derivative",
  "java", "javascript", "json", "json5", "jsonc", "jsx", "kotlin", "less",
  "lua", "markdown", "mdc", "mdx", "objective-c", "objective-cpp", "php",
  "python", "ruby", "rust", "sass", "scss", "shell", "shellscript",
  "sql", "svelte", "swift", "toml", "tsx", "typescript", "vue",
  "vue-html", "xml", "yaml",
]);

function shikiLanguageFilter(): Plugin {
  return {
    name: "shiki-language-filter",
    enforce: "pre",
    resolveId(id) {
      const m = id.match(/^@shikijs\/langs\/(.+)$/);
      if (m && !SHIKI_ALLOWED_LANGS.has(m[1])) {
        return "\0shiki-lang-noop";
      }
      return null;
    },
    load(id) {
      if (id === "\0shiki-lang-noop") {
        return "export default []";
      }
      return null;
    },
  };
}

// Remove crossorigin attributes from script/link tags in the built HTML.
// Tauri's custom protocol (tauri://localhost) does not handle CORS preflight
// requests, so crossorigin attributes cause all scripts and stylesheets to
// fail loading in the packaged app, resulting in a blank window.
function removeCrossorigin(): Plugin {
  return {
    name: "remove-crossorigin",
    transformIndexHtml(html) {
      return html.replace(/ crossorigin/g, "");
    },
  };
}

export default defineConfig(async () => ({
  base: "./",
  plugins: [react(), tailwindcss(), monacoEditorPlugin({}), shikiLanguageFilter(), removeCrossorigin()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  build: {
    modulePreload: { polyfill: false },
    chunkSizeWarningLimit: 600,
    rolldownOptions: {
      output: {
        codeSplitting: {
          minSize: 20000,
          groups: [
            {
              name: 'vendor',
              test: /node_modules/,
              priority: 10,
            },
            {
              name: 'monaco-editor',
              test: /monaco-editor/,
              priority: 20,
            },
            {
              name: 'markstream',
              test: /markstream/,
              priority: 15,
            },
          ],
        },
      },
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
    exclude: [
      "src/pages/__tests__/ChatPage.test.tsx",
      "src/components/chat/__tests__/InputArea.test.tsx",
      "src/components/files/__tests__/FilesPage.test.tsx",
      "src/components/settings/__tests__/ProviderDetail.test.tsx",
    ],
  },
}));
