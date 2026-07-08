import i18n from "@/i18n";
import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import { render, screen } from "@testing-library/react";
import { I18nextProvider } from "react-i18next";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { TimeAnchorTour } from "../TimeAnchorTour";

beforeEach(() => {
  useTimeAnchorStore.setState({
    asOfDate: null,
    mode: "live",
    tourSeen: false,
    pendingLiveConfirm: false,
  });
});

afterEach(() => {
  useTimeAnchorStore.setState({ tourSeen: false });
});

function renderWithI18n(ui: React.ReactNode) {
  return render(<I18nextProvider i18n={i18n}>{ui}</I18nextProvider>);
}

describe("TimeAnchorTour", () => {
  it("renders the tour bubble when tourSeen is false", () => {
    renderWithI18n(<TimeAnchorTour />);
    expect(screen.getByTestId("time-anchor-tour")).toBeInTheDocument();
    expect(screen.getByRole("dialog")).toHaveAttribute(
      "aria-label",
      i18n.t("timeTravel.tour.title"),
    );
  });

  it("does not render when tourSeen is true", () => {
    useTimeAnchorStore.setState({ tourSeen: true });
    renderWithI18n(<TimeAnchorTour />);
    expect(screen.queryByTestId("time-anchor-tour")).toBeNull();
  });

  it("renders the body text and a 'Got it' button", () => {
    renderWithI18n(<TimeAnchorTour />);
    expect(screen.getByText(i18n.t("timeTravel.tour.body"))).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: i18n.t("timeTravel.tour.gotIt") }),
    ).toBeInTheDocument();
  });

  it("renders the anchor hint pointing to the LIVE pill", () => {
    renderWithI18n(<TimeAnchorTour />);
    expect(
      screen.getByText(i18n.t("timeTravel.tour.stepAnchor")),
    ).toBeInTheDocument();
  });

  it("markTourSeen hides the bubble on next render", () => {
    const { rerender } = renderWithI18n(<TimeAnchorTour />);
    expect(screen.getByTestId("time-anchor-tour")).toBeInTheDocument();
    useTimeAnchorStore.getState().markTourSeen();
    rerender(
      <I18nextProvider i18n={i18n}>
        <TimeAnchorTour />
      </I18nextProvider>,
    );
    expect(screen.queryByTestId("time-anchor-tour")).toBeNull();
  });
});
