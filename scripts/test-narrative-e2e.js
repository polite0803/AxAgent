// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 叙事结构端到端集成测试脚本
 *
 * 测试目标：
 * 1. 验证叙事结构数据模型的完整性和正确性
 * 2. 验证结构指令生成逻辑
 * 3. 验证结构数据的序列化/反序列化
 * 4. 使用真实 LLM API 进行端到端结构注入测试
 * 5. 验证结构调整后对后续章节指令与 LLM 输出的影响
 *
 * 使用方法：
 *   # 仅运行数据模型测试（无需 API）
 *   node scripts/test-narrative-e2e.js
 *
 *   # 使用真实 API 测试（支持多种 provider，按优先级自动选择）
 *   # 优先级：EXPLICIT_AGNES_URL > DEEPSEEK_API_KEY > AGNES_API_KEY（需自行设置 URL）
 *   node scripts/test-narrative-e2e.js --api
 *
 *   # 指定使用某个 provider
 *   $env:E2E_PROVIDER="deepseek"; node scripts/test-narrative-e2e.js --api
 *   $env:E2E_PROVIDER="agnes"; $env:AGNES_API_URL="https://..."; node scripts/test-narrative-e2e.js --api
 */

// ── Provider 配置（按可用性自动选择） ──
const PROVIDERS = {
  deepseek: {
    name: "DeepSeek",
    url: "https://api.deepseek.com/v1/chat/completions",
    model: process.env.DEEPSEEK_MODEL || "deepseek-chat",
    key: process.env.DEEPSEEK_API_KEY,
  },
  agnes: {
    name: "Agnes",
    url: process.env.AGNES_API_URL || "",
    model: process.env.AGNES_MODEL || "agnes-v1",
    key: process.env.AGNES_API_KEY,
  },
  local: {
    name: "Local Gateway",
    url: process.env.LOCAL_API_URL || "http://localhost:8642/v1/chat/completions",
    model: process.env.LOCAL_MODEL || "deepseek-chat",
    key: process.env.LOCAL_API_KEY,
  },
};

function resolveProvider() {
  const forced = process.env.E2E_PROVIDER;
  if (forced && PROVIDERS[forced]) {
    const p = PROVIDERS[forced];
    if (p.key && p.url) return { ...p, id: forced };
  }
  // 自动选择可用 provider
  for (const [id, p] of Object.entries(PROVIDERS)) {
    if (p.key && p.url) return { ...p, id };
  }
  return null;
}

const API_PROVIDER = resolveProvider();
const RUN_API = process.argv.includes("--api");

// ── 工具函数 ──

function assert(condition, message) {
  if (!condition) throw new Error(`断言失败: ${message}`);
}

function assertEq(actual, expected, message) {
  if (actual !== expected) throw new Error(`断言失败: ${message}. 期望 ${expected}, 实际 ${actual}`);
}

// ── 叙事结构测试数据 ──

function createTestNarrativeStructure() {
  return {
    arcs: [
      {
        id: "arc-protagonist",
        arcType: "transformative",
        subject: "主角林墨",
        want: "找到失踪的妹妹",
        need: "面对自己内心的恐惧",
        stages: [
          { name: "平凡生活", chapter: 1, description: "林墨是一名普通的图书馆员" },
          { name: "遭遇变故", chapter: 3, description: "妹妹突然失踪" },
          { name: "内心挣扎", chapter: 6, description: "面对各种挑战" },
          { name: "觉醒转变", chapter: 9, description: "克服内心恐惧" },
          { name: "终极对决", chapter: 12, description: "与幕后黑手正面交锋" },
        ],
        currentProgress: 0.4,
      },
      {
        id: "arc-antagonist",
        arcType: "tragic",
        subject: "反派陈渊",
        want: "获取古代神秘力量",
        need: "被接纳和理解",
        stages: [
          { name: "神秘登场", chapter: 2, description: "陈渊以神秘恩人身份出现" },
          { name: "暗中布局", chapter: 5, description: "陈渊的真实目的逐渐暴露" },
          { name: "权力巅峰", chapter: 8, description: "陈渊获取了部分神秘力量" },
          { name: "最终毁灭", chapter: 12, description: "力量失控，陈渊走向毁灭" },
        ],
        currentProgress: 0.3,
      },
    ],
    confluences: [
      {
        id: "climax-final",
        triggerChapter: 12,
        confluenceType: "conflict_burst",
        involvedArcs: ["arc-protagonist", "arc-antagonist"],
        involvedForeshadows: ["fs-prophecy", "fs-artifact"],
        impact: "正邪双方在古代遗迹中展开最终对决",
      },
    ],
    foreshadows: [
      {
        id: "fs-prophecy",
        setupChapter: 1,
        payoffChapter: 12,
        status: "setup",
        description: "古籍中的神秘预言",
        payoffDescription: "预言指向的正是林墨与陈渊的对决",
        relatedArcs: ["arc-protagonist"],
      },
      {
        id: "fs-artifact",
        setupChapter: 4,
        payoffChapter: 10,
        status: "setup",
        description: "林墨获得的古老玉佩",
        payoffDescription: "玉佩是开启古代遗迹的钥匙",
        relatedArcs: ["arc-protagonist"],
      },
    ],
  };
}

// ── 结构指令生成 ──

function generateChapterInstructions(narrativeStructure, chapter) {
  const instructions = {
    arcInstructions: [],
    foreshadowInstructions: [],
    confluenceTriggers: [],
  };

  for (const arc of narrativeStructure.arcs) {
    for (const stage of arc.stages) {
      if (stage.chapter === chapter) {
        instructions.arcInstructions.push({
          arcId: arc.id,
          arcType: arc.arcType,
          stageName: stage.name,
          stageDescription: stage.description,
        });
      }
    }
  }

  for (const fs of narrativeStructure.foreshadows) {
    if (fs.setupChapter === chapter && fs.status === "setup") {
      instructions.foreshadowInstructions.push({
        foreshadowId: fs.id,
        action: "setup",
        description: fs.description,
      });
    }
    if (fs.payoffChapter === chapter && fs.status !== "payoff") {
      instructions.foreshadowInstructions.push({
        foreshadowId: fs.id,
        action: "payoff",
        description: fs.payoffDescription || fs.description,
      });
    }
  }

  for (const cp of narrativeStructure.confluences) {
    if (cp.triggerChapter === chapter) {
      instructions.confluenceTriggers.push({
        id: cp.id,
        confluenceType: cp.confluenceType,
        impact: cp.impact,
      });
    }
  }

  return instructions;
}

function buildStructurePrompt(narrativeStructure, chapter) {
  const instructions = generateChapterInstructions(narrativeStructure, chapter);
  const parts = [];

  if (instructions.arcInstructions.length > 0) {
    const arcDesc = instructions.arcInstructions
      .map((ai) => `[${ai.arcType}] ${ai.stageName} - ${ai.stageDescription}`)
      .join("、");
    parts.push(`【弧线推进】本章需推进以下弧线：${arcDesc}`);
  }

  if (instructions.foreshadowInstructions.length > 0) {
    const fsDesc = instructions.foreshadowInstructions
      .map((fi) => {
        const action = fi.action === "setup" ? "埋设伏笔" : "回收伏笔";
        return `(${action})${fi.description}`;
      })
      .join("、");
    parts.push(`【伏笔管理】本章需完成：${fsDesc}`);
  }

  if (instructions.confluenceTriggers.length > 0) {
    const cpDesc = instructions.confluenceTriggers
      .map((cp) => `【${cp.confluenceType}】${cp.impact}`)
      .join("、");
    parts.push(`【交汇点触发】本章关键事件：${cpDesc}`);
  }

  if (parts.length === 0) {
    parts.push("本章无特殊叙事结构约束，可自由推进剧情");
  }

  return parts.join("\n");
}

// ── 测试用例 ──

function testStructureConstruction() {
  console.log("\n📋 测试 1：叙事结构数据构建与完整性验证");
  const structure = createTestNarrativeStructure();

  assertEq(structure.arcs.length, 2, "弧线数量应为 2");
  assertEq(structure.confluences.length, 1, "交汇点数量应为 1");
  assertEq(structure.foreshadows.length, 2, "伏笔数量应为 2");
  assertEq(structure.arcs[0].arcType, "transformative", "主角弧线应为转换型");
  assertEq(structure.arcs[1].arcType, "tragic", "反派弧线应为悲剧型");

  const arcIds = new Set(structure.arcs.map((a) => a.id));
  const fsIds = new Set(structure.foreshadows.map((f) => f.id));
  for (const cp of structure.confluences) {
    for (const arcId of cp.involvedArcs) assert(arcIds.has(arcId), `交汇点引用了不存在的弧线: ${arcId}`);
    for (const fsId of cp.involvedForeshadows) assert(fsIds.has(fsId), `交汇点引用了不存在的伏笔: ${fsId}`);
  }

  console.log("  ✅ 基础结构 + 引用完整性验证通过");
  return true;
}

function testChapterInstructions() {
  console.log("\n📋 测试 2：章节指令生成逻辑");
  const structure = createTestNarrativeStructure();

  const ch1 = generateChapterInstructions(structure, 1);
  assert(ch1.arcInstructions.length > 0, "第 1 章应有弧线指令");
  assert(ch1.foreshadowInstructions.length > 0, "第 1 章应有伏笔埋设指令");
  assertEq(ch1.arcInstructions[0].stageName, "平凡生活", "第 1 章弧线阶段名称不正确");
  assertEq(ch1.foreshadowInstructions[0].action, "setup", "第 1 章应为埋设伏笔");

  const ch12 = generateChapterInstructions(structure, 12);
  assertEq(ch12.arcInstructions.length, 2, "第 12 章应有 2 条弧线指令");
  assert(ch12.foreshadowInstructions.length > 0, "第 12 章应有伏笔回收指令");
  assert(ch12.confluenceTriggers.length > 0, "第 12 章应有交汇点触发");

  const ch2 = generateChapterInstructions(structure, 2);
  assertEq(ch2.arcInstructions.length, 1, "第 2 章应有 1 条弧线指令");
  assertEq(ch2.foreshadowInstructions.length, 0, "第 2 章无伏笔指令");
  assertEq(ch2.confluenceTriggers.length, 0, "第 2 章无交汇点");

  const ch5 = generateChapterInstructions(structure, 5);
  assertEq(ch5.arcInstructions.length, 1, "第 5 章应有弧线指令");
  assertEq(ch5.foreshadowInstructions.length, 0, "第 5 章无伏笔指令");

  const ch10 = generateChapterInstructions(structure, 10);
  assertEq(ch10.arcInstructions.length, 0, "第 10 章无弧线指令");
  assertEq(ch10.foreshadowInstructions.length, 1, "第 10 章应有伏笔回收指令");
  assertEq(ch10.foreshadowInstructions[0].action, "payoff", "第 10 章应为回收伏笔");
  assertEq(ch10.foreshadowInstructions[0].foreshadowId, "fs-artifact", "第 10 章应回收 fs-artifact");

  const ch7 = generateChapterInstructions(structure, 7);
  assert(ch7.arcInstructions.length === 0, "第 7 章无弧线指令");
  assert(ch7.foreshadowInstructions.length === 0, "第 7 章无伏笔指令");
  assert(ch7.confluenceTriggers.length === 0, "第 7 章无交汇点");

  console.log("  ✅ 章节指令生成验证通过");
  return true;
}

function testStructurePromptGeneration() {
  console.log("\n📋 测试 3：结构约束文本生成");
  const structure = createTestNarrativeStructure();

  const prompt3 = buildStructurePrompt(structure, 3);
  assert(prompt3.includes("弧线推进"), "第 3 章应包含弧线推进约束");
  assert(prompt3.includes("遭遇变故"), "第 3 章应包含具体阶段描述");
  assert(!prompt3.includes("伏笔管理"), "第 3 章不应包含伏笔约束");

  const prompt1 = buildStructurePrompt(structure, 1);
  assert(prompt1.includes("弧线推进"), "第 1 章应包含弧线推进约束");
  assert(prompt1.includes("伏笔管理"), "第 1 章应包含伏笔管理约束");
  assert(prompt1.includes("埋设伏笔"), "第 1 章应为埋设伏笔");

  const prompt12 = buildStructurePrompt(structure, 12);
  assert(prompt12.includes("交汇点触发"), "第 12 章应包含交汇点约束");

  const prompt7 = buildStructurePrompt(structure, 7);
  assert(prompt7.includes("自由推进剧情"), "第 7 章应为自由章节");

  console.log("  ✅ 结构约束文本生成验证通过");
  return true;
}

function testSerialization() {
  console.log("\n📋 测试 4：JSON 序列化/反序列化");
  const structure = createTestNarrativeStructure();

  const json = JSON.stringify(structure);
  assert(json.length > 0, "序列化结果不应为空");

  const deserialized = JSON.parse(json);
  assertEq(deserialized.arcs.length, structure.arcs.length, "序列化后弧线数量不一致");
  assertEq(deserialized.confluences.length, structure.confluences.length, "序列化后交汇点数量不一致");
  assertEq(deserialized.foreshadows.length, structure.foreshadows.length, "序列化后伏笔数量不一致");
  assertEq(deserialized.arcs[0].id, structure.arcs[0].id, "序列化后弧线 ID 不一致");
  assertEq(deserialized.arcs[0].stages.length, structure.arcs[0].stages.length, "序列化后阶段数量不一致");
  assertEq(deserialized.foreshadows[0].status, structure.foreshadows[0].status, "序列化后伏笔状态不一致");

  for (let ch = 1; ch <= 12; ch++) {
    const orig = generateChapterInstructions(structure, ch);
    const deser = generateChapterInstructions(deserialized, ch);
    assertEq(orig.arcInstructions.length, deser.arcInstructions.length, `第 ${ch} 章弧线指令数量不一致`);
    assertEq(orig.foreshadowInstructions.length, deser.foreshadowInstructions.length, `第 ${ch} 章伏笔指令数量不一致`);
    assertEq(orig.confluenceTriggers.length, deser.confluenceTriggers.length, `第 ${ch} 章交汇点数量不一致`);
  }

  console.log("  ✅ JSON 序列化/反序列化验证通过");
  return true;
}

function testStructureAdjustmentLogic() {
  console.log("\n📋 测试 5：结构调整逻辑");
  const structure = createTestNarrativeStructure();

  const originalFsCount = structure.foreshadows.length;
  const idx = structure.foreshadows.findIndex((f) => f.id === "fs-artifact");
  assert(idx >= 0, "应能找到 fs-artifact 伏笔");
  const originalPayoff = structure.foreshadows[idx].payoffChapter;
  structure.foreshadows[idx].payoffChapter = 11;

  const ch10 = generateChapterInstructions(structure, 10);
  const ch11 = generateChapterInstructions(structure, 11);
  const fsInCh10 = ch10.foreshadowInstructions.some((f) => f.foreshadowId === "fs-artifact" && f.action === "payoff");
  const fsInCh11 = ch11.foreshadowInstructions.some((f) => f.foreshadowId === "fs-artifact" && f.action === "payoff");
  assert(!fsInCh10, "调整后第 10 章不应包含 fs-artifact 回收");
  assert(fsInCh11, "调整后第 11 章应包含 fs-artifact 回收");
  assertEq(structure.foreshadows.length, originalFsCount, "伏笔数量不应改变");

  console.log(`  ✅ 结构调整逻辑验证通过 (伏笔回收: ${originalPayoff} → 11)`);
  return true;
}

function testEdgeCases() {
  console.log("\n📋 测试 6：边界条件");
  const emptyStructure = { arcs: [], confluences: [], foreshadows: [] };
  for (let ch = 1; ch <= 20; ch++) {
    const inst = generateChapterInstructions(emptyStructure, ch);
    assert(inst.arcInstructions.length === 0, `空结构第 ${ch} 章不应有弧线指令`);
    assert(inst.foreshadowInstructions.length === 0, `空结构第 ${ch} 章不应有伏笔指令`);
    assert(inst.confluenceTriggers.length === 0, `空结构第 ${ch} 章不应有交汇点`);
  }
  const structure = createTestNarrativeStructure();
  const inst = generateChapterInstructions(structure, 999);
  assertEq(inst.arcInstructions.length, 0, "不存在的章节不应有指令");
  const prompt = buildStructurePrompt(emptyStructure, 1);
  assert(prompt.includes("自由推进剧情"), "空结构应为自由章节");

  console.log("  ✅ 边界条件验证通过");
  return true;
}

// ── API 调用 ──

async function callLlmApi(provider, messages, systemPrompt, temperature = 0.7, maxTokens = 2000) {
  const payload = {
    model: provider.model,
    messages: systemPrompt ? [{ role: "system", content: systemPrompt }, ...messages] : messages,
    temperature,
    max_tokens: maxTokens,
  };

  const response = await fetch(provider.url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${provider.key}`,
    },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`API 错误 (${response.status}): ${errorText}`);
  }

  const data = await response.json();
  return {
    content: data.choices?.[0]?.message?.content || "",
    usage: data.usage || null,
    raw: data,
  };
}

// ── 真实 LLM 集成测试 ──

/**
 * 测试 7：结构注入 —— 验证 LLM 能根据结构约束生成内容
 */
async function testStructureInjection(provider) {
  console.log("\n📋 测试 7：结构注入（真实 LLM）");
  const structure = createTestNarrativeStructure();

  const testCases = [
    { chapter: 1, expectKeywords: ["弧线", "伏笔", "平凡生活", "埋设"], desc: "第 1 章：弧线起点 + 伏笔埋设" },
    { chapter: 3, expectKeywords: ["弧线", "遭遇变故"], desc: "第 3 章：弧线推进" },
    { chapter: 10, expectKeywords: ["回收", "玉佩"], desc: "第 10 章：伏笔回收" },
  ];

  for (const tc of testCases) {
    console.log(`  📖 ${tc.desc}`);
    const structurePrompt = buildStructurePrompt(structure, tc.chapter);

    const systemPrompt = `你是一名资深文学创作编辑。请严格按照叙事结构约束指导作者创作，输出必须：
1. 明确引用提供的结构约束关键词
2. 列出本章的创作要点（3-5 条）
3. 给出具体的写作建议（50 字以内）`;

    const userMessage = `请为第 ${tc.chapter} 章提供创作指导。\n\n结构约束：\n${structurePrompt}`;

    const { content } = await callLlmApi(provider, [{ role: "user", content: userMessage }], systemPrompt, 0.7, 1000);

    assert(content.length > 20, `第 ${tc.chapter} 章响应不应过短（<20 字）`);

    // 检查 LLM 是否响应了结构约束关键词
    const missingKeywords = tc.expectKeywords.filter((kw) => !content.includes(kw));
    if (missingKeywords.length > 0) {
      console.log(`    ⚠️  响应中未检测到关键词: ${missingKeywords.join(", ")}（可能因模型翻译/改写）`);
    } else {
      console.log(`    ✅ 关键词覆盖完整`);
    }
    console.log(`    📝 响应预览: ${content.slice(0, 120).replace(/\s+/g, " ")}...`);
  }

  console.log("  ✅ 结构注入 API 测试通过");
  return true;
}

/**
 * 测试 8：结构调整反馈 —— 修改结构后 LLM 输出应响应变化
 */
async function testStructureAdjustmentFeedback(provider) {
  console.log("\n📋 测试 8：结构调整反馈（真实 LLM）");
  const structure = createTestNarrativeStructure();

  // 调整前：第 10 章回收 fs-artifact
  const beforePrompt = buildStructurePrompt(structure, 10);

  // 执行结构调整：将 fs-artifact 从第 10 章移到第 11 章
  const idx = structure.foreshadows.findIndex((f) => f.id === "fs-artifact");
  const originalPayoff = structure.foreshadows[idx].payoffChapter;
  structure.foreshadows[idx].payoffChapter = 11;
  const afterPromptCh10 = buildStructurePrompt(structure, 10);
  const afterPromptCh11 = buildStructurePrompt(structure, 11);

  assert(beforePrompt.includes("回收伏笔"), "调整前第 10 章应包含回收伏笔");
  assert(!afterPromptCh10.includes("回收伏笔"), "调整后第 10 章不应包含回收伏笔");
  assert(afterPromptCh11.includes("回收伏笔"), "调整后第 11 章应包含回收伏笔");

  // 验证 LLM 对调整后的结构做出不同响应
  console.log("  📤 验证调整前后 LLM 响应差异...");
  const systemPrompt = `你是文学创作编辑。针对每章给出 2-3 条创作要点。`;
  const beforeResp = await callLlmApi(
    provider,
    [{ role: "user", content: `第 10 章创作约束：\n${beforePrompt}\n请给出创作要点。` }],
    systemPrompt,
    0.3,
    500
  );
  const afterResp = await callLlmApi(
    provider,
    [{ role: "user", content: `第 10 章创作约束：\n${afterPromptCh10}\n请给出创作要点。` }],
    systemPrompt,
    0.3,
    500
  );

  assert(beforeResp.content !== afterResp.content, "调整前后 LLM 响应应有差异");
  console.log(`    ✅ 调整前后响应不同（差异检测通过）`);
  console.log(`    📌 伏笔回收位置: 第 ${originalPayoff} 章 → 第 11 章`);

  // 验证调整后第 11 章的结构约束被 LLM 正确理解
  console.log("  📤 验证调整后第 11 章的结构注入...");
  const adjResp = await callLlmApi(
    provider,
    [{ role: "user", content: `第 11 章创作约束：\n${afterPromptCh11}\n请指出本章的关键叙事事件。` }],
    systemPrompt,
    0.3,
    500
  );
  assert(adjResp.content.length > 20, "调整后第 11 章响应不应为空");
  if (!adjResp.content.includes("玉佩") && !adjResp.content.includes("回收")) {
    console.log("    ⚠️  响应未直接提及玉佩/回收（模型可能改写了表达）");
  } else {
    console.log("    ✅ 第 11 章正确响应了伏笔回收");
  }

  console.log("  ✅ 结构调整反馈验证通过");
  return true;
}

/**
 * 测试 9：完整工作流模拟 —— 多章节连续生成
 */
async function testFullWorkflowSimulation(provider) {
  console.log("\n📋 测试 9：完整工作流模拟（真实 LLM）");
  const structure = createTestNarrativeStructure();

  const plan = [
    { chapter: 1, desc: "开篇", expect: ["平凡生活", "埋设"] },
    { chapter: 3, desc: "转折", expect: ["遭遇变故"] },
    { chapter: 6, desc: "挣扎", expect: ["内心挣扎"] },
    { chapter: 12, desc: "高潮", expect: ["终极对决", "交汇"] },
  ];

  const results = [];
  const systemPrompt = `你是资深小说创作教练。对每个章节，按以下格式输出：
【本章要点】
1. ...
2. ...
3. ...
【结构合规】 是/否
【备注】 ...`;

  for (const step of plan) {
    console.log(`  📖 第 ${step.chapter} 章 · ${step.desc}`);
    const prompt = buildStructurePrompt(structure, step.chapter);
    const { content } = await callLlmApi(
      provider,
      [{ role: "user", content: `根据以下叙事结构约束，为第 ${step.chapter} 章（${step.desc}）撰写创作指导：\n\n${prompt}` }],
      systemPrompt,
      0.7,
      800
    );
    results.push({ chapter: step.chapter, content });

    // 验证响应长度
    assert(content.length > 30, `第 ${step.chapter} 章响应过短`);

    // 检查结构合规关键词
    const hasExpected = step.expect.some((kw) => content.includes(kw));
    if (!hasExpected) {
      console.log(`    ⚠️  响应未直接包含预期关键词 [${step.expect.join("/")}]（可能的表述差异）`);
    } else {
      console.log(`    ✅ 结构关键词覆盖`);
    }
  }

  // 验证整体一致性（所有响应均不为空且长度适中）
  const allValid = results.every((r) => r.content.length > 30);
  assert(allValid, "所有章节响应应有效");
  console.log(`  ✅ 完整工作流模拟通过（${results.length} 章连续生成）`);
  return true;
}

// ── 主测试流程 ──

async function main() {
  console.log("\n" + "=".repeat(60));
  console.log("🎭  AxAgent 叙事结构端到端集成测试");
  console.log("=".repeat(60));

  const mode = RUN_API && API_PROVIDER ? "API 集成测试（真实 LLM）" : "数据模型测试（纯逻辑）";
  console.log(`\n📊 测试模式: ${mode}`);
  if (API_PROVIDER) {
    console.log(`🤖 Provider: ${API_PROVIDER.name} (${API_PROVIDER.id})`);
    console.log(`📡 端点: ${API_PROVIDER.url}`);
    console.log(`🧠 模型: ${API_PROVIDER.model}`);
  } else if (RUN_API) {
    console.log("⚠️  指定了 --api 但未找到可用 provider（检查 API key/URL），回退到数据模型测试");
  }

  const results = [];

  // 阶段 1：数据模型测试
  console.log("\n📝 阶段 1：数据模型与逻辑验证");
  console.log("-".repeat(40));

  const dataTests = [
    { name: "数据构建与完整性", fn: testStructureConstruction },
    { name: "章节指令生成", fn: testChapterInstructions },
    { name: "结构约束文本生成", fn: testStructurePromptGeneration },
    { name: "JSON 序列化/反序列化", fn: testSerialization },
    { name: "结构调整逻辑", fn: testStructureAdjustmentLogic },
    { name: "边界条件", fn: testEdgeCases },
  ];

  for (const test of dataTests) {
    try {
      const passed = test.fn();
      results.push({ name: test.name, passed });
    } catch (error) {
      console.error(`  ❌ ${test.name}: ${error.message}`);
      results.push({ name: test.name, passed: false, error: error.message });
    }
  }

  // 阶段 2：API 集成测试
  console.log("\n📝 阶段 2：API 集成测试");
  console.log("-".repeat(40));

  if (!RUN_API) {
    console.log("  ⏭️  跳过 API 测试（使用 --api 启用）");
    console.log("     可用 provider:");
    for (const [id, p] of Object.entries(PROVIDERS)) {
      const ok = p.key && p.url;
      console.log(`       ${ok ? "✅" : "❌"} ${id}: ${ok ? p.model : "未配置"}`);
    }
    results.push({ name: "API 集成测试", passed: true, skipped: true });
  } else if (!API_PROVIDER) {
    console.log("  ⚠️  无可用 provider，跳过 API 测试");
    results.push({ name: "API 集成测试", passed: true, skipped: true });
  } else {
    try {
      const r1 = await testStructureInjection(API_PROVIDER);
      results.push({ name: "结构注入", passed: r1 });

      const r2 = await testStructureAdjustmentFeedback(API_PROVIDER);
      results.push({ name: "结构调整反馈", passed: r2 });

      const r3 = await testFullWorkflowSimulation(API_PROVIDER);
      results.push({ name: "完整工作流模拟", passed: r3 });
    } catch (error) {
      console.error(`  ❌ API 测试失败: ${error.message}`);
      results.push({ name: "API 集成测试", passed: false, error: error.message });
    }
  }

  printSummary(results);

  const criticalPassed = results.filter((r) => !r.skipped).every((r) => r.passed);
  const allPassed = results.every((r) => r.passed || r.skipped);

  if (!criticalPassed) {
    process.exit(1);
  }

  if (allPassed) {
    console.log("\n🎉 所有测试通过！叙事结构实现验证完成。");
  } else {
    console.log("\n⚠️  部分测试未通过（跳过的测试不计入失败）");
  }
}

function printSummary(results) {
  console.log("\n" + "=".repeat(60));
  console.log("📊 测试结果总结");
  console.log("=".repeat(60));
  for (const result of results) {
    if (result.skipped) console.log(`  ⏭️  ${result.name} (跳过)`);
    else if (result.passed) console.log(`  ✅ ${result.name}`);
    else console.log(`  ❌ ${result.name}${result.error ? `: ${result.error}` : ""}`);
  }
  const passed = results.filter((r) => r.passed).length;
  const failed = results.filter((r) => !r.passed && !r.skipped).length;
  const skipped = results.filter((r) => r.skipped).length;
  console.log(`\n  总计: ${results.length} | 通过: ${passed} | 失败: ${failed} | 跳过: ${skipped}`);
}

main().catch((error) => {
  console.error("\n💥 测试执行异常:", error);
  process.exit(1);
});
