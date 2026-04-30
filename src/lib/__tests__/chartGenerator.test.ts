import { describe, expect, it } from "vitest";

import { detectChartIntent } from "../chartGenerator";

describe("detectChartIntent", () => {
  it("detects Chinese chart intent with 图表", () => {
    const result = detectChartIntent("画一个销售趋势图表");
    expect(result).not.toBeNull();
    expect(result!.description).toContain("销售趋势");
  });

  it("detects Chinese chart intent with 图", () => {
    const result = detectChartIntent("生成一个柱状图");
    expect(result).not.toBeNull();
    expect(result!.chartType).toBe("bar");
  });

  it("detects chart intent with English keywords", () => {
    const result = detectChartIntent("show a chart of revenue");
    expect(result).not.toBeNull();
  });

  it("detects visualization intent", () => {
    const result = detectChartIntent("visualize the data");
    expect(result).not.toBeNull();
    expect(result!.description).toContain("the data");
  });

  it("infers line chart from 趋势 keyword", () => {
    const result = detectChartIntent("画一个趋势图显示用户增长");
    expect(result).not.toBeNull();
    expect(result!.chartType).toBe("line");
  });

  it("infers bar chart from 对比 keyword", () => {
    const result = detectChartIntent("做一个对比图表");
    expect(result).not.toBeNull();
    expect(result!.chartType).toBe("bar");
  });

  it("infers pie chart from 占比 keyword", () => {
    const result = detectChartIntent("显示各部门预算占比");
    expect(result).not.toBeNull();
    expect(result!.chartType).toBe("pie");
  });

  it("infers scatter chart from 散点 keyword", () => {
    const result = detectChartIntent("绘制散点图显示相关性");
    expect(result).not.toBeNull();
    expect(result!.chartType).toBe("scatter");
  });

  it("infers heatmap from 热力 keyword", () => {
    const result = detectChartIntent("生成热力图");
    expect(result).not.toBeNull();
    expect(result!.chartType).toBe("heatmap");
  });

  it("returns null for non-chart messages", () => {
    const result = detectChartIntent("今天天气怎么样？");
    expect(result).toBeNull();
  });

  it("returns null for plain text without chart context", () => {
    const result = detectChartIntent("Hello world");
    expect(result).toBeNull();
  });

  it("handles 占比 pattern with description", () => {
    const result = detectChartIntent("各部门预算占比图");
    expect(result).not.toBeNull();
    expect(result!.chartType).toBe("pie");
  });
});
