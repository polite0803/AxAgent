// SPDX-License-Identifier: AGPL-3.0-only

import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { LatexRenderer, splitContentWithLatex } from "../LatexRenderer";

describe("splitContentWithLatex", () => {
  it("无 $ 时返回单段文本（快速路径）", () => {
    const segments = splitContentWithLatex("普通文本无公式");
    expect(segments).toEqual([{ type: "text", content: "普通文本无公式" }]);
  });

  it("解析行内公式 $E=mc^2$", () => {
    const segments = splitContentWithLatex("质能方程 $E=mc^2$ 很有名");
    expect(segments).toEqual([
      { type: "text", content: "质能方程 " },
      { type: "inline-math", content: "E=mc^2" },
      { type: "text", content: " 很有名" },
    ]);
  });

  it("解析块级公式 $$\\int_0^1 x^2 dx$$", () => {
    const segments = splitContentWithLatex("积分 $$\\int_0^1 x^2 dx$$ 结果");
    expect(segments).toEqual([
      { type: "text", content: "积分 " },
      { type: "block-math", content: "\\int_0^1 x^2 dx" },
      { type: "text", content: " 结果" },
    ]);
  });

  it("块级公式优先于行内（$$...$$ 不被拆成两个行内公式）", () => {
    const segments = splitContentWithLatex("$$a + b$$");
    expect(segments).toEqual([
      { type: "block-math", content: "a + b" },
    ]);
  });

  it("代码块内的 $ 不被解析", () => {
    const code = "```\nconst price = '$5';\n```";
    const segments = splitContentWithLatex(code);
    // 整个代码块作为一段文本，内部 $5 不被当作公式
    expect(segments).toEqual([{ type: "text", content: code }]);
  });

  it("行内代码内的 $ 不被解析", () => {
    const segments = splitContentWithLatex("价格 `$5` 很便宜");
    expect(segments).toEqual([
      { type: "text", content: "价格 `$5` 很便宜" },
    ]);
  });

  it("转义 \\$ 不被当作公式分隔符", () => {
    const segments = splitContentWithLatex("价格 \\$5 和 \\$3");
    expect(segments).toEqual([
      { type: "text", content: "价格 \\$5 和 \\$3" },
    ]);
  });

  it("混合文本、行内、块级公式", () => {
    const segments = splitContentWithLatex("行内 $a$ 块级\n$$b$$\n尾部");
    expect(segments).toEqual([
      { type: "text", content: "行内 " },
      { type: "inline-math", content: "a" },
      { type: "text", content: " 块级\n" },
      { type: "block-math", content: "b" },
      { type: "text", content: "\n尾部" },
    ]);
  });

  it("行内公式不跨行（$a\\nb$ 不被匹配为公式）", () => {
    const segments = splitContentWithLatex("$a\nb$");
    expect(segments).toEqual([{ type: "text", content: "$a\nb$" }]);
  });
});

describe("LatexRenderer", () => {
  it("行内公式渲染成功生成 katex 节点", () => {
    const { container } = render(<LatexRenderer content="E=mc^2" />);
    expect(container.querySelector(".katex")).not.toBeNull();
  });

  it("块级公式渲染成功生成 katex-display 节点", () => {
    const { container } = render(
      <LatexRenderer content="\\int_0^1 x^2 dx" displayMode />,
    );
    expect(container.querySelector(".katex-display")).not.toBeNull();
  });

  it("无效 LaTeX 回退显示原始文本（红色边框）", () => {
    // \frac{1 未闭合，katex 会抛 ParseError
    const content = "\\frac{1";
    const { container } = render(<LatexRenderer content={content} />);
    const errorSpan = container.querySelector("span[title='LaTeX 解析失败']");
    expect(errorSpan).not.toBeNull();
    // 调试：确认 textContent 与 content 一致（React 不会转义反斜杠）
    expect(errorSpan?.textContent).toBe(content);
  });
});
