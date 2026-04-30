import { describe, expect, it } from "vitest";

import { composeHtml, isChartOption } from "../htmlParser";

describe("composeHtml", () => {
  it("wraps HTML with boilerplate structure", () => {
    const result = composeHtml({ html: "<p>Hello</p>" });
    expect(result).toContain("<!DOCTYPE html>");
    expect(result).toContain("<p>Hello</p>");
    expect(result).toContain("</html>");
  });

  it("injects CSS into style tag", () => {
    const result = composeHtml({
      html: "<div>Content</div>",
      css: ".custom { color: red; }",
    });
    expect(result).toContain(".custom { color: red; }");
  });

  it("injects JS into script tag with error handling", () => {
    const result = composeHtml({
      html: "<div>Content</div>",
      js: "console.log('hello')",
    });
    expect(result).toContain("console.log('hello')");
  });

  it("produces valid full HTML document", () => {
    const result = composeHtml({
      html: "<h1>Title</h1>",
      css: "h1 { font-size: 24px; }",
      js: "alert('hi')",
    });
    expect(result).toContain("<h1>Title</h1>");
    expect(result).toContain("h1 { font-size: 24px; }");
    expect(result).toContain("alert('hi')");
  });

  it("handles empty parts gracefully", () => {
    const result = composeHtml({});
    expect(result).toContain("<!DOCTYPE html>");
    expect(result).toContain("<body>");
    expect(result).toContain("</body>");
  });

  it("handles only HTML part", () => {
    const result = composeHtml({ html: "<span>Text</span>" });
    expect(result).toContain("<span>Text</span>");
  });
});

describe("isChartOption", () => {
  it("returns true for ECharts option with series", () => {
    const result = isChartOption(JSON.stringify({
      series: [{ type: "line", data: [1, 2, 3] }],
    }));
    expect(result).toBe(true);
  });

  it("returns true for ECharts option with xAxis", () => {
    const result = isChartOption(JSON.stringify({
      xAxis: { type: "category" },
    }));
    expect(result).toBe(true);
  });

  it("returns true for ECharts option with yAxis", () => {
    const result = isChartOption(JSON.stringify({
      yAxis: { type: "value" },
    }));
    expect(result).toBe(true);
  });

  it("returns true for ECharts polar chart", () => {
    const result = isChartOption(JSON.stringify({
      polar: {},
      radiusAxis: {},
    }));
    expect(result).toBe(true);
  });

  it("returns false for plain object", () => {
    const result = isChartOption(JSON.stringify({ name: "test", value: 42 }));
    expect(result).toBe(false);
  });

  it("returns false for invalid JSON", () => {
    const result = isChartOption("not json at all");
    expect(result).toBe(false);
  });

  it("returns false for empty object", () => {
    const result = isChartOption("{}");
    expect(result).toBe(false);
  });
});
