import i18n from "@/i18n";
import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { I18nextProvider } from "react-i18next";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ReplayWorkbench } from "../ReplayWorkbench";

const navigateMock = vi.fn();
vi.mock("react-router-dom", async () => {
  const actual = await vi.importActual<typeof import("react-router-dom")>("react-router-dom");
  return {
    ...actual,
    useNavigate: () => navigateMock,
  };
});

function renderWithProviders() {
  return render(
    <MemoryRouter>
      <I18nextProvider i18n={i18n}>
        <ReplayWorkbench />
      </I18nextProvider>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  navigateMock.mockReset();
  useTimeAnchorStore.setState({
    asOfDate: null,
    mode: "live",
    tourSeen: true,
    pendingLiveConfirm: false,
  });
});

afterEach(() => {
  useTimeAnchorStore.setState({ asOfDate: null, mode: "live" });
});

describe("ReplayWorkbench", () => {
  it("renders the workbench title", () => {
    renderWithProviders();
    expect(screen.getByText(i18n.t("replayWorkbench.title"))).toBeInTheDocument();
  });

  it("does NOT pre-fill the date picker (forces re-selection)", () => {
    useTimeAnchorStore.setState({ asOfDate: "2026-05-01", mode: "replay" });
    renderWithProviders();
    // 第一次进入 workbench 总是空白 picker，强迫用户重选
    expect(screen.getByText(i18n.t("replayWorkbench.step1.title"))).toBeInTheDocument();
  });

  it("shows the action card only after picking a date", () => {
    renderWithProviders();
    // action 区域没出现
    expect(screen.queryByTestId("goto-stock-analysis")).toBeNull();
  });

  it("renders an exit panel with Switch-to-Live button when mode is replay", () => {
    useTimeAnchorStore.setState({ asOfDate: "2026-05-01", mode: "replay" });
    renderWithProviders();
    expect(screen.getByTestId("replay-exit-btn")).toBeInTheDocument();
  });

  it("exit button triggers enterLive", () => {
    useTimeAnchorStore.setState({ asOfDate: "2026-05-01", mode: "replay" });
    renderWithProviders();
    act(() => {
      fireEvent.click(screen.getByTestId("replay-exit-btn"));
    });
    // enterLive(false) → 直接切到 live，asOfDate 清空
    expect(useTimeAnchorStore.getState().mode).toBe("live");
    expect(useTimeAnchorStore.getState().asOfDate).toBeNull();
  });
});
