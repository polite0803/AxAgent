// Add all missing i18n keys (found by audit-i18n-keys.cjs / find-missing-keys.cjs)
// to en-US.json and zh-CN.json.
//
// Strategy:
// 1) Try to extract `defaultValue` from the code for `t(key, { defaultValue: "..." })` calls.
// 2) Otherwise, derive a sensible English value from the key itself
//    (Title-case the last segment, treat camelCase as word boundaries).
// 3) Provide a best-guess Chinese translation for keys that have well-known mappings;
//    for the rest, fall back to the English value with a "[ZH]" prefix so they
//    are easy to spot for native reviewers.
//
// Re-runnable: existing keys are NOT overwritten.

const fs = require("fs");
const path = require("path");

const ROOT = path.resolve(__dirname, "..");
const SRC = path.join(ROOT, "src");
const LOCALES_DIR = path.join(SRC, "i18n", "locales");
const enUSPath = path.join(LOCALES_DIR, "en-US.json");
const zhCNPath = path.join(LOCALES_DIR, "zh-CN.json");

const enUS = JSON.parse(fs.readFileSync(enUSPath, "utf8"));
const zhCN = JSON.parse(fs.readFileSync(zhCNPath, "utf8"));

function getAllKeys(obj, prefix = "") {
  const keys = [];
  for (const [k, v] of Object.entries(obj)) {
    const fk = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === "object" && !Array.isArray(v)) {
      keys.push(...getAllKeys(v, fk));
    } else {
      keys.push(fk);
    }
  }
  return keys;
}
const enDefined = new Set(getAllKeys(enUS));
const zhDefined = new Set(getAllKeys(zhCN));

// Walk source files
function walk(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === "node_modules" || entry.name === "dist" || entry.name === "build") continue;
    if (entry.name === "i18n" && dir === SRC) continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full, out);
    else if (/\.(tsx?|jsx?)$/.test(entry.name)) out.push(full);
  }
  return out;
}

// Find all keys used in code, with optional defaultValue
const PATTERNS = [
  [/(?<![\w$.])t\s*\(\s*(['"])([^'"\n]{2,}?)\1\s*(?:[,)])/g, 2],
  [/(?<![\w$.])t\s*\(\s*`([^`\n]{2,}?)`\s*(?:[,)])/g, 1],
  [/\b(?:i18next|i18n|translation)\s*\.\s*t\s*\(\s*(['"])([^'"\n]{2,}?)\1\s*(?:[,)])/g, 2],
  [/\b(?:i18next|i18n|translation)\s*\.\s*t\s*\(\s*`([^`\n]{2,}?)`\s*(?:[,)])/g, 1],
  [/i18nKey\s*=\s*(['"])([^'"\n]{2,}?)\1/g, 2],
];

const isDynamic = (key) =>
  /\$\{/.test(key) || /\+\s*['"`]/.test(key) || /\?\s*['"`]/.test(key);

// Multi-line t-call: t("foo", { defaultValue: "bar" })
// Match across continuation lines is complex; we just look at the same line.
const DEFAULT_VAL_RE = /defaultValue\s*:\s*(['"])([^'"\n]+?)\1/;

// Convert a key path to a sensible English value
function keyToEnglish(key) {
  const last = key.split(".").pop();
  if (!last) return key;
  // snake_case → words
  let s = last.replace(/_/g, " ");
  // camelCase / PascalCase → words
  s = s.replace(/([a-z])([A-Z])/g, "$1 $2");
  // Title case each word
  s = s.replace(/\b\w/g, (c) => c.toUpperCase());
  return s;
}

// Curated Chinese translations for high-confidence keys
const ZH_OVERRIDES = {
  "chat.error": "聊天出错",
  "chat.loadError": "加载失败",
  "input.uploadFile": "上传文件",
  "input.takePhoto": "拍照",
  "input.recordAudio": "录音",
  "nudge.noSuggestions": "暂无建议",
  "reportViewer.noReport": "暂无报告",
  "reportViewer.noOutline": "暂无大纲",
  "chatRightPanel.close": "关闭",
  "chat.collaboration.sessionShare.createFailed": "创建分享失败",
  "chat.collaboration.sessionShare.updateFailed": "更新分享失败",
  "subAgentCard.fork": "分叉",
  "userProfile.prefKey": "偏好键",
  "userProfile.prefValue": "偏好值",
  "gateway.cacheHitTokens": "缓存命中 Token 数",
  "settings.provider.deepseekBalance": "DeepSeek 余额",
  "settings.provider.noValidKey": "暂无可用密钥",
  "wiki.wiki.name": "知识库名称",
  "wiki.wiki.rootPath": "根路径",
  "wiki.wiki.description": "描述",
  "error": "错误",
  "chat.workflow.chart.renderError": "图表渲染失败",
  "skill.noSkillName": "技能未命名",
  "skill.notFound": "未找到技能",

  // workflow.aiPanel
  "workflow.aiPanel.promptAppliedToNode": "提示词已应用到节点",
  "workflow.aiPanel.chatWelcome": "你好，我是 AI 助手。",
  "workflow.aiPanel.chatWelcomeHint": "可以问我关于工作流或节点配置的问题。",
  "workflow.aiPanel.chatPlaceholder": "输入消息…",
  "workflow.aiPanel.replaceMode": "替换模式",
  "workflow.aiPanel.mergeMode": "合并模式",
  "workflow.aiPanel.generateMergeBtn": "生成合并结果",
  "workflow.aiPanel.retry": "重试",
  "workflow.aiPanel.fillFromSelectedNode": "从选中节点填充",
  "workflow.aiPanel.applyToNode": "应用到节点",
  "workflow.aiPanel.dragCanvasToAdd": "拖到画布添加",
  "workflow.aiPanel.dragHintUpdated": "已根据提示更新画布",
  "workflow.aiPanel.chatMode": "对话模式",
  "workflow.aiPanel.toolsMode": "工具模式",

  // workflow.debug
  "workflow.debug.colNode": "节点",
  "workflow.debug.orphan": "孤立",
  "workflow.debug.deadEnd": "死路",
  "workflow.debug.colIssues": "问题",
  "workflow.debug.noPrompt": "无提示",
  "workflow.debug.colType": "类型",
  "workflow.debug.colStatus": "状态",
  "workflow.debug.colTime": "时间",
  "workflow.debug.viewDetail": "查看详情",
  "workflow.debug.totalNodes": "节点总数",
  "workflow.debug.totalEdges": "连线总数",
  "workflow.debug.issuesFound": "发现问题",
  "workflow.debug.cyclesDetected": "检测到环路",
  "workflow.debug.nodeDiagnostics": "节点诊断",
  "workflow.debug.structuralValidation": "结构校验",
  "workflow.debug.runValidation": "运行校验",
  "workflow.debug.clickToValidate": "点击开始校验",
  "workflow.debug.errors": "错误",
  "workflow.debug.warnings": "警告",
  "workflow.debug.allClear": "一切正常",
  "workflow.debug.topoAnalysis": "拓扑分析",
  "workflow.debug.unreachableNodesCount": "不可达节点数",
  "workflow.debug.topoHealthy": "拓扑正常",
  "workflow.debug.runFullCheck": "运行完整检查",
  "workflow.debug.startDebug": "开始调试",
  "workflow.debug.pause": "暂停",
  "workflow.debug.resume": "继续",
  "workflow.debug.cancel": "取消",
  "workflow.debug.resumeBreakpoint": "从断点继续",
  "workflow.debug.continue": "继续",
  "workflow.debug.stepBreakpoint": "步过断点",
  "workflow.debug.step": "单步",
  "workflow.debug.execTime": "执行时间",
  "workflow.debug.nodesExecuted": "已执行节点数",
  "workflow.debug.breakpoints": "断点",
  "workflow.debug.nodeRecords": "节点记录",
  "workflow.debug.input": "输入",
  "workflow.debug.output": "输出",
  "workflow.debug.error": "错误",
  "workflow.debug.waitingForNodes": "等待节点",
  "workflow.debug.noRuntimeData": "暂无运行时数据",
  "workflow.debug.noVariables": "暂无变量",
  "workflow.debug.breakpointsPanel": "断点面板",
  "workflow.debug.noBreakpoints": "无断点",
  "workflow.debug.executionHistory": "执行历史",
  "workflow.debug.view": "查看",
  "workflow.debug.noHistory": "无历史",
  "workflow.debug.staticDebugHint": "点击节点进行静态调试",
  "workflow.debug.running": "运行中",
  "workflow.debug.staticCheck": "静态检查",
  "workflow.debug.runtimeTrace": "运行时追踪",
  "workflow.debug.viewSubExecution": "查看子执行",
  "workflow.debug.subExecutionDetail": "子执行详情",

  // workflow misc
  "workflow.noExecutionHistory": "无执行历史",
  "workflow.batchEditMode": "批量编辑模式",
  "workflow.nodeTypes.databaseQuery": "数据库查询",
  "workflow.delayNode.notConfigured": "延迟节点未配置",
  "workflow.loopNode.notConfigured": "循环节点未配置",
  "workflow.parallelNode.waitAll": "等待全部完成",
  "workflow.parallelNode.waitAny": "任一完成即可",
  "workflow.subWorkflowNode.inputCount": "输入参数数",
  "workflow.subWorkflowNode.outputCount": "输出参数数",
  "workflow.nodeTypes.switch": "分支",
  "workflow.validationNode.rules": "校验规则",

  // workflow.props
  "workflow.props.retryEnabled": "启用重试",
  "workflow.props.promptSaved": "提示词已保存",
  "workflow.props.promptSaveFailed": "提示词保存失败",
  "workflow.props.editExpertPrompt": "编辑专家提示词",
  "workflow.props.editRolePrompt": "编辑角色提示词",
  "workflow.props.aggregationAny": "任意结果",
  "workflow.props.maxDelayMs": "最大延迟（毫秒）",
  "workflow.props.resetTimeoutHint": "重置超时",
  "workflow.props.aiGenerate": "AI 生成",
  "workflow.props.codeTemplates": "代码模板",
  "workflow.props.hideTools": "隐藏工具",
  "workflow.props.showTools": "显示工具",
  "workflow.props.availableTools": "可用工具",
  "workflow.props.noToolsAvailable": "暂无可用工具",
  "workflow.props.llmRouting": "LLM 路由",
  "workflow.props.routingPrompt": "路由提示词",
  "workflow.props.routingPromptPlaceholder": "描述路由判断逻辑",
  "workflow.props.routingModel": "路由模型",
  "workflow.props.routingModelPlaceholder": "选择路由模型",
  "workflow.props.connection": "数据源",
  "workflow.props.defaultConnection": "默认数据源",
  "workflow.props.to": "收件人",
  "workflow.props.subject": "主题",
  "workflow.props.autoInputsFromBranches": "从分支自动汇聚输入",
  "workflow.props.autoInputsFromBranchesHint": "启用后，分支的输出将自动作为子节点输入。",
  "workflow.props.mergeAny": "任一完成",
  "workflow.props.mergeRace": "竞速",
  "workflow.props.mergeMajority": "多数一致",
  "workflow.props.connectInputsManually": "手动连接输入",
  "workflow.props.autoInputFromParent": "从父节点自动获取输入",
  "workflow.props.autoInputFromParentHint": "启用后，父节点的输出将自动作为子节点输入。",
  "workflow.props.matchMode": "匹配模式",
  "workflow.props.cases": "分支列表",
  "workflow.props.addCase": "添加分支",

  // workflow.nodeConfig (snake_case keys in code)
  "workflow.nodeConfig.topic_var": "主题变量",
  "workflow.nodeConfig.max_rounds": "最大轮数",
  "workflow.nodeConfig.debaters": "辩手列表",
  "workflow.nodeConfig.add_debater": "添加辩手",
  "workflow.nodeConfig.convergence_prompt": "收敛提示词",
  "workflow.nodeConfig.convergence_prompt_placeholder": "用于判断辩论何时收敛的提示词",
  "workflow.nodeConfig.convergence_model_role": "收敛模型角色",

  // workflow.aiAssist
  "workflow.aiAssist.dbQuery.sqlHint": "提示：可输入 SQL，由 AI 辅助补全。",
  "workflow.aiAssist.loop.continueHint": "可输入循环继续条件，如 `index < 10`。",
  "workflow.aiAssist.parallel.branchesHint": "描述并行分支的输入与目标输出。",
  "workflow.aiAssist.subWorkflow.parseFailed": "解析失败，请检查描述。",
  "workflow.aiAssist.subWorkflow.needPick": "请先选择一个已有工作流或描述新工作流。",
  "workflow.aiAssist.switch.casesHint": "列出所有分支及其触发条件。",
  "workflow.aiAssist.trigger.cronHint": "支持标准 cron 表达式，如 `0 9 * * *`。",

  // workflow.versionHistory
  "workflow.versionHistory.compareVersions": "对比版本",
  "workflow.versionHistory.compare": "对比",
  "workflow.versionHistory.noChanges": "无差异",

  // workflow.diagnostic / search
  "workflow.diagnostic.error": "诊断错误",
  "workflow.searchNodes": "搜索节点",

  // EN counterparts for the curated ZH keys
  "chat.error": "Chat error",
  "chat.loadError": "Failed to load",
  "input.uploadFile": "Upload file",
  "input.takePhoto": "Take photo",
  "input.recordAudio": "Record audio",
  "nudge.noSuggestions": "No suggestions",
  "reportViewer.noReport": "No report",
  "reportViewer.noOutline": "No outline",
  "chatRightPanel.close": "Close",
  "chat.collaboration.sessionShare.createFailed": "Failed to create share",
  "chat.collaboration.sessionShare.updateFailed": "Failed to update share",
  "subAgentCard.fork": "Fork",
  "userProfile.prefKey": "Preference Key",
  "userProfile.prefValue": "Preference Value",
  "gateway.cacheHitTokens": "Cache hit tokens",
  "settings.provider.deepseekBalance": "DeepSeek balance",
  "settings.provider.noValidKey": "No valid key",
  "wiki.wiki.name": "Wiki name",
  "wiki.wiki.rootPath": "Root path",
  "wiki.wiki.description": "Description",
  "error": "Error",
  "chat.workflow.chart.renderError": "Chart render error",
  "skill.noSkillName": "Unnamed skill",
  "skill.notFound": "Skill not found",
  "workflow.aiPanel.promptAppliedToNode": "Prompt applied to node",
  "workflow.aiPanel.chatWelcome": "Hi, I'm your AI assistant.",
  "workflow.aiPanel.chatWelcomeHint": "Ask me about workflow or node configuration.",
  "workflow.aiPanel.chatPlaceholder": "Type a message…",
  "workflow.aiPanel.replaceMode": "Replace mode",
  "workflow.aiPanel.mergeMode": "Merge mode",
  "workflow.aiPanel.generateMergeBtn": "Generate merge",
  "workflow.aiPanel.retry": "Retry",
  "workflow.aiPanel.fillFromSelectedNode": "Fill from selected node",
  "workflow.aiPanel.applyToNode": "Apply to node",
  "workflow.aiPanel.dragCanvasToAdd": "Drag to canvas to add",
  "workflow.aiPanel.dragHintUpdated": "Canvas updated from prompt",
  "workflow.aiPanel.chatMode": "Chat mode",
  "workflow.aiPanel.toolsMode": "Tools mode",
  "workflow.debug.colNode": "Node",
  "workflow.debug.orphan": "Orphan",
  "workflow.debug.deadEnd": "Dead end",
  "workflow.debug.colIssues": "Issues",
  "workflow.debug.noPrompt": "No prompt",
  "workflow.debug.colType": "Type",
  "workflow.debug.colStatus": "Status",
  "workflow.debug.colTime": "Time",
  "workflow.debug.viewDetail": "View detail",
  "workflow.debug.totalNodes": "Total nodes",
  "workflow.debug.totalEdges": "Total edges",
  "workflow.debug.issuesFound": "Issues found",
  "workflow.debug.cyclesDetected": "Cycles detected",
  "workflow.debug.nodeDiagnostics": "Node diagnostics",
  "workflow.debug.structuralValidation": "Structural validation",
  "workflow.debug.runValidation": "Run validation",
  "workflow.debug.clickToValidate": "Click to validate",
  "workflow.debug.errors": "Errors",
  "workflow.debug.warnings": "Warnings",
  "workflow.debug.allClear": "All clear",
  "workflow.debug.topoAnalysis": "Topo analysis",
  "workflow.debug.unreachableNodesCount": "Unreachable nodes",
  "workflow.debug.topoHealthy": "Topology healthy",
  "workflow.debug.runFullCheck": "Run full check",
  "workflow.debug.startDebug": "Start debug",
  "workflow.debug.pause": "Pause",
  "workflow.debug.resume": "Resume",
  "workflow.debug.cancel": "Cancel",
  "workflow.debug.resumeBreakpoint": "Resume breakpoint",
  "workflow.debug.continue": "Continue",
  "workflow.debug.stepBreakpoint": "Step breakpoint",
  "workflow.debug.step": "Step",
  "workflow.debug.execTime": "Execution time",
  "workflow.debug.nodesExecuted": "Nodes executed",
  "workflow.debug.breakpoints": "Breakpoints",
  "workflow.debug.nodeRecords": "Node records",
  "workflow.debug.input": "Input",
  "workflow.debug.output": "Output",
  "workflow.debug.error": "Error",
  "workflow.debug.waitingForNodes": "Waiting for nodes",
  "workflow.debug.noRuntimeData": "No runtime data",
  "workflow.debug.noVariables": "No variables",
  "workflow.debug.breakpointsPanel": "Breakpoints panel",
  "workflow.debug.noBreakpoints": "No breakpoints",
  "workflow.debug.executionHistory": "Execution history",
  "workflow.debug.view": "View",
  "workflow.debug.noHistory": "No history",
  "workflow.debug.staticDebugHint": "Click a node to debug",
  "workflow.debug.running": "Running",
  "workflow.debug.staticCheck": "Static check",
  "workflow.debug.runtimeTrace": "Runtime trace",
  "workflow.debug.viewSubExecution": "View sub-execution",
  "workflow.debug.subExecutionDetail": "Sub-execution detail",
  "workflow.noExecutionHistory": "No execution history",
  "workflow.batchEditMode": "Batch edit mode",
  "workflow.nodeTypes.databaseQuery": "Database Query",
  "workflow.delayNode.notConfigured": "Delay node not configured",
  "workflow.loopNode.notConfigured": "Loop node not configured",
  "workflow.parallelNode.waitAll": "Wait for all",
  "workflow.parallelNode.waitAny": "Wait for any",
  "workflow.subWorkflowNode.inputCount": "Input count",
  "workflow.subWorkflowNode.outputCount": "Output count",
  "workflow.nodeTypes.switch": "Switch",
  "workflow.validationNode.rules": "Validation rules",
  "workflow.props.retryEnabled": "Enable retry",
  "workflow.props.promptSaved": "Prompt saved",
  "workflow.props.promptSaveFailed": "Failed to save prompt",
  "workflow.props.editExpertPrompt": "Edit expert prompt",
  "workflow.props.editRolePrompt": "Edit role prompt",
  "workflow.props.aggregationAny": "Any result",
  "workflow.props.maxDelayMs": "Max delay (ms)",
  "workflow.props.resetTimeoutHint": "Reset timeout",
  "workflow.props.aiGenerate": "AI Generate",
  "workflow.props.codeTemplates": "Code templates",
  "workflow.props.hideTools": "Hide tools",
  "workflow.props.showTools": "Show tools",
  "workflow.props.availableTools": "Available tools",
  "workflow.props.noToolsAvailable": "No tools available",
  "workflow.props.llmRouting": "LLM Routing",
  "workflow.props.routingPrompt": "Routing prompt",
  "workflow.props.routingPromptPlaceholder": "Describe the routing decision logic",
  "workflow.props.routingModel": "Routing model",
  "workflow.props.routingModelPlaceholder": "Select routing model",
  "workflow.props.connection": "Data source",
  "workflow.props.defaultConnection": "Default data source",
  "workflow.props.to": "To",
  "workflow.props.subject": "Subject",
  "workflow.props.autoInputsFromBranches": "Auto inputs from branches",
  "workflow.props.autoInputsFromBranchesHint": "When enabled, branch outputs are auto-injected as child inputs.",
  "workflow.props.mergeAny": "Merge any",
  "workflow.props.mergeRace": "Merge race",
  "workflow.props.mergeMajority": "Merge majority",
  "workflow.props.connectInputsManually": "Connect inputs manually",
  "workflow.props.autoInputFromParent": "Auto input from parent",
  "workflow.props.autoInputFromParentHint": "When enabled, parent outputs are auto-injected as child inputs.",
  "workflow.props.matchMode": "Match mode",
  "workflow.props.cases": "Cases",
  "workflow.props.addCase": "Add case",
  "workflow.nodeConfig.topic_var": "Topic variable",
  "workflow.nodeConfig.max_rounds": "Max rounds",
  "workflow.nodeConfig.debaters": "Debaters",
  "workflow.nodeConfig.add_debater": "Add debater",
  "workflow.nodeConfig.convergence_prompt": "Convergence prompt",
  "workflow.nodeConfig.convergence_prompt_placeholder": "Prompt used to decide when the debate has converged",
  "workflow.nodeConfig.convergence_model_role": "Convergence model role",
  "workflow.aiAssist.dbQuery.sqlHint": "Tip: type SQL and AI will help complete it.",
  "workflow.aiAssist.loop.continueHint": "Provide a loop continue condition, e.g. `index < 10`.",
  "workflow.aiAssist.parallel.branchesHint": "Describe parallel branch inputs and the target output.",
  "workflow.aiAssist.subWorkflow.parseFailed": "Failed to parse; please check the description.",
  "workflow.aiAssist.subWorkflow.needPick": "Please pick an existing workflow or describe a new one.",
  "workflow.aiAssist.switch.casesHint": "List all cases and their trigger conditions.",
  "workflow.aiAssist.trigger.cronHint": "Standard cron expression, e.g. `0 9 * * *`.",
  "workflow.versionHistory.compareVersions": "Compare versions",
  "workflow.versionHistory.compare": "Compare",
  "workflow.versionHistory.noChanges": "No changes",
  "workflow.diagnostic.error": "Diagnostic error",
  "workflow.searchNodes": "Search nodes",
};

const used = new Map(); // key -> defaultValue (or null)

for (const file of walk(SRC)) {
  const text = fs.readFileSync(file, "utf8");
  const lines = text.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (/^\s*(\/\/|\*|\/\*)/.test(line)) continue;
    for (const [re, keyIdx] of PATTERNS) {
      re.lastIndex = 0;
      let m;
      while ((m = re.exec(line)) !== null) {
        const key = m[keyIdx];
        if (!key || isDynamic(key) || !key.includes(".")) continue;
        if (!used.has(key)) used.set(key, null);
        // Try to extract defaultValue from this call
        const dv = line.match(DEFAULT_VAL_RE);
        if (dv && dv[2]) {
          used.set(key, dv[2]);
        }
      }
    }
  }
}

const missing = [...used.keys()].filter((k) => !enDefined.has(k)).sort();

function setNested(obj, dottedKey, value) {
  const parts = dottedKey.split(".");
  let cur = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    if (!cur[parts[i]] || typeof cur[parts[i]] !== "object") cur[parts[i]] = {};
    cur = cur[parts[i]];
  }
  cur[parts[parts.length - 1]] = value;
}

let enAdded = 0, zhAdded = 0;
const enFallbacks = [];
const zhFallbacks = [];

for (const key of missing) {
  if (!enDefined.has(key)) {
    let enVal = ZH_OVERRIDES[key] || used.get(key) || keyToEnglish(key);
    setNested(enUS, key, enVal);
    enDefined.add(key);
    enAdded++;
    if (!ZH_OVERRIDES[key] && !used.get(key)) {
      enFallbacks.push(key);
    }
  }
  if (!zhDefined.has(key)) {
    let zhVal = ZH_OVERRIDES[key] || used.get(key) || enUS[getRootFromKey(enUS, key)] || keyToEnglish(key);
    setNested(zhCN, key, zhVal);
    zhDefined.add(key);
    zhAdded++;
    if (!ZH_OVERRIDES[key] && !used.get(key)) {
      zhFallbacks.push(key);
    }
  }
}

// Try to be helpful: if there's no ZH override but en-US has a reasonable value
// and the value looks like a simple word/phrase, use it as the zh value too.
function getRootFromKey(obj, key) {
  const parts = key.split(".");
  let cur = obj;
  for (const p of parts) {
    if (cur && typeof cur === "object" && p in cur) cur = cur[p];
    else return null;
  }
  return cur;
}

fs.writeFileSync(enUSPath, JSON.stringify(enUS, null, 2) + "\n");
fs.writeFileSync(zhCNPath, JSON.stringify(zhCN, null, 2) + "\n");

console.log(`Missing keys: ${missing.length}`);
console.log(`en-US: added ${enAdded}`);
console.log(`zh-CN: added ${zhAdded}`);
console.log("");
if (enFallbacks.length) {
  console.log("en-US fallbacks (key → derived Title Case):");
  enFallbacks.forEach((k) => console.log("  -", k, "→", keyToEnglish(k)));
  console.log("");
}
if (zhFallbacks.length) {
  console.log("zh-CN fallbacks (no override / defaultValue; using EN value as placeholder):");
  zhFallbacks.forEach((k) => console.log("  -", k));
}
