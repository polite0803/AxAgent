import { Badge, Card, Typography } from "antd";
import { CheckCircle, Cpu, Eye, Loader2, Monitor, MousePointer2, XCircle } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;

interface UiElement {
  element_type: string;
  label: string | null;
  bounding_box: { x: number; y: number; width: number; height: number } | null;
  actionable: boolean;
}

interface UISnapshotViewerProps {
  imageUrl?: string;
  elements: UiElement[];
  rawDescription: string;
  loading?: boolean;
  error?: string | null;
  onElementClick?: (element: UiElement) => void;
}

const ELEMENT_ICONS: Record<string, React.ReactNode> = {
  button: <MousePointer2 size={12} />,
  input: <Monitor size={12} />,
  link: <MousePointer2 size={12} />,
  menu: <Cpu size={12} />,
  checkbox: <CheckCircle size={12} />,
  toggle: <Cpu size={12} />,
};

const ACTIONABLE_COLOR = "#52c41a";
const NON_ACTIONABLE_COLOR = "#8c8c8c";

function UISnapshotViewer({
  imageUrl,
  elements,
  rawDescription,
  loading,
  error,
  onElementClick,
}: UISnapshotViewerProps) {
  const { t } = useTranslation();
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);

  const stats = useMemo(() => {
    const actionable = elements.filter((el) => el.actionable).length;
    const byType: Record<string, number> = {};
    for (const el of elements) {
      byType[el.element_type] = (byType[el.element_type] || 0) + 1;
    }
    return { total: elements.length, actionable, byType };
  }, [elements]);

  if (loading) {
    return (
      <Card size="small">
        <div className="flex items-center gap-2 py-4 text-sm text-gray-500">
          <Loader2 size={14} className="animate-spin" />
          <span>{t("chat.vision.ui.analyzing")}</span>
        </div>
      </Card>
    );
  }

  if (error) {
    return (
      <Card size="small">
        <div className="flex items-center gap-2 py-2 text-sm text-red-500">
          <XCircle size={14} />
          <span>{error}</span>
        </div>
      </Card>
    );
  }

  return (
    <Card size="small" className="ui-snapshot-viewer">
      <div className="flex items-center gap-2 mb-3">
        <Monitor size={16} className="text-green-500" />
        <Title level={5} className="mb-0">{t("chat.vision.ui.title")}</Title>
      </div>

      {imageUrl && (
        <div className="mb-3 rounded overflow-hidden border border-gray-200 dark:border-gray-700 max-h-48 relative">
          <img
            src={imageUrl}
            alt={t("chat.vision.ui.screenshot")}
            className="w-full h-full object-contain bg-gray-100 dark:bg-gray-800"
          />
        </div>
      )}

      <div className="grid grid-cols-3 gap-2 mb-3">
        <Card size="small" className="bg-blue-50 dark:bg-blue-900/10 text-center">
          <Text className="text-lg font-bold text-blue-600 block">{stats.total}</Text>
          <Text type="secondary" className="text-xs">{t("chat.vision.ui.elements")}</Text>
        </Card>
        <Card size="small" className="bg-green-50 dark:bg-green-900/10 text-center">
          <Text className="text-lg font-bold text-green-600 block">{stats.actionable}</Text>
          <Text type="secondary" className="text-xs">{t("chat.vision.ui.actionable")}</Text>
        </Card>
        <Card size="small" className="bg-purple-50 dark:bg-purple-900/10 text-center">
          <Text className="text-lg font-bold text-purple-600 block">{Object.keys(stats.byType).length}</Text>
          <Text type="secondary" className="text-xs">{t("chat.vision.ui.types")}</Text>
        </Card>
      </div>

      <div>
        <Text strong className="text-sm block mb-1">
          {t("chat.vision.ui.elementList")} ({elements.length})
        </Text>
        <div className="space-y-1 max-h-64 overflow-auto">
          {elements.length === 0 && (
            <Text type="secondary" className="text-xs">
              {t("chat.vision.ui.noElements")}
            </Text>
          )}
          {elements.map((el, i) => (
            <div
              key={i}
              className={`flex items-center gap-2 px-2 py-1.5 rounded text-xs cursor-pointer transition-colors ${
                selectedIndex === i
                  ? "bg-green-50 dark:bg-green-900/20 ring-1 ring-green-300"
                  : "hover:bg-gray-50 dark:hover:bg-gray-800/50"
              }`}
              onClick={() => {
                setSelectedIndex(i);
                onElementClick?.(el);
              }}
            >
              <span style={{ color: el.actionable ? ACTIONABLE_COLOR : NON_ACTIONABLE_COLOR, display: "flex" }}>
                {ELEMENT_ICONS[el.element_type] || <Eye size={12} />}
              </span>
              <span className="font-medium">{el.element_type}</span>
              {el.label && <span className="text-gray-500 truncate flex-1">{el.label}</span>}
              <Badge
                status={el.actionable ? "success" : "default"}
                text={
                  <span className="text-xs">
                    {el.actionable ? t("chat.vision.ui.clickable") : t("chat.vision.ui.static")}
                  </span>
                }
              />
              {el.bounding_box && (
                <span className="text-gray-400 font-mono">
                  {el.bounding_box.x},{el.bounding_box.y}
                </span>
              )}
            </div>
          ))}
        </div>
      </div>

      {rawDescription && (
        <div className="mt-3">
          <Text strong className="text-sm block mb-1">{t("chat.vision.ui.description")}</Text>
          <Text className="text-sm text-gray-600 dark:text-gray-400">{rawDescription}</Text>
        </div>
      )}
    </Card>
  );
}

export default UISnapshotViewer;
