import { invoke } from "@/lib/invoke";
import { Button, Card, Empty, Input, message, Select, Spin, Table, Tag, Typography } from "antd";
import { Download, Search, Upload } from "lucide-react";
import { useState } from "react";
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

export default function SkillsHubSettings() {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState("");
  const [category, setCategory] = useState("all");
  const [loading, setLoading] = useState(false);
  const [searchResult, setSearchResult] = useState<SkillsHubSearchResult | null>(null);
  const [installingId, setInstallingId] = useState<string | null>(null);
  const [installedSkills, setInstalledSkills] = useState<Set<string>>(new Set());

  const handleSearch = async () => {
    setLoading(true);
    try {
      const result = await invoke<SkillsHubSearchResult>("skills_hub_search", {
        query: searchQuery || "",
        category: category === "all" ? null : category,
        page: 1,
        page_size: 20,
      });
      setSearchResult(result);
    } catch (error) {
      message.error(`Search failed: ${error}`);
      setSearchResult({
        skills: [],
        total: 0,
        page: 1,
        page_size: 20,
      });
    } finally {
      setLoading(false);
    }
  };

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
          <Text type="secondary" className="text-xs">v{record.version}</Text>
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
    <div className="max-w-5xl">
      <Title level={4}>{t("settings.skillsHub.title")}</Title>
      <Paragraph type="secondary" className="mb-6">
        {t("settings.skillsHub.description")}
      </Paragraph>

      <Card className="mb-6">
        <div className="flex gap-3 flex-wrap">
          <Input
            placeholder={t("settings.skillsHub.searchPlaceholder")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onPressEnter={handleSearch}
            prefix={<Search size={16} />}
            className="flex-1 min-w-50"
          />
          <Select
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
                    onChange: (_page) => {
                    },
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
          <Button icon={<Upload size={16} />}>{t("settings.skillsHub.exportSkill")}</Button>
          <Button icon={<Download size={16} />}>{t("settings.skillsHub.importSkill")}</Button>
        </div>
      </Card>
    </div>
  );
}
