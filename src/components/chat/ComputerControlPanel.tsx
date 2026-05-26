import { Tooltip } from "@/components/layout/Tooltip";
import { invoke } from "@/lib/invoke";
import { Button, Card, Input, message, Space, Switch, Typography } from "antd";
import { Scissors, Search } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

interface CaptureResult {
  image_base64: string;
  width: number;
  height: number;
  monitor_index?: number;
  scale_factor?: number;
}

interface UIElement {
  role: string;
  name: string;
  bounds: { x: number; y: number; width: number; height: number };
  is_clickable: boolean;
}

export function ComputerControlPanel() {
  const { t } = useTranslation();
  const mountedRef = useRef(true);
  useEffect(
    () => () => {
      mountedRef.current = false;
    },
    [],
  );
  const [screenshot, setScreenshot] = useState<string | null>(null);
  const [autoMode, setAutoMode] = useState(false);
  const [elements, setElements] = useState<UIElement[]>([]);
  const [loading, setLoading] = useState(false);
  const [clickCoords, setClickCoords] = useState<
    {
      x: number;
      y: number;
    } | null
  >(null);
  const [nativeResolution, setNativeResolution] = useState({
    width: 1920,
    height: 1080,
  });
  const [dpiScale, setDpiScale] = useState(1.0);
  const imgRef = useRef<HTMLImageElement>(null);

  const handleCapture = async () => {
    setLoading(true);
    try {
      const result = await invoke<CaptureResult>("screen_capture", {
        monitor: 0,
      });
      setNativeResolution({ width: result.width, height: result.height });
      setScreenshot(`data:image/png;base64,${result.image_base64}`);
      if (result.scale_factor && result.scale_factor > 0) {
        setDpiScale(result.scale_factor);
      }
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
    if (!imgRef.current) {
      return;
    }
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
      message.success(t("computerControl.clickSuccess", { x, y }));
      setTimeout(() => {
        if (mountedRef.current) {
          handleCapture();
        }
      }, 500);
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleTypeText = async (text: string, x?: number, y?: number) => {
    try {
      await invoke("type_text", { text, x, y });
      message.success(t("computerControl.typeComplete"));
    } catch (e) {
      message.error(String(e));
    }
  };

  const handlePressKey = async (key: string, modifiers?: string[]) => {
    try {
      await invoke("press_key", { key, modifiers: modifiers || [] });
      message.success(t("computerControl.keyPressed", { key }));
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleScroll = async (x: number, y: number, delta: number) => {
    try {
      await invoke("mouse_scroll", { x, y, delta });
      message.success(
        t("computerControl.scrolled", {
          direction: delta > 0 ? t("computerControl.down") : t("computerControl.up"),
        }),
      );
    } catch (e) {
      message.error(String(e));
    }
  };

  return (
    <div
      style={{ padding: 16, display: "flex", flexDirection: "column", gap: 12 }}
    >
      <Space>
        <Button
          icon={<Scissors size={14} />}
          onClick={handleCapture}
          loading={loading}
        >
          {t("computerControl.capture")}
        </Button>
        <Button
          icon={<Search size={14} />}
          onClick={() => handleFindElements()}
        >
          {t("computerControl.findElement")}
        </Button>
        <Tooltip title={t("computerControl.autoModeTooltip")}>
          <Switch
            checked={autoMode}
            onChange={setAutoMode}
            checkedChildren={t("computerControl.auto")}
            unCheckedChildren={t("computerControl.manual")}
          />
        </Tooltip>
        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          {t("computerControl.resolution")}: {nativeResolution.width}x
          {nativeResolution.height}
          {dpiScale !== 1.0 && ` (${Math.round(dpiScale * 100)}% DPI)`}
        </Typography.Text>
      </Space>

      {screenshot && (
        <Card size="small" styles={{ body: { padding: 0 } }}>
          <div style={{ position: "relative", cursor: "crosshair" }}>
            <img
              ref={imgRef}
              src={screenshot}
              onClick={handleImageClick}
              role="button"
              tabIndex={0}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  if (imgRef.current) {
                    const rect = imgRef.current.getBoundingClientRect();
                    const scaleX = nativeResolution.width / rect.width;
                    const scaleY = nativeResolution.height / rect.height;
                    const x = Math.round((rect.width / 2) * scaleX);
                    const y = Math.round((rect.height / 2) * scaleY);
                    setClickCoords({ x, y });
                  }
                }
              }}
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
            {elements.map((el, _i) => (
              <div
                key={`${el.role}-${el.name}-${el.bounds.x}-${el.bounds.y}`}
                role="button"
                tabIndex={el.is_clickable ? 0 : -1}
                onKeyDown={(e) => {
                  if ((e.key === "Enter" || e.key === " ") && el.is_clickable) {
                    e.preventDefault();
                    e.stopPropagation();
                    executeClick(
                      el.bounds.x + el.bounds.width / 2,
                      el.bounds.y + el.bounds.height / 2,
                    );
                  }
                }}
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
                    el.bounds.y + el.bounds.height / 2,
                  );
                }}
                title={`${el.role}: ${el.name}`}
              />
            ))}
          </div>
        </Card>
      )}

      {clickCoords && (
        <Card size="small" title={t("computerControl.coordOps")}>
          <Space direction="vertical">
            <Typography.Text>
              {t("computerControl.coordinates")}: ({clickCoords.x}, {clickCoords.y})
            </Typography.Text>
            <Space>
              <Button
                size="small"
                type="primary"
                onClick={() => executeClick(clickCoords.x, clickCoords.y)}
              >
                {t("computerControl.executeClick")}
              </Button>
              <Input
                id="computer-control-panel-input-9"
                placeholder={t("computerControl.typePlaceholder")}
                style={{ width: 200 }}
                onPressEnter={(e) =>
                  handleTypeText(
                    e.currentTarget.value,
                    clickCoords.x,
                    clickCoords.y,
                  )}
              />
              <Button
                size="small"
                onClick={() => handleScroll(clickCoords.x, clickCoords.y, -3)}
              >
                {t("computerControl.scrollUp")}
              </Button>
              <Button
                size="small"
                onClick={() => handleScroll(clickCoords.x, clickCoords.y, 3)}
              >
                {t("computerControl.scrollDown")}
              </Button>
            </Space>
          </Space>
        </Card>
      )}

      {elements.length > 0 && (
        <Card
          size="small"
          title={t("computerControl.foundElements", { count: elements.length })}
        >
          <div style={{ maxHeight: 200, overflow: "auto" }}>
            {elements.slice(0, 20).map((el, _i) => (
              <div
                key={`${el.role}-${el.name}-${el.bounds.x}-${el.bounds.y}`}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    executeClick(
                      el.bounds.x + el.bounds.width / 2,
                      el.bounds.y + el.bounds.height / 2,
                    );
                  }
                }}
                style={{
                  padding: "4px 8px",
                  cursor: "pointer",
                  borderRadius: 4,
                }}
                onClick={() =>
                  executeClick(
                    el.bounds.x + el.bounds.width / 2,
                    el.bounds.y + el.bounds.height / 2,
                  )}
              >
                <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                  {el.role}
                </Typography.Text>{" "}
                <Typography.Text>
                  {el.name || t("computerControl.unnamed")}
                </Typography.Text>
              </div>
            ))}
          </div>
        </Card>
      )}

      <Card size="small" title={t("computerControl.shortcuts")}>
        <Space wrap>
          <Button size="small" onClick={() => handlePressKey("Enter")}>
            Enter
          </Button>
          <Button size="small" onClick={() => handlePressKey("Tab")}>
            Tab
          </Button>
          <Button size="small" onClick={() => handlePressKey("Escape")}>
            Esc
          </Button>
          <Button size="small" onClick={() => handlePressKey("a", ["control"])}>
            Ctrl+A
          </Button>
          <Button size="small" onClick={() => handlePressKey("c", ["control"])}>
            Ctrl+C
          </Button>
          <Button size="small" onClick={() => handlePressKey("v", ["control"])}>
            Ctrl+V
          </Button>
          <Button size="small" onClick={() => handlePressKey("z", ["control"])}>
            Ctrl+Z
          </Button>
        </Space>
      </Card>
    </div>
  );
}
