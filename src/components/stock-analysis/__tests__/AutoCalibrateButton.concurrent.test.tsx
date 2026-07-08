// 单元测试: R2-Bug-X — AutoCalibrateButton 必须防止并发点击
//
// 触发: handlePreview / handleApply 缺少 setApplying/applying 检查,极快双击
// 会触发并发 apply_reco_weights/preview_adjust_reco_weights 请求。

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

describe("AutoCalibrateButton — R2-Bug-X: 防并发点击", () => {
  it("handlePreview 函数体第一行必须有 applying/loading 检查", () => {
    const projectRoot = process.cwd();
    const srcPath = resolve(
      projectRoot,
      "src/components/stock-analysis/RecommendationPanel.tsx",
    );
    const src = readFileSync(srcPath, "utf8");

    // 1. handlePreview 必须有 if (loading) return; 守卫
    const previewIdx = src.indexOf("const handlePreview");
    expect(previewIdx).toBeGreaterThan(0);
    const previewBody = src.slice(previewIdx, previewIdx + 200);
    expect(previewBody).toMatch(/if\s*\(\s*loading\s*\)\s*{\s*return;\s*}/);

    // 2. handleApply 必须有 if (applying) return; 守卫(防双击)
    const applyIdx = src.indexOf("const handleApply");
    expect(applyIdx).toBeGreaterThan(0);
    const applyBody = src.slice(applyIdx, applyIdx + 200);
    expect(applyBody).toMatch(/if\s*\(\s*applying\s*\)\s*{\s*return;\s*}/);

    // 3. 校准按钮必须显式 disabled={loading}(不只 loading,确保 JSDOM 下也安全)
    // disabled 写在 Button 标签的 props 里(在校准文本之前),从 Button 标签起点往后查
    const btnStart = src.lastIndexOf("<Button", src.indexOf('⚡ {t("stockAnalysis.recommendation.calibrate")'));
    expect(btnStart).toBeGreaterThan(0);
    const btnEnd = src.indexOf("</Button>", btnStart);
    const btnCode = src.slice(btnStart, btnEnd);
    expect(btnCode).toMatch(/disabled=\{loading\}/);

    // 4. 应用按钮必须显式 disabled={applying}
    const applyBtnStart = src.lastIndexOf("<Button", src.indexOf("应用选中项"));
    expect(applyBtnStart).toBeGreaterThan(0);
    const applyBtnEnd = src.indexOf("</Button>", applyBtnStart);
    const applyBtnCode = src.slice(applyBtnStart, applyBtnEnd);
    expect(applyBtnCode).toMatch(/disabled=\{applying/);
  });
});
