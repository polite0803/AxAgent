const fs = require("fs");
let code = fs.readFileSync("src/components/stock-analysis/DebatePanel.tsx", "utf8");

const replacements = [
  ["置信度 ", '{t("stockAnalysis.debate.confidence")} '],
  ["强度: ", '{t("stockAnalysis.debate.strength")}: '],
  ["最终立场:", '{t("stockAnalysis.debate.finalPosition")}:'],
  ["逐条回应 R2 质询", '{t("stockAnalysis.debate.r2Responses")}'],
  ["质询 ", '{t("stockAnalysis.debate.challenge")} '],
  ["针对: ", '{t("stockAnalysis.debate.target")}: '],
  ["修正:", '{t("stockAnalysis.debate.concession")}:'],
  ["强化保留论据", '{t("stockAnalysis.debate.retainedArguments")}'],
  ["最终强度:", '{t("stockAnalysis.debate.finalStrength")}:'],
  ["补充证据: ", '{t("stockAnalysis.debate.additionalEvidence")}: '],
  ["仍未解决的数据缺口: ", '{t("stockAnalysis.debate.unresolvedGaps")}: '],
  ["最终裁决:", '{t("stockAnalysis.debate.finalVerdict")}:'],
  ["立场:", '{t("stockAnalysis.debate.position")}:'],
  ["置信度:", '{t("stockAnalysis.debate.confidence")}:'],
  ["暂无裁决说明", '{t("stockAnalysis.debate.noVerdict")}'],
  ["裁决理由", '{t("stockAnalysis.debate.verdictReason")}'],
  ["展开全文", '{t("stockAnalysis.debate.expandFull")}'],
  ["反驳预防", '{t("stockAnalysis.debate.counterArgument")}'],
  ["核心论点", '{t("stockAnalysis.debate.coreArguments")}'],
  ["多维度共振", '{t("stockAnalysis.debate.multiDimResonance")}'],
  ["引用: ", '{t("stockAnalysis.debate.references")}: '],
  ["权重:", '{t("stockAnalysis.debate.weight")}:'],
  ["空方攻击:", '{t("stockAnalysis.debate.bearAttack")}:'],
  ["我方回应:", '{t("stockAnalysis.debate.ourResponse")}:'],
  ["看多强度: ", '{t("stockAnalysis.debate.bullStrength")}: '],
  ["看空强度: ", '{t("stockAnalysis.debate.bearStrength")}: '],
  ["数据缺口: ", '{t("stockAnalysis.debate.dataGaps")}: '],
  ["收敛总结:", '{t("stockAnalysis.debate.convergenceSummary")}:'],
  ["共振点", '{t("stockAnalysis.debate.resonancePoint")}'],
  ["待定", '{t("stockAnalysis.debate.pending")}'],
  ["论据 ", '{t("stockAnalysis.debate.argument")} '],
  ["论点 ", '{t("stockAnalysis.debate.argument")} '],
  ["未命名论点", '{t("stockAnalysis.debate.unnamedArgument")}'],
];

let count = 0;
for (const [from, to] of replacements) {
  const re = new RegExp(from.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "g");
  const before = code;
  code = code.replace(re, to);
  if (code !== before) { count++; }
}

// Remove STANCE_LABEL constant and replace with getStanceLabel calls
code = code.replace(/const STANCE_LABEL.*?^};/ms, "");
// Update call site
code = code.replace(/STANCE_LABEL\[/g, "getStanceLabel(");
// Fix the call to add t parameter
code = code.replace(/getStanceLabel\(([^)]+)\)/g, (m, args) => {
  if (args.includes(", t")) { return m; }
  return `getStanceLabel(${args.trim()}, t)`;
});

fs.writeFileSync("src/components/stock-analysis/DebatePanel.tsx", code);
console.log("Applied", count, "text replacements");
