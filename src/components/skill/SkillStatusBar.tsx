import { invoke } from "@/lib/invoke";
import { executeActionChain } from "@/lib/skillActionExecutor";
import { resolveIconComponent } from "@/lib/skillIcons";
import { useSkillExtensionStore } from "@/stores";
import type { MergedStatusBarItem } from "@/stores/feature/skillExtensionStore";
import { Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";

interface SkillStatusBarProps {
  alignment: "left" | "right";
}

export function SkillStatusBar({ alignment }: SkillStatusBarProps) {
  const statusBarItems = useSkillExtensionStore((s) => s.statusBarItems);

  const items = statusBarItems
    .filter((item) => item.alignment === alignment)
    .sort((a, b) => a.priority - b.priority);

  if (items.length === 0) { return null; }

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8, height: "100%" }}>
      {items.map((item) => <StatusBarItem key={`${item.skillName}:${item.id}`} item={item} />)}
    </div>
  );
}

function StatusBarItem({ item }: { item: MergedStatusBarItem }) {
  const navigate = useNavigate();
  const [dynamicValue, setDynamicValue] = useState<string | null>(null);

  // 动态轮询
  useEffect(() => {
    if (!item.dynamicText) { return; }
    const { command, args, refreshIntervalMs } = item.dynamicText;
    const fetchValue = async () => {
      try {
        const result = await invoke<Record<string, unknown>>(command, args || {});
        const template = item.dynamicText!.template || "{{value}}";
        const val = result?.value ?? result?.count ?? Object.values(result || {})[0];
        setDynamicValue(template.replace("{{value}}", String(val ?? "")));
      } catch {
        setDynamicValue("--");
      }
    };
    fetchValue();
    const timer = setInterval(fetchValue, Math.max(refreshIntervalMs, 5000));
    return () => clearInterval(timer);
  }, [item.dynamicText]);

  const handleClick = useCallback(() => {
    if (item.onClick && item.onClick.length > 0) {
      executeActionChain(item.onClick, navigate);
    }
  }, [item.onClick, navigate]);

  const IconComp = item.icon ? resolveIconComponent(item.icon) : undefined;
  const displayText = dynamicValue ?? item.text ?? "";

  return (
    <Typography.Text
      style={{
        fontSize: 12,
        color: "var(--color-text-secondary)",
        cursor: item.onClick ? "pointer" : "default",
        display: "inline-flex",
        alignItems: "center",
        gap: 4,
        whiteSpace: "nowrap",
      }}
      onClick={handleClick}
    >
      {IconComp && <IconComp size={12} />}
      {displayText}
    </Typography.Text>
  );
}
