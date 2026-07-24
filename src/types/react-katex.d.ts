// SPDX-License-Identifier: AGPL-3.0-only

// react-katex 未内置 TypeScript 类型，这里声明最小可用类型
declare module "react-katex" {
  import type { ComponentType, ReactNode } from "react";

  // KaTeX 渲染失败时的回调，返回回退节点
  type RenderError = (error: Error) => ReactNode;

  // InlineMath / BlockMath 的公共 props
  interface KaTeXProps {
    math?: string;
    children?: string;
    errorColor?: string;
    renderError?: RenderError;
    settings?: Record<string, unknown>;
  }

  // 行内公式组件（$...$）
  export const InlineMath: ComponentType<KaTeXProps>;

  // 块级公式组件（$$...$$）
  export const BlockMath: ComponentType<KaTeXProps>;
}
