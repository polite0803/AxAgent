// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { Button, Card, Empty, Input, message, Select, Spin, Table, Tag, Typography } from "antd";
import { Download, Search, Upload } from "lucide-react";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph, Title } = Typography;

interface SkillsHubSkill {
  id: string;
  name: string;
  description: string;
  category: string;
  author: string;
  version: string;
  tags: string[];
  downloads: number;
  rating: number;
}

interface SkillsHubSearchResult {
  skills: SkillsHubSkill[];
  total: number;
  page: number;
  page_size: number;
}

const CATEGORIES = [
  { value: "all", label: "All Categories" },
  { value: "code", label: "Code Generation" },
  { value: "debug", label: "Debugging" },
  { value: "refactor", label: "Refactoring" },
  { value: "test", label: "Testing" },
  { value: "docs", label: "Documentation" },
  { value: "security", label: "Security" },
  { value: "performance", label: "Performance" },
  { value: "database", label: "Database" },
  { value: "api", label: "API Development" },
  { value: "cloud", label: "Cloud & DevOps" },
  { value: "ai", label: "AI & ML" },
];

export function SkillsHubSettings() {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState("");
  const [category, setCategory] = useState("all");
  const [loading, setLoading] = useState(false);
  const [searchResult, setSearchResult] = useState<SkillsHubSearchResult | null>(null);
  const [installingId, setInstallingId] = useState<string | null>(null);
  const [installedSkills, setInstalledSkills] = useState<Set<string>>(
    new Set(),
  );

  const doSearch = useCallback(async (page = 1) => {
    setLoading(true);
    try {
      const result = await invoke<SkillsHubSearchResult>("skills_hub_search", {
        query: searchQuery || "",
        category: category === "all" ? null : category,
        page,
        page_size: 20,
      });
      setSearchResult(result);
    } catch (error) {
      message.error(`Search failed: ${error}`);
      setSearchResult({
        skills: [],
        total: 0,
        page: page,
        page_size: 20,
      });
    } finally {
      setLoading(false);
    }
  }, [searchQuery, category]);

  const handleSearch = () => doSearch(1);
  const handlePageChange = (page: number) => doSearch(page);

  const handleInstall = async (skill: SkillsHubSkill) => {
    setInstallingId(skill.id);
    try {
      await invoke("skills_hub_install", { skillId: skill.id });
      message.success(`Installed ${skill.name}`);
      setInstalledSkills((prev) => new Set([...prev, skill.id]));
    } catch (error) {
      message.error(`Install failed: ${error}`);
    } finally {
      setInstallingId(null);
    }
  };

  // P1 #13: 导出 Skill 处理函数
  const handleExportSkill = useCallback(async () => {
    try {
      const result = await invoke<{ skills: Array<{ name: string }> }>("list_skills");
      const availableSkills = result?.skills ?? [];
      if (availableSkills.length === 0) {
        message.warning(t("settings.skillsHub.noSkillsToExport"));
        return;
      }
      // 导出第一个 Skill 作为示例，后续可扩展为多选 UI
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
      message.success(`Exported ${skillName}`);
    } catch (e) {
      console.error("Export failed:", e);
      message.error(`Export failed: ${e}`);
    }
  }, [t]);

  // P1 #13: 导入 Skill 处理函数
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
        message.success(`Imported ${skillData.name}`);
      } catch (e) {
        console.error("Import failed:", e);
        message.error(`Import failed: ${e}`);
      }
    };
    input.click();
  }, [t]);

  const columns = [
    {
      title: t("settings.skillsHub.name"),
      dataIndex: "name",
      key: "name",
      width: 200,
      render: (name: string, record: SkillsHubSkill) => (
        <div>
          <Text strong>{name}</Text>
          <br />
          <Text type="secondary" className="text-xs">
            v{record.version}
          </Text>
        </div>
      ),
    },
    {
      title: t("settings.skillsHub.description"),
      dataIndex: "description",
      key: "description",
      ellipsis: true,
    },
    {
      title: t("settings.skillsHub.category"),
      dataIndex: "category",
      key: "category",
      width: 120,
      render: (cat: string) => <Tag color="blue">{cat}</Tag>,
    },
    {
      title: t("settings.skillsHub.author"),
      dataIndex: "author",
      key: "author",
      width: 120,
      ellipsis: true,
    },
    {
      title: t("settings.skillsHub.downloads"),
      dataIndex: "downloads",
      key: "downloads",
      width: 100,
      render: (n: number) => n.toLocaleString(),
    },
    {
      title: t("settings.skillsHub.rating"),
      dataIndex: "rating",
      key: "rating",
      width: 80,
      render: (r: number) => <span className="text-yellow-500">{"★".repeat(Math.round(r))}</span>,
    },
    {
      title: "",
      key: "actions",
      width: 120,
      render: (_: unknown, record: SkillsHubSkill) =>
        installedSkills.has(record.id) ? <Tag color="green">{t("settings.skillsHub.installed")}</Tag> : (
          <Button
            type="primary"
            size="small"
            icon={<Download size={14} />}
            onClick={() => handleInstall(record)}
            loading={installingId === record.id}
          >
            {t("settings.skillsHub.install")}
          </Button>
        ),
    },
  ];

  return (
    <div>
      <Title level={4}>{t("settings.skillsHub.title")}</Title>
      <Paragraph type="secondary" className="mb-6">
        {t("settings.skillsHub.description")}
      </Paragraph>

      <Card className="mb-6">
        <div className="flex gap-3 flex-wrap">
          <Input
            id="skills-hub-settings-input-174"
            placeholder={t("settings.skillsHub.searchPlaceholder")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onPressEnter={handleSearch}
            prefix={<Search size={16} />}
            className="flex-1 min-w-50"
          />
          <Select
            id="skills-hub-settings-select-175"
            value={category}
            onChange={setCategory}
            options={CATEGORIES}
            className="w-40"
          />
          <Button type="primary" onClick={handleSearch} loading={loading}>
            {t("settings.skillsHub.search")}
          </Button>
        </div>
      </Card>

      {loading
        ? (
          <div className="flex items-center justify-center h-48">
            <Spin size="large" />
          </div>
        )
        : searchResult
        ? (
          <>
            <div className="mb-4">
              <Text type="secondary">
                {t("settings.skillsHub.results", { count: searchResult.total })}
              </Text>
            </div>
            {searchResult.skills.length > 0
              ? (
                <Table
                  dataSource={searchResult.skills}
                  columns={columns}
                  rowKey="id"
                  pagination={{
                    total: searchResult.total,
                    pageSize: searchResult.page_size,
                    current: searchResult.page,
                    onChange: handlePageChange,
                  }}
                />
              )
              : <Empty description={t("settings.skillsHub.noResults")} />}
          </>
        )
        : <Empty description={t("settings.skillsHub.getStarted")} />}

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
