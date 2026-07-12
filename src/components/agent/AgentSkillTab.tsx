// SPDX-License-Identifier: AGPL-3.0-only

import { CheckCircleOutlined, CloseCircleOutlined, LoadingOutlined } from "@ant-design/icons";
import { Empty, Input, Switch, Tag, Typography } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

/**
 * 技能管理标签页
 *
 * 展示已安装的技能列表，支持启用/禁用切换。
 * 复用 skillStore 的数据和 toggleSkill 方法。
 */
export function AgentSkillTab() {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  const [skills, setSkills] = useState<Array<{ name: string; description: string; enabled: boolean }>>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    let unsub: (() => void) | undefined;
    (async () => {
      try {
        const { useSkillStore } = await import("@/stores");
        const store = useSkillStore.getState();
        if (store.skills.length === 0) {
          await store.loadSkills();
        }
        if (!cancelled) {
          setSkills(useSkillStore.getState().skills);
          setLoading(false);
          unsub = useSkillStore.subscribe((s) => {
            if (!cancelled) {
              setSkills(s.skills);
            }
          });
        }
      } catch {
        if (!cancelled) { setLoading(false); }
      }
    })();
    return () => {
      cancelled = true;
      unsub?.();
    };
  }, []);

  const handleToggle = async (name: string, enabled: boolean) => {
    try {
      const { useSkillStore } = await import("@/stores");
      await useSkillStore.getState().toggleSkill(name, enabled);
    } catch {
      // 静默失败
    }
  };

  const filtered = skills.filter((s) => {
    if (!search.trim()) { return true; }
    const q = search.toLowerCase();
    return s.name.toLowerCase().includes(q) || s.description.toLowerCase().includes(q);
  });

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full p-6">
        <LoadingOutlined style={{ fontSize: 24, color: "var(--color-text-secondary)" }} />
      </div>
    );
  }

  if (skills.length === 0) {
    return (
      <div className="flex items-center justify-center h-full p-6">
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={
            <span className="text-[var(--color-text-secondary)]">
              {t("agentPanel.skillComingSoon")}
            </span>
          }
        />
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="px-3 pt-2 pb-1 shrink-0">
        <Input.Search
          size="small"
          placeholder={t("common.search")}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          allowClear
        />
      </div>
      <div className="flex-1 overflow-auto px-2 pb-2">
        {filtered.map((skill) => (
          <div
            key={skill.name}
            className="flex items-center justify-between px-2 py-1.5 rounded hover:bg-[var(--color-fill-alter)] transition-colors"
          >
            <div className="flex-1 min-w-0 mr-2">
              <div className="flex items-center gap-1.5">
                <Text className="text-sm font-medium truncate">{skill.name}</Text>
                {skill.enabled
                  ? (
                    <Tag
                      color="success"
                      style={{ fontSize: 10, lineHeight: "16px", padding: "0 4px", marginInlineEnd: 0 }}
                    >
                      <CheckCircleOutlined /> {t("common.enabled")}
                    </Tag>
                  )
                  : (
                    <Tag style={{ fontSize: 10, lineHeight: "16px", padding: "0 4px", marginInlineEnd: 0 }}>
                      <CloseCircleOutlined /> {t("common.disabled")}
                    </Tag>
                  )}
              </div>
              <Text type="secondary" className="text-xs block truncate">{skill.description}</Text>
            </div>
            <Switch
              size="small"
              checked={skill.enabled}
              onChange={(checked) => handleToggle(skill.name, checked)}
            />
          </div>
        ))}
        {filtered.length === 0 && (
          <div className="flex items-center justify-center h-20">
            <Text type="secondary" className="text-xs">{t("common.noResults")}</Text>
          </div>
        )}
      </div>
    </div>
  );
}
