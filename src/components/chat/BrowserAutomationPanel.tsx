import { invoke } from "@tauri-apps/api/core";
import { Button, Card, Input, message, Space, Table, Typography } from "antd";
import { Globe, Image, Keyboard, MousePointer, Search, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

interface NavigateResult {
  url: string;
  title: string;
}

interface ScreenshotResult {
  image_base64: string;
}

interface ExtractedElement {
  tag: string;
  text?: string;
  href?: string;
  type?: string;
  placeholder?: string;
}

export function BrowserAutomationPanel() {
  const { t } = useTranslation();
  const mountedRef = useRef(true);
  useEffect(() => () => {
    mountedRef.current = false;
  }, []);
  const [url, setUrl] = useState("");
  const [currentUrl, setCurrentUrl] = useState("");
  const [title, setTitle] = useState("");
  const [screenshot, setScreenshot] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [elements, setElements] = useState<ExtractedElement[]>([]);
  const [selector, setSelector] = useState("a, button, input, select, textarea");

  const handleNavigate = async () => {
    if (!url.trim()) {
      message.warning(t("browser.enterUrl"));
      return;
    }
    setLoading(true);
    try {
      const result = await invoke<NavigateResult>("browser_navigate", { url: url.trim() });
      setCurrentUrl(result.url);
      setTitle(result.title);
      message.success(t("browser.navigateSuccess"));
    } catch (e) {
      message.error(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleScreenshot = async (fullPage?: boolean) => {
    setLoading(true);
    try {
      const result = await invoke<ScreenshotResult>("browser_screenshot", { fullPage });
      setScreenshot(`data:image/png;base64,${result.image_base64}`);
      message.success(t("browser.screenshotSuccess"));
    } catch (e) {
      message.error(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleExtractElements = async () => {
    if (!selector.trim()) {
      message.warning(t("browser.pleaseEnterSelector"));
      return;
    }
    setLoading(true);
    try {
      const result = await invoke<ExtractedElement[]>("browser_extract_all", { selector });
      setElements(result);
      message.success(t("browser.elementsFound", { count: result.length }));
    } catch (e) {
      message.error(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleClick = async (sel: string) => {
    try {
      await invoke("browser_click", { selector: sel });
      message.success(t("browser.clickSuccess"));
      setTimeout(() => {
        if (mountedRef.current) { handleScreenshot(); }
      }, 500);
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleFill = async (sel: string, value: string) => {
    try {
      await invoke("browser_fill", { selector: sel, value });
      message.success(t("browser.fillSuccess"));
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleClose = async () => {
    try {
      await invoke("browser_close");
      setScreenshot(null);
      setCurrentUrl("");
      setTitle("");
      setElements([]);
      message.success(t("browser.closed"));
    } catch (e) {
      message.error(String(e));
    }
  };

  const columns = [
    { title: t("common.tag"), dataIndex: "tag", key: "tag", width: 80 },
    { title: t("common.text"), dataIndex: "text", key: "text", ellipsis: true },
    { title: "Href", dataIndex: "href", key: "href", ellipsis: true },
    { title: t("common.type"), dataIndex: "type", key: "type", width: 80 },
  ];

  return (
    <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 12 }}>
      <Card size="small" title={t("browser.control")}>
        <Space direction="vertical" style={{ width: "100%" }}>
          <Space>
            <Input
              placeholder={t("browser.enterUrl")}
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              onPressEnter={handleNavigate}
              style={{ width: 300 }}
            />
            <Button
              icon={<Globe size={14} />}
              type="primary"
              onClick={handleNavigate}
              loading={loading}
            >
              {t("browser.navigate")}
            </Button>
            <Button icon={<Image size={14} />} onClick={() => handleScreenshot()} loading={loading}>
              {t("browser.screenshot")}
            </Button>
            <Button icon={<X size={14} />} onClick={handleClose} danger>
              {t("browser.close")}
            </Button>
          </Space>

          {currentUrl && (
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              {title} - {currentUrl}
            </Typography.Text>
          )}
        </Space>
      </Card>

      {screenshot && (
        <Card size="small" bodyStyle={{ padding: 0 }}>
          <img src={screenshot} alt="browser screenshot" style={{ width: "100%", display: "block" }} />
        </Card>
      )}

      <Card size="small" title={t("browser.elementOperations")}>
        <Space direction="vertical" style={{ width: "100%" }}>
          <Space>
            <Input
              placeholder={t("browser.cssSelectorPlaceholder")}
              value={selector}
              onChange={(e) => setSelector(e.target.value)}
              style={{ width: 250 }}
            />
            <Button icon={<Search size={14} />} onClick={handleExtractElements} loading={loading}>
              {t("browser.extractElements")}
            </Button>
          </Space>

          {elements.length > 0 && (
            <Table
              size="small"
              dataSource={elements.map((el, i) => ({ ...el, key: i }))}
              columns={columns}
              pagination={{ pageSize: 10 }}
              scroll={{ y: 200 }}
              onRow={(record) => ({
                onClick: () => record.tag === "a" || record.tag === "button" ? handleClick(`css_selector_here`) : null,
                style: { cursor: "pointer" },
              })}
            />
          )}
        </Space>
      </Card>

      <Card size="small" title={t("browser.quickActions")}>
        <Space wrap>
          <Button size="small" icon={<MousePointer size={12} />} onClick={() => handleClick("body")}>
            {t("browser.clickBody")}
          </Button>
          <Button size="small" icon={<Keyboard size={12} />} onClick={() => handleFill("input[type='text']", "test")}>
            {t("browser.fillText")}
          </Button>
        </Space>
      </Card>
    </div>
  );
}
