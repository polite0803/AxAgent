const fs = require("fs");

// 替换表：中文 → t() 调用
// 第一个参数：匹配的字符串（原始中文）
// 第二个参数：替换结果
// 第三个参数：是否精准匹配（避免误伤）
const rules = [
  // getNodeTypeLabel 函数（独立的）
  [/return "分析师"/g, 'return t("stockAnalysis.workflow.analyst")'],
  [/return "风控"/g, 'return t("stockAnalysis.riskControl")'],
  [/return "多方辩论"/g, 'return t("stockAnalysis.bullDebate")'],
  [/return "空方辩论"/g, 'return t("stockAnalysis.bearDebate")'],
  [/return "数据工具"/g, 'return t("stockAnalysis.dataTool")'],
  [/return "辩论收敛"/g, 'return t("stockAnalysis.debateConvergence")'],
  [/return "决策引擎"/g, 'return t("stockAnalysis.decisionEngine")'],
  [/return "规则检查"/g, 'return t("stockAnalysis.ruleCheck")'],
  [/return "其他"/g, 'return t("stockAnalysis.other")'],
];

// 批量替换
let files = ["src/components/stock-analysis/AnalysisDebugPanel.tsx"];
for (const f of files) {
  let code = fs.readFileSync(f, "utf8");
  let count = 0;
  for (const [re, to] of rules) {
    const before = code;
    code = code.replace(re, to);
    if (code !== before) { count++; }
  }
  fs.writeFileSync(f, code);
  console.log(f + ": " + count + " replacements");
}
