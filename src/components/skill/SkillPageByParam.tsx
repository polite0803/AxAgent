import { useSkillExtensionStore } from "@/stores";
import { Spin } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useParams } from "react-router-dom";
import { SkillPageRenderer } from "./SkillPageRenderer";

export default function SkillPageByParam() {
  const { t } = useTranslation();
  const { skillName, pageId } = useParams<{
    skillName: string;
    pageId?: string;
  }>();
  const pages = useSkillExtensionStore((s) => s.pages);
  const fetchSkills = useSkillExtensionStore((s) => s.fetchSkills);
  const loading = useSkillExtensionStore((s) => s.loading);
  const [notFound, setNotFound] = useState(false);

  useEffect(() => {
    if (!skillName) {
      return;
    }
    // 如果 store 为空且不在加载中，触发 fetch
    if (pages.length === 0 && !loading) {
      fetchSkills();
    }
  }, [skillName, pages.length, loading, fetchSkills]);

  useEffect(() => {
    // 加载完成后检查页面是否存在，超时后显示 404
    if (loading) {
      return;
    }
    const timer = setTimeout(() => {
      const found = pages.some((p) => {
        if (pageId) {
          return p.skillName === skillName && p.id === pageId;
        }
        return p.skillName === skillName;
      });
      if (!found) {
        setNotFound(true);
      }
    }, 1000);
    return () => clearTimeout(timer);
  }, [loading, pages, skillName, pageId]);

  if (!skillName) {
    return (
      <div style={{ padding: 24, textAlign: "center" }}>
        {t("skill.noSkillName")}
      </div>
    );
  }

  const page = pages.find((p) => {
    if (pageId) {
      return p.skillName === skillName && p.id === pageId;
    }
    return p.skillName === skillName;
  });

  if (!page) {
    if (loading) {
      return (
        <div
          style={{
            padding: 48,
            textAlign: "center",
            color: "var(--color-text-secondary)",
          }}
        >
          <Spin size="large" />
        </div>
      );
    }
    if (notFound) {
      return (
        <div
          style={{
            padding: 48,
            textAlign: "center",
            color: "var(--color-text-secondary)",
          }}
        >
          {t("skill.notFound", {
            skillName,
            pageId: pageId ? `/${pageId}` : "",
          })}
        </div>
      );
    }
    return (
      <div
        style={{
          padding: 48,
          textAlign: "center",
          color: "var(--color-text-secondary)",
        }}
      >
        <Spin size="large" />
      </div>
    );
  }

  return (
    <SkillPageRenderer
      componentType={page.componentType}
      componentConfig={page.componentConfig}
      skillName={page.skillName}
    />
  );
}

export { SkillPageByParam };
