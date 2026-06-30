// SPDX-License-Identifier: AGPL-3.0-only
// 集成测试：进化引擎触发 → 状态更新 → 历史查询

import { useEvolutionStore } from "@/stores/feature/evolutionStore";
import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";

describe("Evolution Engine Integration", () => {
  it("starts and stops an engine", async () => {
    const { result } = renderHook(() => useEvolutionStore());

    // Start
    await act(async () => {
      await result.current.startEngine("auto_tool_creator");
    });

    expect(result.current.engines["auto_tool_creator"].running).toBe(true);
    expect(
      result.current.evolutionHistory.some(
        (e) => e.engine === "auto_tool_creator" && e.type === "started",
      ),
    ).toBe(true);

    // Stop
    await act(async () => {
      await result.current.stopEngine("auto_tool_creator");
    });

    expect(result.current.engines["auto_tool_creator"].running).toBe(false);
    expect(
      result.current.evolutionHistory.some(
        (e) => e.engine === "auto_tool_creator" && e.type === "stopped",
      ),
    ).toBe(true);
  });

  it("triggers skill evolution and records event", async () => {
    const { result } = renderHook(() => useEvolutionStore());

    const historyBefore = result.current.evolutionHistory.length;

    await act(async () => {
      await result.current.triggerSkillEvolution("wf-123::node-abc");
    });

    // Should have a new evolution event
    expect(result.current.evolutionHistory.length).toBeGreaterThan(historyBefore);
    const latest = result.current.evolutionHistory[result.current.evolutionHistory.length - 1];
    expect(latest.type).toBe("evolved");
    expect(latest.engine).toBe("skill_evolution");
    expect(latest.detail).toContain("wf-123::node-abc");
  });

  it("returns skill evolution history with mock data", () => {
    const { result } = renderHook(() => useEvolutionStore());

    const history = result.current.getSkillEvolutionHistory("any-skill-id");
    expect(history.length).toBeGreaterThan(0);
    expect(history[0]).toHaveProperty("version");
    expect(history[0]).toHaveProperty("summary");
    expect(history[0]).toHaveProperty("metrics");
  });

  it("returns AB test results with mock data", () => {
    const { result } = renderHook(() => useEvolutionStore());

    const results = result.current.getABTestResults("any-skill-id");
    expect(results.length).toBeGreaterThan(0);
    expect(results[0]).toHaveProperty("metric");
    expect(results[0]).toHaveProperty("winner");
  });

  it("updates engine config", async () => {
    const { result } = renderHook(() => useEvolutionStore());

    await act(async () => {
      await result.current.updateEngineConfig("skill_evolution", {
        evolutionRate: 0.05,
        populationSize: 50,
      });
    });

    const engine = result.current.engines["skill_evolution"];
    expect(engine.config.evolutionRate).toBe(0.05);
    expect(engine.config.populationSize).toBe(50);

    expect(
      result.current.evolutionHistory.some(
        (e) => e.engine === "skill_evolution" && e.type === "config_changed",
      ),
    ).toBe(true);
  });

  it("initializes with core and safety engines running", () => {
    const { result } = renderHook(() => useEvolutionStore());

    const engines = result.current.engines;
    expect(engines["skill_evolution"].running).toBe(true);
    expect(engines["constitution"].running).toBe(true);
    expect(engines["sandbox"].running).toBe(true);
  });
});
