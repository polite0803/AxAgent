import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import path from "path";
import monacoEditorPluginModule from "vite-plugin-monaco-editor";
import { defineConfig, type Plugin } from "vitest/config";

// Windows 盘符大小写规范化（必须在任何路径解析前执行）：
// node ESM 按字面 URL 缓存模块，cwd 盘符为小写（如 d:/）时，vite 与 node 各自
// 解析出的模块 URL 盘符大小写不一致，会把 @vitest/runner 加载成两个独立实例
// （worker 收集器与测试文件各持一份），表现为所有测试收集阶段失败：
// "TypeError: Cannot read properties of undefined (reading 'config')"。
// 此处统一把 cwd 盘符转为大写，确保 worker 子进程继承一致的模块 URL。
if (process.platform === "win32") {
  const cwd = process.cwd();
  if (/^[a-z]:/.test(cwd)) {
    process.chdir(cwd[0].toUpperCase() + cwd.slice(1));
  }
}

interface MonacoEditorPluginModule {
  default?: Plugin;
  [key: string]: unknown;
}

const monacoEditorPlugin = (monacoEditorPluginModule as MonacoEditorPluginModule).default
  || monacoEditorPluginModule as Plugin;

const host = process.env.TAURI_DEV_HOST;

// Only bundle commonly-used Shiki language grammars (saves ~8 MB in build).
// Languages not listed here will gracefully degrade (no syntax highlighting).
const SHIKI_ALLOWED_LANGS = new Set([
  "angular-html",
  "angular-ts",
  "astro",
  "bash",
  "c",
  "cpp",
  "csharp",
  "css",
  "dart",
  "dockerfile",
  "go",
  "graphql",
  "html",
  "html-derivative",
  "java",
  "javascript",
  "json",
  "json5",
  "jsonc",
  "jsx",
  "kotlin",
  "less",
  "lua",
  "markdown",
  "mdc",
  "mdx",
  "objective-c",
  "objective-cpp",
  "php",
  "python",
  "rhai",
  "ruby",
  "rust",
  "sass",
  "scss",
  "shell",
  "shellscript",
  "sql",
  "svelte",
  "swift",
  "toml",
  "tsx",
  "typescript",
  "vue",
  "vue-html",
  "xml",
  "yaml",
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
      "@": path.resolve(import.meta.dirname, "./src"),
    },
  },
  css: {
    // postcss 而非 lightningcss：antd/@ant-design/x 的 CSS-in-JS
    // 会生成非标准选择器/属性（如 .[agent:researcher-1]），
    // lightningcss 严格解析器报 Unexpected token Semicolon
    transformer: "postcss",
  },
  build: {
    sourcemap: false, // 生产构建不暴露源码
    modulePreload: { polyfill: false },
    // 阈值设为略高于最大的单体第三方库 chunk（d2 ~8.2MB）。
    // antd / monaco-editor / mermaid / d2 等单体库无法拆分到 1MB 以下，
    // 可拆分的大库（mermaid/phaser/recharts/xyflow/framer-motion/xterm/dagre/antv 等）
    // 已在上方 codeSplitting.groups 中拆为独立 chunk。此阈值仅用于兜底这些单体库，
    // 仍可捕获新增的明显体积回归。
    chunkSizeWarningLimit: 8500, // 单位 kB
    rolldownOptions: {
      output: {
        codeSplitting: {
          minSize: 20000,
          groups: [
            // ── High-priority named groups (split BEFORE the generic vendor group) ──
            {
              name: "monaco-editor",
              test: /monaco-editor/,
              priority: 30,
            },
            {
              name: "markstream",
              test: /markstream/,
              priority: 25,
            },
            {
              name: "antd-vendor",
              test: /node_modules\/(?:antd|@ant-design|antd-style|@lobehub|@rc-component|rc-[^/]+)/,
              priority: 20,
            },
            {
              name: "react-vendor",
              test: /node_modules\/(?:react[^/]*|scheduler|react-dom|react-router|@remix-run)/,
              priority: 20,
            },
            {
              name: "tauri-vendor",
              test: /node_modules\/@tauri-apps/,
              priority: 20,
            },
            {
              name: "markdown-vendor",
              test: /node_modules\/(?:stream-markdown|stream-monaco|katex)/,
              priority: 20,
            },
            {
              name: "i18n-vendor",
              test: /node_modules\/(?:i18next|react-i18next)/,
              priority: 20,
            },
            {
              name: "ui-vendor",
              test:
                /node_modules\/(?:lucide-react|overlayscrollbars|clsx|emoji-picker-element|html2canvas|@tanstack|reactflow|@atlaskit)/,
              priority: 20,
            },
            {
              name: "d2-vendor",
              test: /node_modules\/@terrastruct\/d2/,
              priority: 20,
            },
            // ── Granular groups for the large fallback libs (priority 15) ──
            // 将原先落入 vendor 兜底组的单体大库拆分为独立 chunk，便于缓存与按需加载
            {
              name: "recharts-vendor",
              test: /node_modules\/recharts/,
              priority: 15,
            },
            {
              name: "phaser-vendor",
              test: /node_modules\/phaser/,
              priority: 15,
            },
            {
              name: "sigma-vendor",
              test: /node_modules\/sigma\//,
              priority: 15,
            },
            {
              name: "xyflow-vendor",
              test: /node_modules\/@xyflow/,
              priority: 15,
            },
            {
              name: "framer-motion-vendor",
              test: /node_modules\/framer-motion/,
              priority: 15,
            },
            {
              name: "dnd-kit-vendor",
              test: /node_modules\/@dnd-kit/,
              priority: 15,
            },
            {
              name: "xterm-vendor",
              test: /node_modules\/@xterm/,
              priority: 15,
            },
            {
              name: "dagre-vendor",
              test: /node_modules\/(?:dagre|graphlib|@dagrejs)/,
              priority: 15,
            },
            {
              name: "antv-vendor",
              test: /node_modules\/@antv/,
              priority: 15,
            },
            {
              name: "font-vendor",
              test: /node_modules\/@fontsource-variable/,
              priority: 15,
            },
            {
              name: "dprint-vendor",
              test: /node_modules\/@dprint/,
              priority: 15,
            },
            {
              name: "mermaid-vendor",
              test: /node_modules\/mermaid/,
              priority: 15,
            },
            // ── Fallback: everything else in node_modules ──
            {
              name: "vendor",
              test: /node_modules/,
              priority: 10,
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
    host: host || "127.0.0.1",
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  test: {
    environment: "jsdom",
    globals: true,
    // 使用 forks 池替代默认 threads 池：vitest 4.1.4 的 threads 池在 Windows 下
    // 加载 @/stores 等大依赖树时偶发进程级崩溃（无输出、退出码 1，吞掉 shell 输出），
    // forks 池稳定通过。2026-08-02 实测确认（WorkspaceHub.test.tsx 等）。
    pool: "forks",
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
    exclude: [],
    coverage: {
      provider: "v8",
      reporter: ["text", "lcov"],
      exclude: ["src/test/**", "src/**/*.test.*", "src/**/*.spec.*"],
    },
  },
}));
