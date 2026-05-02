import { SavedFilter, SearchResult, useSearchStore } from "@/stores/feature/searchStore";
import {
  Button,
  Card,
  Checkbox,
  Empty,
  Input,
  List,
  Modal,
  Popconfirm,
  Select,
  Space,
  Tag,
  Tooltip,
  Typography,
} from "antd";
import { Clock, FileText, Filter, History, Regex, Search as SearchIcon, Star } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

interface SessionSearchPanelProps {
  visible: boolean;
  onClose: () => void;
  onSelectResult?: (result: SearchResult) => void;
}

export default function SessionSearchPanel({
  visible,
  onClose,
  onSelectResult,
}: SessionSearchPanelProps) {
  const { t } = useTranslation();
  const {
    query,
    results,
    isSearching,
    error,
    recentSearches,
    savedFilters,
    searchOptions,
    setQuery,
    setSearchOptions,
    search,
    clearRecentSearches,
    deleteFilter,
  } = useSearchStore();

  const [showFilters, setShowFilters] = useState(false);
  const [saveModalVisible, setSaveModalVisible] = useState(false);
  const [filterName, setFilterName] = useState("");

  useEffect(() => {
    if (visible) {
      setShowFilters(false);
    }
  }, [visible]);

  const handleSearch = () => {
    if (query.trim()) {
      search();
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      handleSearch();
    }
  };

  const handleSaveFilter = () => {
    if (!filterName.trim()) { return; }

    const filter: SavedFilter = {
      name: filterName,
      query,
      options: searchOptions,
    };

    useSearchStore.getState().saveFilter(filter);
    setSaveModalVisible(false);
    setFilterName("");
  };

  const handleLoadFilter = (filter: SavedFilter) => {
    setQuery(filter.query);
    setSearchOptions(filter.options);
    search();
  };

  const highlightText = (text: string, ranges: [number, number][]) => {
    if (!ranges || ranges.length === 0) { return text; }

    const parts: React.ReactNode[] = [];
    let lastEnd = 0;

    ranges.forEach(([start, end], index) => {
      if (start > lastEnd) {
        parts.push(text.slice(lastEnd, start));
      }
      parts.push(
        <mark
          key={index}
          style={{
            backgroundColor: "var(--accent-primary, #89b4fa)",
            color: "var(--background, #1e1e2e)",
            padding: "0 2px",
            borderRadius: 2,
          }}
        >
          {text.slice(start, end)}
        </mark>,
      );
      lastEnd = end;
    });

    if (lastEnd < text.length) {
      parts.push(text.slice(lastEnd));
    }

    return parts;
  };

  const renderResultItem = (result: SearchResult) => (
    <List.Item
      style={{
        padding: "12px 16px",
        cursor: "pointer",
        borderLeft: result.score > 2 ? "3px solid #a6e3a1" : "none",
      }}
      onClick={() => onSelectResult?.(result)}
    >
      <div style={{ width: "100%" }}>
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: 4,
          }}
        >
          <Space>
            <FileText size={14} />
            <Text strong style={{ fontSize: 13 }}>
              Session: {result.session_id.slice(0, 8)}...
            </Text>
            {result.agent_name && (
              <Tag color="blue" style={{ fontSize: 11 }}>
                {result.agent_name}
              </Tag>
            )}
          </Space>
          <Text type="secondary" style={{ fontSize: 11 }}>
            {result.timestamp}
          </Text>
        </div>
        <Paragraph
          ellipsis={{ rows: 2, expandable: false }}
          style={{
            margin: 0,
            fontSize: 13,
            fontFamily: "'JetBrains Mono', monospace",
          }}
        >
          {highlightText(result.content, result.highlight_ranges)}
        </Paragraph>
        <div style={{ marginTop: 4 }}>
          <Text type="secondary" style={{ fontSize: 11 }}>
            Score: {result.score.toFixed(2)} | Message #{result.message_index}
          </Text>
        </div>
      </div>
    </List.Item>
  );

  return (
    <Modal
      title={
        <Space>
          <SearchIcon size={18} />
          <span>{t("search.title")}</span>
        </Space>
      }
      open={visible}
      onCancel={onClose}
      width={800}
      footer={null}
      style={{ top: 20 }}
    >
      <div style={{ padding: "8px 0" }}>
        <Space direction="vertical" style={{ width: "100%" }} size="middle">
          <Space style={{ width: "100%" }}>
            <Input.Search
              placeholder={t("search.placeholder")}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onSearch={handleSearch}
              onPressEnter={handleKeyPress}
              style={{ flex: 1 }}
              size="large"
              loading={isSearching}
              allowClear
            />
            <Tooltip title={t("search.filters")}>
              <Button
                icon={<Filter size={16} />}
                onClick={() => setShowFilters(!showFilters)}
                type={showFilters ? "primary" : "default"}
              />
            </Tooltip>
          </Space>

          {showFilters && (
            <Card size="small" style={{ marginBottom: 8 }}>
              <Space direction="vertical" style={{ width: "100%" }} size="small">
                <div style={{ display: "flex", gap: 16, flexWrap: "wrap" }}>
                  <Checkbox
                    checked={searchOptions.useRegex}
                    onChange={(e) => setSearchOptions({ useRegex: e.target.checked })}
                  >
                    <Space>
                      <Regex size={14} />
                      <span>{t("search.useRegex")}</span>
                    </Space>
                  </Checkbox>
                  <Checkbox
                    checked={searchOptions.caseSensitive}
                    onChange={(e) => setSearchOptions({ caseSensitive: e.target.checked })}
                  >
                    {t("search.caseSensitive")}
                  </Checkbox>
                </div>
                <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                  <Text type="secondary">{t("search.limit")}:</Text>
                  <Select
                    value={searchOptions.limit}
                    onChange={(value) => setSearchOptions({ limit: value })}
                    style={{ width: 100 }}
                    options={[
                      { value: 20, label: "20" },
                      { value: 50, label: "50" },
                      { value: 100, label: "100" },
                    ]}
                  />
                </div>
              </Space>
            </Card>
          )}

          {recentSearches.length > 0 && !query && results.length === 0 && (
            <div>
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  marginBottom: 8,
                }}
              >
                <Space>
                  <History size={14} />
                  <Text type="secondary">{t("search.recentSearches")}</Text>
                </Space>
                <Popconfirm
                  title={t("search.clearRecent")}
                  onConfirm={clearRecentSearches}
                >
                  <Button type="link" size="small" danger>
                    {t("common.clear")}
                  </Button>
                </Popconfirm>
              </div>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                {recentSearches.map((q, i) => (
                  <Tag
                    key={i}
                    style={{ cursor: "pointer", padding: "4px 12px" }}
                    onClick={() => {
                      setQuery(q);
                      search();
                    }}
                  >
                    <Clock size={12} style={{ marginRight: 4 }} />
                    {q}
                  </Tag>
                ))}
              </div>
            </div>
          )}

          {savedFilters.length > 0 && (
            <div>
              <Text type="secondary" style={{ display: "block", marginBottom: 8 }}>
                <Star size={14} style={{ marginRight: 4 }} />
                {t("search.savedFilters")}
              </Text>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                {savedFilters.map((filter) => (
                  <Tag
                    key={filter.name}
                    style={{ cursor: "pointer", padding: "4px 12px" }}
                    onClick={() => handleLoadFilter(filter)}
                    closable
                    onClose={(e) => {
                      e.preventDefault();
                      deleteFilter(filter.name);
                    }}
                  >
                    {filter.name}
                  </Tag>
                ))}
              </div>
            </div>
          )}

          {query && (
            <div style={{ textAlign: "right" }}>
              <Button
                type="text"
                size="small"
                icon={<Star size={14} />}
                onClick={() => setSaveModalVisible(true)}
              >
                {t("search.saveFilter")}
              </Button>
            </div>
          )}

          {error && (
            <Alert
              type="error"
              message={error}
              style={{ marginTop: 8 }}
            />
          )}

          {results.length > 0
            ? (
              <List
                size="small"
                dataSource={results}
                renderItem={renderResultItem}
                style={{
                  maxHeight: 400,
                  overflow: "auto",
                  border: "1px solid var(--border, #45475a)",
                  borderRadius: 8,
                }}
                pagination={{
                  pageSize: 20,
                  size: "small",
                }}
              />
            )
            : query && !isSearching
            ? (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description={t("search.noResults")}
                style={{ marginTop: 40 }}
              />
            )
            : null}
        </Space>
      </div>

      <Modal
        title={t("search.saveFilterTitle")}
        open={saveModalVisible}
        onCancel={() => setSaveModalVisible(false)}
        onOk={handleSaveFilter}
        okText={t("common.save")}
        cancelText={t("common.cancel")}
      >
        <Input
          placeholder={t("search.filterName")}
          value={filterName}
          onChange={(e) => setFilterName(e.target.value)}
          style={{ marginTop: 8 }}
        />
      </Modal>
    </Modal>
  );
}

function Alert({
  type,
  message,
  style,
}: {
  type: "error" | "info" | "success" | "warning";
  message: string;
  style?: React.CSSProperties;
}) {
  const colors = {
    error: { bg: "#f38ba820", border: "#f38ba8" },
    info: { bg: "#89b4fa20", border: "#89b4fa" },
    success: { bg: "#a6e3a120", border: "#a6e3a1" },
    warning: { bg: "#f9e2af20", border: "#f9e2af" },
  };

  return (
    <div
      style={{
        padding: "8px 12px",
        background: colors[type].bg,
        borderLeft: `3px solid ${colors[type].border}`,
        borderRadius: 4,
        ...style,
      }}
    >
      <span style={{ color: colors[type].border }}>{message}</span>
    </div>
  );
}
