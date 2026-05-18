import { invoke } from "@/lib/invoke";
import { executeActionChain } from "@/lib/skillActionExecutor";
import { resolveIconComponent } from "@/lib/skillIcons";
import { useSkillExtensionStore } from "@/stores";
import type { MergedStatusBarItem } from "@/stores/feature/skillExtensionStore";
import { Typography } from "antd";
import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";

interface SkillStatusBarProps {
  alignment: "left" | "right";
}

export function SkillStatusBar({ alignment }: SkillStatusBarProps) {
  const statusBarItems = useSkillExtensionStore((s) => s.statusBarItems);

  const items = statusBarItems
    .filter((item) => item.alignment === alignment)
    .sort((a, b) => a.priority - b.priority);

  if (items.length === 0) {
    return null;
  }

  return (
    <div
      style={{ display: "flex", alignItems: "center", gap: 8, height: "100%" }}
    >
      {items.map((item) => <StatusBarItem key={`${item.skillName}:${item.id}`} item={item} />)}
    </div>
  );
}

function StatusBarItem({ item }: { item: MergedStatusBarItem }) {
  const navigate = useNavigate();
  const [dynamicValue, setDynamicValue] = useState<string | null>(null);
  const failCountRef = useRef(0);

  // 动态轮询（带退避）
  useEffect(() => {
    if (!item.dynamicText) {
      return;
    }
    const { command, args, refreshIntervalMs } = item.dynamicText;
    let timer: ReturnType<typeof setTimeout>;

    const fetchValue = async () => {
      try {
        const result = await invoke<Record<string, unknown>>(
          command,
          args || {},
        );
        const template = item.dynamicText!.template || "{{value}}";
        const val = result?.value ?? result?.count ?? Object.values(result || {})[0];
        setDynamicValue(template.replace("{{value}}", String(val ?? "")));
        failCountRef.current = 0;
      } catch {
        setDynamicValue("--");
        failCountRef.current += 1;
      }
      // 连续失败时指数退避，最大 5 分钟
      const backoff = Math.min(
        Math.max(refreshIntervalMs, 5000) * Math.pow(2, failCountRef.current),
        300000,
      );
      timer = setTimeout(fetchValue, backoff);
    };
    fetchValue();
    return () => clearTimeout(timer);
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
