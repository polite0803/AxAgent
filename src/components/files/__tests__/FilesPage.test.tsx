import { FilesPage } from "@/pages/FilesPage";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const translations: Record<string, string> = {
        "files.images": "图片",
        "files.files": "文件",
      };
      return translations[key] || key;
    },
  }),
}));

vi.mock("@/components/files/FilesContent", () => ({
  FilesContent: ({ activeCategory }: { activeCategory: string }) => (
    <div data-testid="files-content" data-category={activeCategory}>
      Files Content for {activeCategory}
    </div>
  ),
}));

vi.mock("antd", () => ({
  Tabs: ({
    items,
    activeKey,
    onChange,
  }: {
    items: Array<{ key: string; label: React.ReactNode }>;
    activeKey: string;
    onChange: (key: string) => void;
  }) => (
    <div data-testid="files-tabs">
      {items.map((item) => (
        <button
          key={item.key}
          data-testid={`tab-${item.key}`}
          onClick={() => onChange(item.key)}
          className={activeKey === item.key ? "active" : ""}
        >
          {item.label}
        </button>
      ))}
    </div>
  ),
  ConfigProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

describe("FilesPage", () => {
  it("renders without crashing", () => {
    const { container } = render(<FilesPage />);
    expect(container).toBeTruthy();
  });

  it("renders images as the default category", () => {
    render(<FilesPage />);
    expect(screen.getByTestId("files-content")).toHaveAttribute("data-category", "images");
  });

  it("has tabs for images and files categories", () => {
    render(<FilesPage />);
    expect(screen.getByTestId("tab-images")).toBeTruthy();
    expect(screen.getByTestId("tab-files")).toBeTruthy();
  });
});
