// SPDX-License-Identifier: AGPL-3.0-only

import type {
  AdjustmentType,
  ArcStage,
  ArcType,
  ChapterMeta,
  ConfluencePoint,
  ConfluenceType,
  Foreshadow,
  ForeshadowStatus,
  NarrativeArc,
  NarrativeStructure,
  StructureAdjustmentSuggestion,
} from "@/types/narrative";
import { describe, expect, it } from "vitest";

describe("NarrativeStructure type contracts", () => {
  it("NarrativeStructure can be constructed with arcs, confluences, and foreshadows", () => {
    const structure: NarrativeStructure = {
      arcs: [],
      confluences: [],
      foreshadows: [],
    };
    expect(structure.arcs).toEqual([]);
    expect(structure.confluences).toEqual([]);
    expect(structure.foreshadows).toEqual([]);
  });

  it("NarrativeArc supports all arc types", () => {
    const arcTypes: ArcType[] = [
      "transformative",
      "steadfast",
      "flat",
      "tragic",
      "comedic",
    ];
    arcTypes.forEach((type) => {
      const arc: NarrativeArc = {
        id: `arc-${type}`,
        subject: "主角",
        want: "目标",
        need: "内心需求",
        arcType: type,
        stages: [],
        currentProgress: 0.5,
      };
      expect(arc.arcType).toBe(type);
    });
  });

  it("ArcStage tracks chapter and description", () => {
    const stages: ArcStage[] = [
      { name: "诱因", chapter: 1, description: "事件起点" },
      { name: "转变", chapter: 3, description: "角色变化" },
    ];
    expect(stages.length).toBe(2);
    expect(stages[0].chapter).toBe(1);
  });

  it("ConfluencePoint supports all confluence types", () => {
    const confluenceTypes: ConfluenceType[] = [
      "conflict_burst",
      "reveal_truth",
      "shift_perspective",
    ];
    confluenceTypes.forEach((type) => {
      const cp: ConfluencePoint = {
        id: `cp-${type}`,
        triggerChapter: 5,
        confluenceType: type,
        involvedArcs: ["arc-1", "arc-2"],
        involvedForeshadows: ["fs-1"],
        impact: "多条线索汇聚",
      };
      expect(cp.confluenceType).toBe(type);
      expect(cp.involvedArcs.length).toBe(2);
    });
  });

  it("Foreshadow tracks setup and payoff lifecycle", () => {
    const statuses: ForeshadowStatus[] = ["setup", "payoff", "abandoned"];
    statuses.forEach((status) => {
      const fs: Foreshadow = {
        id: `fs-${status}`,
        setupChapter: 2,
        payoffChapter: status === "payoff" ? 8 : null,
        status,
        description: "神秘信件的伏笔",
        payoffDescription: status === "payoff" ? "信件揭示了真凶身份" : null,
        relatedArcs: ["arc-1"],
      };
      expect(fs.status).toBe(status);
    });
  });

  it("Foreshadow supports resolved and unresolved states", () => {
    const unresolved: Foreshadow = {
      id: "fs-pending",
      setupChapter: 2,
      payoffChapter: null,
      status: "setup",
      description: "未回收的伏笔",
      payoffDescription: null,
      relatedArcs: [],
    };
    expect(unresolved.payoffChapter).toBeNull();
    expect(unresolved.status).toBe("setup");

    const resolved: Foreshadow = {
      id: "fs-done",
      setupChapter: 2,
      payoffChapter: 8,
      status: "payoff",
      description: "已回收的伏笔",
      payoffDescription: "回收说明",
      relatedArcs: ["arc-1"],
    };
    expect(resolved.status).toBe("payoff");
    expect(resolved.payoffChapter).toBe(8);
  });

  it("ChapterMeta tracks chapter-level metadata", () => {
    const chapter: ChapterMeta = {
      number: 1,
      title: "开端",
      wordCount: 3000,
      status: "final",
      summary: "主角开始踏上旅程",
    };
    expect(chapter.number).toBe(1);
    expect(chapter.status).toBe("final");
    expect(chapter.wordCount).toBe(3000);
  });

  it("StructureAdjustmentSuggestion supports all adjustment types", () => {
    const adjustmentTypes: AdjustmentType[] = [
      "delay_foreshadow_payoff",
      "accelerate_foreshadow_payoff",
      "add_arc_stage",
      "adjust_arc_progress",
      "reposition_confluence",
      "add_foreshadow",
    ];
    adjustmentTypes.forEach((type) => {
      const suggestion: StructureAdjustmentSuggestion = {
        id: `sug-${type}`,
        adjustmentType: type,
        description: "建议调整",
        affectedElements: ["arc-1"],
        priority: "high",
        rationale: "需要增强叙事张力",
        targetType: "arc",
        targetId: "arc-1",
        payload: {},
      };
      expect(suggestion.adjustmentType).toBe(type);
    });
  });

  it("NarrativeStructure can hold complex multi-arc structures", () => {
    const structure: NarrativeStructure = {
      arcs: [
        {
          id: "arc-1",
          subject: "主角",
          want: "找到真相",
          need: "面对内心恐惧",
          arcType: "transformative",
          stages: [
            { name: "诱因", chapter: 1, description: "平凡生活被打破" },
            { name: "挑战", chapter: 3, description: "面对试炼" },
            { name: "转变", chapter: 7, description: "思想觉醒" },
          ],
          currentProgress: 0.6,
        },
        {
          id: "arc-2",
          subject: "反派",
          want: "控制一切",
          need: "被爱",
          arcType: "tragic",
          stages: [
            { name: "起点", chapter: 2, description: "理想主义" },
            { name: "腐蚀", chapter: 5, description: "权力诱惑" },
          ],
          currentProgress: 0.8,
        },
      ],
      confluences: [
        {
          id: "cp-1",
          triggerChapter: 10,
          confluenceType: "conflict_burst",
          involvedArcs: ["arc-1", "arc-2"],
          involvedForeshadows: ["fs-1", "fs-2"],
          impact: "正邪双方正面交锋",
        },
      ],
      foreshadows: [
        {
          id: "fs-1",
          setupChapter: 1,
          payoffChapter: 10,
          status: "payoff",
          description: "古老预言被发现",
          payoffDescription: "预言成真",
          relatedArcs: ["arc-1"],
        },
        {
          id: "fs-2",
          setupChapter: 3,
          payoffChapter: 10,
          status: "payoff",
          description: "主角获得神秘武器",
          payoffDescription: "武器在最终战中发挥作用",
          relatedArcs: ["arc-1"],
        },
      ],
    };
    expect(structure.arcs.length).toBe(2);
    expect(structure.confluences.length).toBe(1);
    expect(structure.foreshadows.length).toBe(2);
    expect(structure.confluences[0].involvedArcs.length).toBe(2);
    expect(structure.foreshadows[0].status).toBe("payoff");
  });
});
