// SPDX-License-Identifier: AGPL-3.0-only

import { useSkillExtensionStore } from "@/stores";
import { Spin } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useParams } from "react-router-dom";
import { SkillPageRenderer } from "./SkillPageRenderer";

export function SkillPageByParam() {
  const { t } = useTranslation();
  const { skillName, pageId } = useParams<{
    skillName: string;
    pageId?: string;
  }>();
  const panels = useSkillExtensionStore((s) => s.panels);
  const skills = useSkillExtensionStore((s) => s.skills);
  const fetchSkills = useSkillExtensionStore((s) => s.fetchSkills);
  const loading = useSkillExtensionStore((s) => s.loading);
  const [notFound, setNotFound] = useState(false);

  useEffect(() => {
    if (!skillName) {
      return;
    }
    if (skills.length === 0 && !loading) {
      fetchSkills();
    }
  }, [skillName, skills.length, loading, fetchSkills]);

  useEffect(() => {
    if (loading) {
      return;
    }
    const timer = setTimeout(() => {
      const found = panels.some((p: { skillName: string; id: string }) => {
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
  }, [loading, panels, skillName, pageId]);

  if (!skillName) {
    return (
      <div style={{ padding: 24, textAlign: "center" }}>
        {t("skill.noSkillName")}
      </div>
    );
  }

  const page = panels.find((p: { skillName: string; id: string }) => {
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
