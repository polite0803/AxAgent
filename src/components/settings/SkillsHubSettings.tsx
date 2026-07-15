// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { message } from "@/lib/toast";
import { Button, Card, Typography } from "antd";
import { Download, Upload } from "lucide-react";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";

const { Title, Paragraph } = Typography;

/**
 * Skills Hub — 仅保留本地导入导出功能。
 * 搜索/安装功能已移除（原后端 api.agentskills.io 不存在）。
 */
export function SkillsHubSettings() {
  const { t } = useTranslation();

  const handleExportSkill = useCallback(async () => {
    try {
      const result = await invoke<{ skills: Array<{ name: string }> }>("list_skills");
      const availableSkills = result?.skills ?? [];
      if (availableSkills.length === 0) {
        message.warning(t("settings.skillsHub.noSkillsToExport"));
        return;
      }
      const skillName = availableSkills[0].name;
      const detail = await invoke("get_skill", { name: skillName });
      const blob = new Blob([JSON.stringify(detail, null, 2)], {
        type: "application/json",
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${skillName}.skill.json`;
      a.click();
      URL.revokeObjectURL(url);
      message.success(t("settings.skillsHub.exported", { name: skillName }));
    } catch (e) {
      console.error("Export failed:", e);
      message.error(t("settings.skillsHub.exportFailed", { error: String(e) }));
    }
  }, [t]);

  const handleImportSkill = useCallback(async () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".skill.json";
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) { return; }
      try {
        const text = await file.text();
        const skillData = JSON.parse(text);
        if (!skillData.name) {
          message.error(t("settings.skillsHub.invalidSkillFile"));
          return;
        }
        await invoke("install_skill", {
          name: skillData.name,
          sourcePath: skillData.sourcePath ?? skillData.name,
        });
        message.success(t("settings.skillsHub.imported", { name: skillData.name }));
      } catch (e) {
        console.error("Import failed:", e);
        message.error(t("settings.skillsHub.importFailed", { error: String(e) }));
      }
    };
    input.click();
  }, [t]);

  return (
    <div>
      <Title level={4}>{t("settings.skillsHub.title")}</Title>
      <Paragraph type="secondary" className="mb-6">
        {t("settings.skillsHub.description")}
      </Paragraph>

      <Card className="mt-6">
        <Title level={5}>{t("settings.skillsHub.mySkills")}</Title>
        <Paragraph type="secondary" className="mb-4">
          {t("settings.skillsHub.mySkillsDescription")}
        </Paragraph>
        <div className="flex gap-3">
          <Button
            icon={<Upload size={16} />}
            onClick={handleExportSkill}
          >
            {t("settings.skillsHub.exportSkill")}
          </Button>
          <Button
            icon={<Download size={16} />}
            onClick={handleImportSkill}
          >
            {t("settings.skillsHub.importSkill")}
          </Button>
        </div>
      </Card>
    </div>
  );
}
