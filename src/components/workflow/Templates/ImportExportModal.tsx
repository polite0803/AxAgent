import { invoke } from "@/lib/invoke";
import { Alert, Button, Descriptions, Divider, Input, message, Modal, Select, Tabs, Typography, Upload } from "antd";
import type { UploadProps } from "antd";
import { Check, Copy, Download, FolderOpen, Upload as UploadIcon } from "lucide-react";
import React, { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { WorkflowTemplateResponse } from "../types";

interface N8nConnectionGroup {
  node: string;
  index: number;
}

interface N8nConnection {
  main?: N8nConnectionGroup[][];
}

function getImportPreview(
  jsonStr: string,
): {
  name: string;
  nodeCount: number;
  edgeCount: number;
  format: string;
} | null {
  try {
    const json = JSON.parse(jsonStr);
    const isN8n = json.nodes?.some?.((n: any) => n.type?.startsWith?.("n8n-nodes-base."));
    const name = json.name || "Untitled";
    const nodeCount = json.nodes?.length || 0;
    let edgeCount = 0;
    if (isN8n) {
      const connections: Record<string, N8nConnection> = json.connections || {};
      for (const conn of Object.values(connections)) {
        const main = conn?.main;
        if (Array.isArray(main)) {
          for (const group of main) {
            if (Array.isArray(group)) {
              edgeCount += group.length;
            }
          }
        }
      }
    } else {
      edgeCount = json.edges?.length || 0;
    }
    return { name, nodeCount, edgeCount, format: isN8n ? "n8n" : "AxAgent" };
  } catch {
    return null;
  }
}

function BatchImportN8n({
  onImportComplete,
}: {
  onImportComplete?: () => void;
}) {
  const { t } = useTranslation();
  const [importing, setImporting] = useState(false);
  const [progressText, setProgressText] = useState<string>("");
  const [result, setResult] = useState<
    {
      imported: number;
      skipped: number;
      errors: string[];
      errorCount: number;
      importedNames: string[];
    } | null
  >(null);
  const [showAllErrors, setShowAllErrors] = useState(false);

  const handleBatchImport = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (!selected) {
        return;
      }

      setImporting(true);
      setResult(null);
      setShowAllErrors(false);
      setProgressText(t("workflow.importExport.scanningFolder"));
      const res = await invoke<{
        imported: number;
        imported_names: string[];
        skipped: number;
        skipped_reasons: string[];
        errors: number;
        error_details: string[];
      }>("import_n8n_directory", { path: selected });
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
        onImportComplete?.();
      }
    } catch (e) {
      message.error(String(e));
    } finally {
      setImporting(false);
      setProgressText("");
    }
  };

  return (
    <div>
      <Button
        icon={<FolderOpen size={14} />}
        onClick={handleBatchImport}
        loading={importing}
        style={{ width: "100%" }}
      >
        {t("workflow.importExport.selectN8nDir")}
      </Button>
      {importing && progressText && (
        <div style={{ marginTop: 8, color: "#999", fontSize: 12 }}>
          {progressText}
        </div>
      )}
      {result && (
        <Alert
          style={{ marginTop: 8 }}
          type={result.errors.length > 0 ? "warning" : "success"}
          message={
            <div style={{ fontSize: 12 }}>
              <div>
                {t("workflow.importExport.n8nResult", {
                  imported: result.imported,
                  skipped: result.skipped,
                })}
                {result.errorCount > 0
                  ? ` · ${t("workflow.importExport.errorCount", { count: result.errorCount })}`
                  : ""}
              </div>
              {result.errors.length > 0 && (
                <div style={{ marginTop: 6 }}>
                  {/* error strings appended sequentially, safe to use index as key */}
                  {(showAllErrors
                    ? result.errors
                    : result.errors.slice(0, 5)).map((e, i) => (
                      <div
                        key={`${e.slice(0, 20)}-${i}`}
                        style={{
                          color: "#595959",
                          fontSize: 12,
                          marginBottom: 2,
                        }}
                      >
                        {e}
                      </div>
                    ))}
                  {result.errors.length > 5 && !showAllErrors && (
                    <Button
                      type="link"
                      size="small"
                      style={{ padding: 0, fontSize: 12 }}
                      onClick={() => setShowAllErrors(true)}
                    >
                      {t("workflow.importExport.viewAllErrors", {
                        count: result.errors.length,
                      })}
                    </Button>
                  )}
                </div>
              )}
            </div>
          }
        />
      )}
    </div>
  );
}

function BatchImportFolder({
  onImportComplete,
}: {
  onImportComplete?: () => void;
}) {
  const { t } = useTranslation();
  const [importing, setImporting] = useState(false);
  const [progressText, setProgressText] = useState<string>("");
  const [result, setResult] = useState<
    {
      imported: number;
      errors: string[];
    } | null
  >(null);

  const handleBatchImport = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (!selected) {
        return;
      }

      setImporting(true);
      setResult(null);
      setProgressText(t("workflow.importExport.scanningFolder"));

      const res = await invoke<{
        imported: number;
        errors: number;
        error_details: string[];
      }>("import_workflow_directory", { path: selected as string });

      setResult({
        imported: res.imported,
        errors: res.error_details,
      });
      if (res.imported > 0) {
        message.success(
          t("workflow.importExport.batchImportSuccess", {
            count: res.imported,
          }),
        );
        onImportComplete?.();
      } else if (res.error_details.length === 0) {
        message.warning(t("workflow.importExport.noJsonFound"));
      }
    } catch (e) {
      message.error(String(e));
    } finally {
      setImporting(false);
      setProgressText("");
    }
  };

  return (
    <div>
      <Button
        icon={<FolderOpen size={14} />}
        onClick={handleBatchImport}
        loading={importing}
        style={{ width: "100%" }}
      >
        {t("workflow.importExport.selectFolder")}
      </Button>
      {importing && progressText && (
        <div style={{ marginTop: 8, color: "#999", fontSize: 12 }}>
          {progressText}
        </div>
      )}
      {result && (
        <Alert
          style={{ marginTop: 8 }}
          type={result.errors.length > 0 ? "warning" : "success"}
          message={
            <div style={{ fontSize: 12 }}>
              <div>
                {t("workflow.importExport.batchResult", {
                  count: result.imported,
                })}
                {result.errors.length > 0
                  ? ` · ${t("workflow.importExport.errorCount", { count: result.errors.length })}`
                  : ""}
              </div>
              {result.errors.length > 0 && (
                <div style={{ marginTop: 6 }}>
                  {/* error strings appended sequentially, safe to use index as key */}
                  {result.errors.slice(0, 10).map((e, i) => (
                    <div
                      key={`${e.slice(0, 20)}-${i}`}
                      style={{
                        color: "#595959",
                        fontSize: 12,
                        marginBottom: 2,
                      }}
                    >
                      {e}
                    </div>
                  ))}
                  {result.errors.length > 10 && (
                    <div style={{ color: "#999", fontSize: 12 }}>
                      {t("workflow.importExport.moreErrors", {
                        count: result.errors.length - 10,
                      })}
                    </div>
                  )}
                </div>
              )}
            </div>
          }
        />
      )}
    </div>
  );
}

interface ImportExportModalProps {
  open: boolean;
  onClose: () => void;
  onExport: (id: string) => Promise<string | null>;
  onImport: (
    jsonData: string,
  ) => Promise<{ id: string; warnings: string[]; errors: string[] } | null>;
  onImportComplete?: () => void;
  onImportedTemplate?: (id: string) => void;
  templates: WorkflowTemplateResponse[];
}

export const ImportExportModal: React.FC<ImportExportModalProps> = ({
  open,
  onClose,
  onExport,
  onImport,
  onImportComplete,
  onImportedTemplate,
  templates,
}) => {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState("export");
  const [exportId, setExportId] = useState("");
  const [exportResult, setExportResult] = useState<string | null>(null);
  const [importData, setImportData] = useState("");
  const [isExporting, setIsExporting] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [importWarnings, setImportWarnings] = useState<string[]>([]);
  const [copied, setCopied] = useState(false);
  const copiedTimerRef = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => {
    return () => clearTimeout(copiedTimerRef.current);
  }, []);

  const preview = useMemo(() => {
    if (!importData.trim()) {
      return null;
    }
    return getImportPreview(importData.trim());
  }, [importData]);

  const handleExport = async () => {
    if (!exportId) {
      message.warning(t("workflow.importExport.pleaseEnterId"));
      return;
    }
    setIsExporting(true);
    setExportResult(null);
    try {
      const result = await onExport(exportId);
      if (result) {
        setExportResult(result);
        message.success(t("workflow.importExport.exportSuccess"));
      } else {
        message.error(t("workflow.importExport.exportNotFound"));
      }
    } catch (error) {
      message.error(t("workflow.importExport.exportFailed"));
    } finally {
      setIsExporting(false);
    }
  };

  const handleImport = async () => {
    if (!importData.trim()) {
      message.warning(t("workflow.importExport.pleaseEnterJson"));
      return;
    }
    try {
      JSON.parse(importData);
    } catch {
      message.error(t("workflow.importExport.invalidJson"));
      return;
    }
    setIsImporting(true);
    try {
      const result = await onImport(importData.trim());
      if (result) {
        message.success(t("workflow.importExport.templateImportSuccess"));
        setImportData("");
        setImportWarnings(result.warnings || []);
        onImportComplete?.();
        if (result.id) {
          onImportedTemplate?.(result.id);
        }
        if (!result.warnings?.length && !result.errors?.length) {
          onClose();
        }
      } else {
        message.error(t("workflow.importExport.importFailed"));
      }
    } catch (error) {
      message.error(
        t("workflow.importExport.importFailedWithError", {
          error: String(error),
        }),
      );
    } finally {
      setIsImporting(false);
    }
  };

  const handleCopy = () => {
    if (exportResult) {
      navigator.clipboard.writeText(exportResult);
      setCopied(true);
      message.success(t("workflow.importExport.copiedToClipboard"));
      clearTimeout(copiedTimerRef.current);
      copiedTimerRef.current = setTimeout(() => setCopied(false), 2000);
    }
  };

  const handleClear = () => {
    setExportId("");
    setExportResult(null);
    setImportData("");
    setImportWarnings([]);
    setCopied(false);
  };

  const handleClose = () => {
    handleClear();
    onClose();
  };

  const handleFileUpload: UploadProps["customRequest"] = async (options) => {
    const { file, onSuccess, onError } = options;
    const reader = new FileReader();
    reader.onload = (e) => {
      const text = e.target?.result as string;
      setImportData(text);
      onSuccess?.(file);
    };
    reader.onerror = () => {
      message.error(t("workflow.importExport.fileReadFailed"));
      onError?.(new Error("File read error"));
    };
    reader.readAsText(file as Blob);
  };

  const tabItems = [
    {
      key: "export",
      label: t("workflow.importExport.export"),
      children: (
        <div style={{ padding: "16px 0" }}>
          <div style={{ marginBottom: 16 }}>
            <label
              style={{
                display: "block",
                color: "#999",
                fontSize: 12,
                marginBottom: 8,
              }}
            >
              {t("workflow.importExport.templateId")}
            </label>
            <Select
              showSearch
              placeholder={t("workflow.importExport.enterTemplateId")}
              value={exportId || undefined}
              onChange={(val) => setExportId(val)}
              size="large"
              style={{ width: "100%" }}
              optionFilterProp="label"
              options={templates.map((template) => ({
                value: template.id,
                label: template.name,
              }))}
              filterOption={(input, option) =>
                (option?.label as string)
                  ?.toLowerCase()
                  .includes(input.toLowerCase())}
            />
          </div>

          <Button
            type="primary"
            icon={<Download size={14} />}
            onClick={handleExport}
            loading={isExporting}
            style={{ width: "100%", marginBottom: 16 }}
          >
            {t("workflow.importExport.exportTemplate")}
          </Button>

          {exportResult && (
            <>
              <Divider style={{ margin: "16px 0" }} />
              <div>
                <label
                  style={{
                    display: "block",
                    color: "#999",
                    fontSize: 12,
                    marginBottom: 8,
                  }}
                >
                  {t("workflow.importExport.exportResultJson")}
                </label>
                <div style={{ position: "relative" }}>
                  <Input.TextArea
                    id="import-export-modal-input-textarea-127"
                    value={exportResult}
                    readOnly
                    rows={10}
                    style={{
                      fontFamily: "Monaco, Consolas, monospace",
                      fontSize: 12,
                      background: "#1a1a1a",
                    }}
                  />
                  <Button
                    type="text"
                    icon={copied ? <Check size={14} /> : <Copy size={14} />}
                    onClick={handleCopy}
                    style={{ position: "absolute", top: 8, right: 8 }}
                  >
                    {copied
                      ? t("workflow.importExport.copied")
                      : t("workflow.importExport.copy")}
                  </Button>
                </div>
              </div>
            </>
          )}
        </div>
      ),
    },
    {
      key: "import",
      label: t("workflow.importExport.import"),
      children: (
        <div style={{ padding: "16px 0" }}>
          <div style={{ marginBottom: 16 }}>
            <label
              style={{
                display: "block",
                color: "#999",
                fontSize: 12,
                marginBottom: 8,
              }}
            >
              {t("workflow.importExport.uploadJsonFile")}
            </label>
            <Upload.Dragger
              accept=".json"
              customRequest={handleFileUpload}
              showUploadList={false}
              style={{ marginBottom: 16 }}
            >
              <p style={{ color: "#666", margin: "16px 0" }}>
                <UploadIcon
                  size={24}
                  color="#666"
                  style={{ marginBottom: 8 }}
                />
                <br />
                {t("workflow.importExport.dragOrClickUpload")}
              </p>
            </Upload.Dragger>
          </div>

          <Divider>{t("workflow.importExport.or")}</Divider>

          <div style={{ marginBottom: 16 }}>
            <label
              style={{
                display: "block",
                color: "#999",
                fontSize: 12,
                marginBottom: 8,
              }}
            >
              {t("workflow.importExport.pasteJsonData")}
            </label>
            <Input.TextArea
              id="import-export-modal-input-textarea-128"
              placeholder={t("workflow.importExport.pasteJsonPlaceholder")}
              value={importData}
              onChange={(e) => setImportData(e.target.value)}
              rows={8}
              style={{
                fontFamily: "Monaco, Consolas, monospace",
                fontSize: 12,
                background: "#1a1a1a",
              }}
            />
          </div>

          {preview && (
            <div style={{ marginBottom: 16 }}>
              <Descriptions
                title={t("workflow.importExport.preview")}
                size="small"
                bordered
                column={2}
                style={{ fontSize: 12 }}
              >
                <Descriptions.Item
                  label={t("workflow.importExport.workflowName")}
                >
                  {preview.name}
                </Descriptions.Item>
                <Descriptions.Item label={t("workflow.importExport.format")}>
                  {preview.format === "n8n"
                    ? t("workflow.importExport.formatN8n")
                    : t("workflow.importExport.formatAxAgent")}
                </Descriptions.Item>
                <Descriptions.Item label={t("workflow.importExport.nodeCount")}>
                  {preview.nodeCount}
                </Descriptions.Item>
                <Descriptions.Item label={t("workflow.importExport.edgeCount")}>
                  {preview.edgeCount}
                </Descriptions.Item>
              </Descriptions>
            </div>
          )}

          <Button
            type="primary"
            icon={<UploadIcon size={14} />}
            onClick={handleImport}
            loading={isImporting}
            style={{ width: "100%" }}
          >
            {t("workflow.importExport.importTemplate")}
          </Button>

          {importWarnings.length > 0 && (
            <Alert
              style={{ marginTop: 12 }}
              type="warning"
              closable
              onClose={() => setImportWarnings([])}
              message={
                <div style={{ fontSize: 12 }}>
                  {/* warning strings appended sequentially, safe to use index as key */}
                  {importWarnings.map((w, i) => <div key={`${w.slice(0, 20)}-${i}`}>{w}</div>)}
                </div>
              }
            />
          )}

          <p style={{ color: "#666", fontSize: 12, marginTop: 12 }}>
            {t("workflow.importExport.importHint")}
          </p>

          <Divider style={{ margin: "12px 0", fontSize: 12 }}>
            {t("workflow.importExport.batchImport")}
          </Divider>
          <Typography.Text
            type="secondary"
            style={{ fontSize: 12, display: "block", marginBottom: 8 }}
          >
            {t("workflow.importExport.axagentFolderHint")}
          </Typography.Text>
          <BatchImportFolder onImportComplete={onImportComplete} />

          <Divider style={{ margin: "12px 0", fontSize: 12 }}>
            {t("workflow.importExport.n8nBatchImport")}
          </Divider>
          <Typography.Text
            type="secondary"
            style={{ fontSize: 12, display: "block", marginBottom: 8 }}
          >
            {t("workflow.importExport.n8nFolderHint")}
          </Typography.Text>
          <BatchImportN8n onImportComplete={onImportComplete} />
        </div>
      ),
    },
  ];

  return (
    <Modal
      title={t("workflow.importExport.title")}
      open={open}
      onCancel={handleClose}
      footer={null}
      width={600}
      destroyOnHidden
    >
      <Tabs activeKey={activeTab} onChange={setActiveTab} items={tabItems} />
    </Modal>
  );
};
