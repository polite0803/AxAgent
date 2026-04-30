import { invoke } from "@tauri-apps/api/core";
import { Button, Card, Input, Space, Switch, Tooltip, Typography, message } from "antd";
import { Search, Scissors } from "lucide-react";
import { useEffect, useRef, useState } from "react";

interface CaptureResult {
  image_base64: string;
  width: number;
  height: number;
}

interface UIElement {
  role: string;
  name: string;
  bounds: { x: number; y: number; width: number; height: number };
  is_clickable: boolean;
}

export function ComputerControlPanel() {
  const mountedRef = useRef(true);
  useEffect(() => () => { mountedRef.current = false; }, []);
  const [screenshot, setScreenshot] = useState<string | null>(null);
  const [autoMode, setAutoMode] = useState(false);
  const [elements, setElements] = useState<UIElement[]>([]);
  const [loading, setLoading] = useState(false);
  const [clickCoords, setClickCoords] = useState<{ x: number; y: number } | null>(null);
  const [nativeResolution, setNativeResolution] = useState({ width: 1920, height: 1080 });
  const imgRef = useRef<HTMLImageElement>(null);

  const handleCapture = async () => {
    setLoading(true);
    try {
      const result = await invoke<CaptureResult>("screen_capture", { monitor: 0 });
      setNativeResolution({ width: result.width, height: result.height });
      setScreenshot(`data:image/png;base64,${result.image_base64}`);
    } catch (e) {
      message.error(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleFindElements = async (nameContains?: string) => {
    try {
      const result = await invoke<UIElement[]>("find_ui_elements", {
        query: { name_contains: nameContains },
      });
      setElements(result);
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleImageClick = (e: React.MouseEvent<HTMLImageElement>) => {
    if (!imgRef.current) return;
    const rect = imgRef.current.getBoundingClientRect();
    const scaleX = nativeResolution.width / rect.width;
    const scaleY = nativeResolution.height / rect.height;
    const x = Math.round((e.clientX - rect.left) * scaleX);
    const y = Math.round((e.clientY - rect.top) * scaleY);
    setClickCoords({ x, y });
  };

  const executeClick = async (x: number, y: number) => {
    try {
      await invoke("mouse_click", { x, y, button: "left" });
      message.success(`点击 (${x}, ${y})`);
      setTimeout(() => { if (mountedRef.current) handleCapture(); }, 500);
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleTypeText = async (text: string, x?: number, y?: number) => {
    try {
      await invoke("type_text", { text, x, y });
      message.success("输入完成");
    } catch (e) {
      message.error(String(e));
    }
  };

  const handlePressKey = async (key: string, modifiers?: string[]) => {
    try {
      await invoke("press_key", { key, modifiers: modifiers || [] });
      message.success(`按键 ${key} 已按下`);
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleScroll = async (x: number, y: number, delta: number) => {
    try {
      await invoke("mouse_scroll", { x, y, delta });
      message.success(`滚动 (${delta} > 0 ? "上" : "下"})`);
    } catch (e) {
      message.error(String(e));
    }
  };

  return (
    <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 12 }}>
      <Space>
        <Button
          icon={<Scissors size={14} />}
          onClick={handleCapture}
          loading={loading}
        >
          截屏
        </Button>
        <Button
          icon={<Search size={14} />}
          onClick={() => handleFindElements()}
        >
          查找元素
        </Button>
        <Tooltip title="自动模式下，AI 将自主控制计算机">
          <Switch
            checked={autoMode}
            onChange={setAutoMode}
            checkedChildren="自动"
            unCheckedChildren="手动"
          />
        </Tooltip>
        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          分辨率: {nativeResolution.width}x{nativeResolution.height}
        </Typography.Text>
      </Space>

      {screenshot && (
        <Card size="small" bodyStyle={{ padding: 0 }}>
          <div style={{ position: "relative", cursor: "crosshair" }}>
            <img
              ref={imgRef}
              src={screenshot}
              onClick={handleImageClick}
              style={{ width: "100%", display: "block" }}
              alt="screenshot"
            />
            {clickCoords && (
              <div
                style={{
                  position: "absolute",
                  left: `${(clickCoords.x / nativeResolution.width) * 100}%`,
                  top: `${(clickCoords.y / nativeResolution.height) * 100}%`,
                  width: 8,
                  height: 8,
                  borderRadius: "50%",
                  background: "red",
                  transform: "translate(-50%, -50%)",
                  pointerEvents: "none",
                }}
              />
            )}
            {elements.map((el, i) => (
              <div
                key={i}
                style={{
                  position: "absolute",
                  left: `${(el.bounds.x / nativeResolution.width) * 100}%`,
                  top: `${(el.bounds.y / nativeResolution.height) * 100}%`,
                  width: `${(el.bounds.width / nativeResolution.width) * 100}%`,
                  height: `${(el.bounds.height / nativeResolution.height) * 100}%`,
                  border: "2px solid #1890ff",
                  borderRadius: 4,
                  cursor: el.is_clickable ? "pointer" : "default",
                  pointerEvents: el.is_clickable ? "auto" : "none",
                }}
                onClick={(e) => {
                  e.stopPropagation();
                  executeClick(
                    el.bounds.x + el.bounds.width / 2,
                    el.bounds.y + el.bounds.height / 2
                  );
                }}
                title={`${el.role}: ${el.name}`}
              />
            ))}
          </div>
        </Card>
      )}

      {clickCoords && (
        <Card size="small" title="坐标操作">
          <Space direction="vertical">
            <Typography.Text>
              坐标: ({clickCoords.x}, {clickCoords.y})
            </Typography.Text>
            <Space>
              <Button size="small" type="primary" onClick={() => executeClick(clickCoords.x, clickCoords.y)}>
                执行点击
              </Button>
              <Input
                placeholder="输入文本"
                style={{ width: 200 }}
                onPressEnter={(e) => handleTypeText(e.currentTarget.value, clickCoords.x, clickCoords.y)}
              />
              <Button size="small" onClick={() => handleScroll(clickCoords.x, clickCoords.y, -3)}>
                向上滚
              </Button>
              <Button size="small" onClick={() => handleScroll(clickCoords.x, clickCoords.y, 3)}>
                向下滚
              </Button>
            </Space>
          </Space>
        </Card>
      )}

      {elements.length > 0 && (
        <Card size="small" title={`发现 ${elements.length} 个元素`}>
          <div style={{ maxHeight: 200, overflow: "auto" }}>
            {elements.slice(0, 20).map((el, i) => (
              <div
                key={i}
                style={{
                  padding: "4px 8px",
                  cursor: "pointer",
                  borderRadius: 4,
                }}
                onClick={() =>
                  executeClick(
                    el.bounds.x + el.bounds.width / 2,
                    el.bounds.y + el.bounds.height / 2
                  )
                }
              >
                <Typography.Text type="secondary" style={{ fontSize: 11 }}>
                  {el.role}
                </Typography.Text>{" "}
                <Typography.Text>{el.name || "(unnamed)"}</Typography.Text>
              </div>
            ))}
          </div>
        </Card>
      )}

      <Card size="small" title="快捷键">
        <Space wrap>
          <Button size="small" onClick={() => handlePressKey("Enter")}>Enter</Button>
          <Button size="small" onClick={() => handlePressKey("Tab")}>Tab</Button>
          <Button size="small" onClick={() => handlePressKey("Escape")}>Esc</Button>
          <Button size="small" onClick={() => handlePressKey("a", ["control"])}>Ctrl+A</Button>
          <Button size="small" onClick={() => handlePressKey("c", ["control"])}>Ctrl+C</Button>
          <Button size="small" onClick={() => handlePressKey("v", ["control"])}>Ctrl+V</Button>
          <Button size="small" onClick={() => handlePressKey("z", ["control"])}>Ctrl+Z</Button>
        </Space>
      </Card>
    </div>
  );
}
