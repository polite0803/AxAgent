// SPDX-License-Identifier: AGPL-3.0-only

import { showBackendError } from "@/lib/errorI18n";
import { invoke } from "@/lib/invoke";
import { message } from "@/lib/toast";
import { Alert, App, Button, Input, Modal, Tabs, theme, Upload } from "antd";
import type { UploadProps } from "antd";
import { FolderOpen, Upload as UploadIcon } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

interface WorkflowImportModalProps {
  open: boolean;
  onClose: () => void;
  /** 导入完成后刷新模板列表 */
  onImported: () => void;
}

interface BatchResult {
  imported: number;
  skipped: number;
  errorCount: number;
  errors: string[];
  importedNames: string[];
}

/** 工作流导入（我的工作流页）：n8n 目录 / 工作流目录 / JSON 粘贴上传 / 内置预设 */
export function WorkflowImportModal({
  open,
  onClose,
  onImported,
}: WorkflowImportModalProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message: appMessage } = App.useApp();
  const [importing, setImporting] = useState(false);
  const [jsonText, setJsonText] = useState("");
  const [result, setResult] = useState<BatchResult | null>(null);
  const [showAllErrors, setShowAllErrors] = useState(false);

  const resetState = () => {
    setImporting(false);
    setJsonText("");
    setResult(null);
    setShowAllErrors(false);
  };

  /** 选目录批量导入（n8n 或通用工作流目录） */
  const handleDirImport = async (command: "import_n8n_directory" | "import_workflow_directory") => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (!selected) {
        return;
      }
      setImporting(true);
      setResult(null);
      setShowAllErrors(false);
      const res = await invoke<{
        imported: number;
        imported_names: string[];
        skipped: number;
        skipped_reasons: string[];
        errors: number;
        error_details: string[];
      }>(command, { path: selected });
      setResult({
        imported: res.imported,
        skipped: res.skipped,
        errors: res.error_details,
        errorCount: res.errors,
        importedNames: res.imported_names,
      });
      if (res.imported > 0) {
        message.success(
          t("workflow.importExport.importSuccess", { count: res.imported }),
        );
        onImported();
      }
    } catch (e) {
      showBackendError(message, e);
    } finally {
      setImporting(false);
    }
  };

  /** JSON 粘贴 / 文件读取 → 单模板导入（自动识别 n8n / AxAgent 格式） */
  const handleJsonImport = async () => {
    if (!jsonText.trim()) {
      appMessage.warning(t("workflow.import.jsonRequired"));
      return;
    }
    setImporting(true);
    try {
      const res = await invoke<{ id: string; warnings: string[]; errors: string[] }>(
        "import_workflow_template",
        { jsonData: jsonText },
      );
      if (res.errors.length > 0) {
        appMessage.error(res.errors.join("; "));
      } else {
        message.success(t("workflow.import.jsonSuccess"));
        setJsonText("");
        onImported();
      }
    } catch (e) {
      showBackendError(message, e);
    } finally {
      setImporting(false);
    }
  };

  /** 读取 JSON 文件内容填入输入框 */
  const handleFilePick: UploadProps["beforeUpload"] = (file) => {
    const reader = new FileReader();
    reader.onload = () => {
      setJsonText(String(reader.result ?? ""));
    };
    reader.readAsText(file);
    return false;
  };

  const handleImportPresets = async () => {
    setImporting(true);
    try {
      const count = await invoke<number>("seed_preset_templates");
      message.success(t("workflow.templateList.presetsImported", { count }));
      onImported();
    } catch (e) {
      showBackendError(message, e);
    } finally {
      setImporting(false);
    }
  };

  const renderResult = () => {
    if (!result) { return null; }
    const { imported, skipped, errorCount, errors, importedNames } = result;
    return (
      <Alert
        style={{ marginTop: 8 }}
        type={errorCount > 0 ? "warning" : "success"}
        showIcon
        message={t("workflow.importExport.importSummary", {
          imported,
          skipped,
          errors: errorCount,
        })}
        description={
          <div style={{ fontSize: 12 }}>
            {importedNames.length > 0 && (
              <div style={{ marginBottom: 4 }}>
                {t("workflow.importExport.importedNames")}: {importedNames.join(", ")}
              </div>
            )}
            {errorCount > 0 && (
              <>
                <Button
                  type="link"
                  size="small"
                  style={{ padding: 0, height: "auto" }}
                  onClick={() => setShowAllErrors((v) => !v)}
                >
                  {showAllErrors
                    ? t("workflow.importExport.hideErrors")
                    : t("workflow.importExport.showErrors")}
                </Button>
                {showAllErrors && (
                  <ul style={{ marginTop: 4, paddingLeft: 16, maxHeight: 120, overflowY: "auto" }}>
                    {errors.map((e, i) => <li key={i}>{e}</li>)}
                  </ul>
                )}
              </>
            )}
          </div>
        }
      />
    );
  };

  return (
    <Modal
      title={t("workflow.import.title")}
      open={open}
      onCancel={() => {
        resetState();
        onClose();
      }}
      footer={null}
      width={560}
      destroyOnHidden
    >
      <Tabs
        size="small"
        items={[
          {
            key: "n8n",
            label: t("workflow.import.tabN8n"),
            children: (
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                <Button
                  icon={<FolderOpen size={14} />}
                  onClick={() => handleDirImport("import_n8n_directory")}
                  loading={importing}
                  style={{ width: "100%" }}
                >
                  {t("workflow.importExport.selectN8nDir")}
                </Button>
                <span style={{ fontSize: 12, color: token.colorTextTertiary }}>
                  {t("workflow.import.n8nHint")}
                </span>
                {renderResult()}
              </div>
            ),
          },
          {
            key: "dir",
            label: t("workflow.import.tabDir"),
            children: (
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                <Button
                  icon={<FolderOpen size={14} />}
                  onClick={() => handleDirImport("import_workflow_directory")}
                  loading={importing}
                  style={{ width: "100%" }}
                >
                  {t("workflow.import.selectWorkflowDir")}
                </Button>
                <span style={{ fontSize: 12, color: token.colorTextTertiary }}>
                  {t("workflow.import.dirHint")}
                </span>
                {renderResult()}
              </div>
            ),
          },
          {
            key: "json",
            label: t("workflow.import.tabJson"),
            children: (
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                <Upload beforeUpload={handleFilePick} showUploadList={false} accept=".json">
                  <Button icon={<UploadIcon size={14} />} size="small">
                    {t("workflow.import.pickFile")}
                  </Button>
                </Upload>
                <Input.TextArea
                  rows={6}
                  value={jsonText}
                  onChange={(e) => setJsonText(e.target.value)}
                  placeholder={t("workflow.import.jsonPlaceholder")}
                  style={{ fontFamily: "var(--font-mono, monospace)", fontSize: 12 }}
                />
                <Button
                  type="primary"
                  size="small"
                  onClick={handleJsonImport}
                  loading={importing}
                  disabled={!jsonText.trim()}
                >
                  {t("workflow.import.jsonImportBtn")}
                </Button>
                <span style={{ fontSize: 12, color: token.colorTextTertiary }}>
                  {t("workflow.import.jsonHint")}
                </span>
              </div>
            ),
          },
          {
            key: "presets",
            label: t("workflow.import.tabPresets"),
            children: (
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                <Button onClick={handleImportPresets} loading={importing} size="small">
                  {t("workflow.import.presetsBtn")}
                </Button>
                <span style={{ fontSize: 12, color: token.colorTextTertiary }}>
                  {t("workflow.import.presetsHint")}
                </span>
              </div>
            ),
          },
        ]}
      />
    </Modal>
  );
}
