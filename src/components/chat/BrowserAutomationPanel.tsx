import { invoke } from "@tauri-apps/api/core";
import { Button, Card, Input, Space, Typography, message, Table } from "antd";
import { Globe, Image, MousePointer, Keyboard, Search, X } from "lucide-react";
import { useState } from "react";
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
  const [url, setUrl] = useState("");
  const [currentUrl, setCurrentUrl] = useState("");
  const [title, setTitle] = useState("");
  const [screenshot, setScreenshot] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [elements, setElements] = useState<ExtractedElement[]>([]);
  const [selector, setSelector] = useState("a, button, input, select, textarea");

  const handleNavigate = async () => {
    if (!url.trim()) {
      message.warning("请输入 URL");
      return;
    }
    setLoading(true);
    try {
      const result = await invoke<NavigateResult>("browser_navigate", { url: url.trim() });
      setCurrentUrl(result.url);
      setTitle(result.title);
      message.success("导航成功");
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
      message.success("截图成功");
    } catch (e) {
      message.error(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleExtractElements = async () => {
    if (!selector.trim()) {
      message.warning("请输入选择器");
      return;
    }
    setLoading(true);
    try {
      const result = await invoke<ExtractedElement[]>("browser_extract_all", { selector });
      setElements(result);
      message.success(`发现 ${result.length} 个元素`);
    } catch (e) {
      message.error(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleClick = async (sel: string) => {
    try {
      await invoke("browser_click", { selector: sel });
      message.success("点击成功");
      setTimeout(() => handleScreenshot(), 500);
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleFill = async (sel: string, value: string) => {
    try {
      await invoke("browser_fill", { selector: sel, value });
      message.success("填写成功");
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
      message.success("浏览器已关闭");
    } catch (e) {
      message.error(String(e));
    }
  };

  const columns = [
    { title: "Tag", dataIndex: "tag", key: "tag", width: 80 },
    { title: "Text", dataIndex: "text", key: "text", ellipsis: true },
    { title: "Href", dataIndex: "href", key: "href", ellipsis: true },
    { title: "Type", dataIndex: "type", key: "type", width: 80 },
  ];

  return (
    <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 12 }}>
      <Card size="small" title="浏览器控制">
        <Space direction="vertical" style={{ width: "100%" }}>
          <Space>
            <Input
              placeholder="输入 URL"
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
              导航
            </Button>
            <Button icon={<Image size={14} />} onClick={() => handleScreenshot()} loading={loading}>
              截图
            </Button>
            <Button icon={<X size={14} />} onClick={handleClose} danger>
              关闭
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

      <Card size="small" title="元素操作">
        <Space direction="vertical" style={{ width: "100%" }}>
          <Space>
            <Input
              placeholder="CSS 选择器 (如: a, button, input)"
              value={selector}
              onChange={(e) => setSelector(e.target.value)}
              style={{ width: 250 }}
            />
            <Button icon={<Search size={14} />} onClick={handleExtractElements} loading={loading}>
              提取元素
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

      <Card size="small" title="快捷操作">
        <Space wrap>
          <Button size="small" icon={<MousePointer size={12} />} onClick={() => handleClick("body")}>
            点击 Body
          </Button>
          <Button size="small" icon={<Keyboard size={12} />} onClick={() => handleFill("input[type='text']", "test")}>
            填写文本
          </Button>
        </Space>
      </Card>
    </div>
  );
}
