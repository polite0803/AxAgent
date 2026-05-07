/**
 * SkillDependencyCheck — Skill 依赖管理 UI
 *
 * 在 SkillsPage 中使用，展示 Skill 之间的依赖关系和缺失警告。
 *
 * @module components/skill/SkillDependencyCheck
 */

import { useSkillStore } from "@/stores";
import { Alert, Badge, Button, List, Space, Tag, Typography } from "antd";
import { AlertTriangle, CheckCircle, RefreshCw, XCircle } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface DependencyInfo {
  skillName: string;
  required: boolean;
  installed: boolean;
  versionConstraint?: string;
}

interface DependencyReport {
  skillName: string;
  satisfied: boolean;
  dependencies: DependencyInfo[];
}

export function SkillDependencyCheck() {
  const { t } = useTranslation();
  const skills = useSkillStore((s) => s.skills);
  const [reports, setReports] = useState<DependencyReport[]>([]);
  const [loading, setLoading] = useState(false);

  const analyzeDependencies = useCallback(() => {
    setLoading(true);
    try {
      const results: DependencyReport[] = [];
      const installedNames = new Set(skills.map((s) => s.name));

      for (const skill of skills) {
        const deps = skill.manifest?.dependencies;
        if (!deps) { continue; }

        const depInfo: DependencyInfo[] = [];
        let allSatisfied = true;

        for (const [depName, versionConstraint] of Object.entries(deps)) {
          const installed = installedNames.has(depName);
          if (!installed) { allSatisfied = false; }
          depInfo.push({
            skillName: depName,
            required: true,
            installed,
            versionConstraint,
          });
        }

        if (depInfo.length > 0) {
          results.push({
            skillName: skill.name,
            satisfied: allSatisfied,
            dependencies: depInfo,
          });
        }
      }
      setReports(results);
    } finally {
      setLoading(false);
    }
  }, [skills]);

  useEffect(() => {
    analyzeDependencies();
  }, [analyzeDependencies]);

  if (reports.length === 0) {
    return (
      <Alert
        type="success"
        showIcon
        icon={<CheckCircle size={14} />}
        message={t("skill.deps.allSatisfied", "所有技能依赖已满足")}
        style={{ marginBottom: 16 }}
      />
    );
  }

  return (
    <div style={{ marginBottom: 16 }}>
      <Space style={{ marginBottom: 12 }}>
        <Typography.Text strong>
          {t("skill.deps.title", "依赖检查")}
        </Typography.Text>
        <Button size="small" icon={<RefreshCw size={12} />} loading={loading} onClick={analyzeDependencies}>
          {t("skill.deps.refresh", "刷新")}
        </Button>
      </Space>

      {reports.map((report) => (
        <Alert
          key={report.skillName}
          type={report.satisfied ? "success" : "warning"}
          showIcon
          icon={report.satisfied ? <CheckCircle size={14} /> : <AlertTriangle size={14} />}
          message={
            <span>
              <Typography.Text strong>{report.skillName}</Typography.Text>
              <Badge
                count={report.dependencies.filter((d) => !d.installed).length}
                size="small"
                style={{ marginLeft: 8 }}
                overflowCount={99}
              />
            </span>
          }
          description={
            <List
              size="small"
              dataSource={report.dependencies}
              renderItem={(dep) => (
                <List.Item style={{ padding: "2px 0", border: "none" }}>
                  <Space size={4}>
                    {dep.installed
                      ? <CheckCircle size={12} style={{ color: "var(--color-success)" }} />
                      : <XCircle size={12} style={{ color: "var(--color-error)" }} />}
                    <Typography.Text
                      style={{ color: dep.installed ? undefined : "var(--color-error)" }}
                    >
                      {dep.skillName}
                    </Typography.Text>
                    {dep.versionConstraint && <Tag style={{ fontSize: 11 }}>{dep.versionConstraint}</Tag>}
                    <Badge
                      status={dep.installed ? "success" : "error"}
                      text={dep.installed
                        ? t("skill.deps.installed", "已安装")
                        : t("skill.deps.missing", "缺失")}
                    />
                  </Space>
                </List.Item>
              )}
              style={{ marginTop: 4 }}
            />
          }
          style={{ marginBottom: 8 }}
        />
      ))}
    </div>
  );
}
