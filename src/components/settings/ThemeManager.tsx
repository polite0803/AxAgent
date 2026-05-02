import { Theme, ThemeColors, useThemeStore } from "@/stores/feature/themeStore";
import { Button, Card, Form, Input, List, message, Modal, Popconfirm, Space, Typography } from "antd";
import { Check, Copy, Delete, RefreshCw, Upload } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Title, Paragraph, Text } = Typography;

interface ThemePreviewProps {
  colors: ThemeColors;
  name: string;
}

function ThemePreview({ colors, name }: ThemePreviewProps) {
  return (
    <div
      style={{
        background: colors.background,
        border: `1px solid ${colors.brightBlack}`,
        borderRadius: 8,
        padding: 16,
        minWidth: 200,
      }}
    >
      <Text style={{ color: colors.foreground, fontSize: 12 }}>
        {name}
      </Text>
      <div style={{ marginTop: 12 }}>
        <div style={{ display: "flex", gap: 4, marginBottom: 4 }}>
          {["red", "green", "yellow", "blue"].map((c) => (
            <div
              key={c}
              style={{
                width: 16,
                height: 16,
                background: colors[c as keyof ThemeColors],
                borderRadius: 2,
              }}
            />
          ))}
        </div>
        <div style={{ color: colors.foreground, fontSize: 11 }}>
          <span style={{ color: colors.brightGreen }}>success</span>{" "}
          <span style={{ color: colors.brightYellow }}>warning</span>{" "}
          <span style={{ color: colors.brightRed }}>error</span>
        </div>
        <div
          style={{
            marginTop: 8,
            padding: 4,
            background: colors.brightBlack,
            borderRadius: 4,
          }}
        >
          <Text
            style={{
              color: colors.background,
              fontSize: 10,
              cursor: "block",
            }}
          >
            {"█"}
          </Text>
        </div>
      </div>
    </div>
  );
}

export default function ThemeManager() {
  const { t } = useTranslation();
  const {
    currentTheme,
    themes,
    customThemes,
    isLoading,
    setCurrentTheme,
    loadThemes,
    deleteCustomTheme,
  } = useThemeStore();

  const [_selectedTheme, setSelectedTheme] = useState<string | null>(null);
  const [_editModalVisible, _setEditModalVisible] = useState(false);
  const [importModalVisible, setImportModalVisible] = useState(false);
  const [_themeToEdit, _setThemeToEdit] = useState<Theme | null>(null);
  const [form] = Form.useForm();

  useEffect(() => {
    loadThemes();
  }, [loadThemes]);

  const handleThemeSelect = (themeName: string) => {
    setSelectedTheme(themeName);
    setCurrentTheme(themeName);
  };

  const handleExportTheme = (theme: Theme) => {
    const yaml = themeToYaml(theme);
    navigator.clipboard.writeText(yaml);
    message.success(t("settings.theme.exported"));
  };

  const themeToYaml = (theme: Theme): string => {
    return `metadata:
  name: ${theme.metadata.name}
  version: ${theme.metadata.version}
  author: ${theme.metadata.author || ""}
  description: ${theme.metadata.description || ""}
colors:
  background: "${theme.colors.background}"
  foreground: "${theme.colors.foreground}"
  cursor: "${theme.colors.cursor}"
  black: "${theme.colors.black}"
  red: "${theme.colors.red}"
  green: "${theme.colors.green}"
  yellow: "${theme.colors.yellow}"
  blue: "${theme.colors.blue}"
  magenta: "${theme.colors.magenta}"
  cyan: "${theme.colors.cyan}"
  white: "${theme.colors.white}"
  brightBlack: "${theme.colors.brightBlack}"
  brightRed: "${theme.colors.brightRed}"
  brightGreen: "${theme.colors.brightGreen}"
  brightYellow: "${theme.colors.brightYellow}"
  brightBlue: "${theme.colors.brightBlue}"
  brightMagenta: "${theme.colors.brightMagenta}"
  brightCyan: "${theme.colors.brightCyan}"
  brightWhite: "${theme.colors.brightWhite}"
`;
  };

  const handleImportTheme = (values: { yaml: string }) => {
    try {
      const theme = yamlToTheme(values.yaml);
      if (theme) {
        useThemeStore.getState().saveCustomTheme(theme);
        message.success(t("settings.theme.imported"));
        setImportModalVisible(false);
        form.resetFields();
      }
    } catch (e) {
      message.error(t("settings.theme.invalidYaml"));
    }
  };

  const yamlToTheme = (yaml: string): Theme | null => {
    try {
      const lines = yaml.split("\n");
      const result: Record<string, any> = { metadata: {}, colors: {} };

      for (const line of lines) {
        const [key, ...valueParts] = line.split(":");
        const value = valueParts.join(":").trim().replace(/"/g, "");

        if (key === "name") { result.metadata.name = value; }
        else if (key === "version") { result.metadata.version = value; }
        else if (key === "author") { result.metadata.author = value; }
        else if (key === "description") { result.metadata.description = value; }
        else if (key && value) {
          const cleanKey = key.replace(/-/g, "");
          if (result.colors && cleanKey in result.colors) {
            (result.colors as Record<string, string>)[cleanKey] = value;
          }
        }
      }

      if (!result.metadata.name || !result.colors.background) {
        return null;
      }

      return {
        metadata: result.metadata as Theme["metadata"],
        colors: result.colors as ThemeColors,
      };
    } catch {
      return null;
    }
  };

  const handleDeleteTheme = async (themeName: string) => {
    try {
      await deleteCustomTheme(themeName);
      message.success(t("settings.theme.deleted"));
    } catch (e) {
      message.error(String(e));
    }
  };

  const builtInThemes = themes.filter(
    (t) => !customThemes.some((ct) => ct.metadata.name === t.name),
  );

  const getThemeColors = (themeName: string): ThemeColors | null => {
    const builtInColors: Record<string, ThemeColors> = {
      default: {
        background: "#1e1e2e",
        foreground: "#cdd6f4",
        cursor: "#f5e0dc",
        black: "#45475a",
        red: "#f38ba8",
        green: "#a6e3a1",
        yellow: "#f9e2af",
        blue: "#89b4fa",
        magenta: "#f5c2e7",
        cyan: "#94e2d5",
        white: "#bac2de",
        brightBlack: "#585b70",
        brightRed: "#f38ba8",
        brightGreen: "#a6e3a1",
        brightYellow: "#f9e2af",
        brightBlue: "#89b4fa",
        brightMagenta: "#f5c2e7",
        brightCyan: "#94e2d5",
        brightWhite: "#a6adc8",
      },
      monokai: {
        background: "#272822",
        foreground: "#f8f8f2",
        cursor: "#f8f8f0",
        black: "#272822",
        red: "#f92672",
        green: "#a6e22e",
        yellow: "#f4bf75",
        blue: "#66d9ef",
        magenta: "#ae81ff",
        cyan: "#a1efe4",
        white: "#f8f8f2",
        brightBlack: "#75715E",
        brightRed: "#f92672",
        brightGreen: "#a6e22e",
        brightYellow: "#f4bf75",
        brightBlue: "#66d9ef",
        brightMagenta: "#ae81ff",
        brightCyan: "#a1efe4",
        brightWhite: "#f9f8f5",
      },
      gruvbox: {
        background: "#282828",
        foreground: "#ebdbb2",
        cursor: "#ebdbb2",
        black: "#282828",
        red: "#cc241d",
        green: "#98971a",
        yellow: "#d79921",
        blue: "#458588",
        magenta: "#b16286",
        cyan: "#689d6a",
        white: "#a89984",
        brightBlack: "#928374",
        brightRed: "#fb4934",
        brightGreen: "#b8bb26",
        brightYellow: "#fabd2f",
        brightBlue: "#83a598",
        brightMagenta: "#d3869b",
        brightCyan: "#8ec07c",
        brightWhite: "#ebdbb2",
      },
      "catppuccin-mocha": {
        background: "#1e1e2e",
        foreground: "#cdd6f4",
        cursor: "#f5e0dc",
        black: "#45475a",
        red: "#f38ba8",
        green: "#a6e3a1",
        yellow: "#f9e2af",
        blue: "#89b4fa",
        magenta: "#f5c2e7",
        cyan: "#94e2d5",
        white: "#bac2de",
        brightBlack: "#585b70",
        brightRed: "#f38ba8",
        brightGreen: "#a6e3a1",
        brightYellow: "#f9e2af",
        brightBlue: "#89b4fa",
        brightMagenta: "#f5c2e7",
        brightCyan: "#94e2d5",
        brightWhite: "#a6adc8",
      },
    };

    if (builtInColors[themeName]) {
      return builtInColors[themeName];
    }

    const customTheme = customThemes.find(
      (t) => t.metadata.name === themeName,
    );
    return customTheme?.colors || null;
  };

  return (
    <div className="max-w-5xl">
      <div className="flex items-center justify-between mb-6">
        <div>
          <Title level={4}>{t("settings.theme.title")}</Title>
          <Paragraph type="secondary">
            {t("settings.theme.description")}
          </Paragraph>
        </div>
        <Space>
          <Button
            icon={<Upload size={16} />}
            onClick={() => setImportModalVisible(true)}
          >
            {t("settings.theme.import")}
          </Button>
          <Button
            icon={<RefreshCw size={16} className={isLoading ? "animate-spin" : ""} />}
            onClick={() => loadThemes()}
          >
            {t("settings.theme.refresh")}
          </Button>
        </Space>
      </div>

      <Title level={5}>{t("settings.theme.builtInThemes")}</Title>
      <div className="grid grid-cols-4 gap-4 mb-8">
        {builtInThemes.map((theme) => {
          const colors = getThemeColors(theme.name);
          if (!colors) { return null; }
          return (
            <Card
              key={theme.name}
              hoverable
              onClick={() => handleThemeSelect(theme.name)}
              style={{
                border: currentTheme === theme.name
                  ? "2px solid #89b4fa"
                  : "1px solid #45475a",
                background: colors.background,
              }}
            >
              <ThemePreview colors={colors} name={theme.name} />
              <div
                style={{
                  marginTop: 8,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                }}
              >
                <Text style={{ color: colors.foreground, fontSize: 13 }}>
                  {theme.name}
                </Text>
                {currentTheme === theme.name && <Check size={16} color="#a6e3a1" />}
              </div>
              {theme.description && (
                <Text
                  style={{ color: colors.white, fontSize: 11 }}
                  className="block mt-1"
                >
                  {theme.description}
                </Text>
              )}
            </Card>
          );
        })}
      </div>

      {customThemes.length > 0 && (
        <>
          <Title level={5}>{t("settings.theme.customThemes")}</Title>
          <List
            grid={{ gutter: 16, xs: 1, sm: 2, md: 3, lg: 4 }}
            dataSource={customThemes}
            renderItem={(theme) => {
              const colors = theme.colors;
              return (
                <List.Item>
                  <Card
                    hoverable
                    onClick={() => handleThemeSelect(theme.metadata.name)}
                    style={{
                      border: currentTheme === theme.metadata.name
                        ? "2px solid #89b4fa"
                        : "1px solid #45475a",
                      background: colors.background,
                    }}
                  >
                    <ThemePreview
                      colors={colors}
                      name={theme.metadata.name}
                    />
                    <div
                      style={{
                        marginTop: 8,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                      }}
                    >
                      <Text style={{ color: colors.foreground, fontSize: 13 }}>
                        {theme.metadata.name}
                      </Text>
                      <Space size={4}>
                        <Button
                          type="text"
                          size="small"
                          icon={<Copy size={12} />}
                          onClick={(e) => {
                            e.stopPropagation();
                            handleExportTheme(theme);
                          }}
                          style={{ color: colors.foreground }}
                        />
                        <Popconfirm
                          title={t("settings.theme.deleteConfirm")}
                          onConfirm={(e) => {
                            e?.stopPropagation();
                            handleDeleteTheme(theme.metadata.name);
                          }}
                          onCancel={(e) => e?.stopPropagation()}
                        >
                          <Button
                            type="text"
                            size="small"
                            danger
                            icon={<Delete size={12} />}
                            onClick={(e) => e.stopPropagation()}
                          />
                        </Popconfirm>
                      </Space>
                    </div>
                  </Card>
                </List.Item>
              );
            }}
          />
        </>
      )}

      <Modal
        title={t("settings.theme.importTitle")}
        open={importModalVisible}
        onCancel={() => {
          setImportModalVisible(false);
          form.resetFields();
        }}
        footer={null}
      >
        <Form form={form} onFinish={handleImportTheme} layout="vertical">
          <Form.Item
            name="yaml"
            label={t("settings.theme.yamlContent")}
            rules={[
              { required: true, message: t("settings.theme.yamlRequired") },
            ]}
          >
            <Input.TextArea
              rows={15}
              placeholder={`metadata:
  name: my-custom-theme
  version: 1.0.0
  author: Your Name
  description: My custom theme
colors:
  background: "#1e1e2e"
  foreground: "#cdd6f4"
  ...`}
            />
          </Form.Item>
          <Form.Item className="mb-0">
            <div className="flex justify-end gap-2">
              <Button onClick={() => setImportModalVisible(false)}>
                {t("common.cancel")}
              </Button>
              <Button type="primary" htmlType="submit">
                {t("settings.theme.import")}
              </Button>
            </div>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
