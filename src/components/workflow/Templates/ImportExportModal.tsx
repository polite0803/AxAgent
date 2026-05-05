import { invoke } from "@/lib/invoke";
import { Alert, Button, Divider, Input, message, Modal, Tabs, Upload } from "antd";
import type { UploadProps } from "antd";
import { Check, Copy, Download, FolderOpen, Upload as UploadIcon } from "lucide-react";
import React, { useState } from "react";

function BatchImportN8n() {
  const [importing, setImporting] = useState(false);
  const [result, setResult] = useState<
    {
      imported: number;
      skipped: number;
      errors: string[];
    } | null
  >(null);

  const handleBatchImport = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (!selected) { return; }

      setImporting(true);
      setResult(null);
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
        errors: res.error_details.slice(0, 5),
      });
      if (res.imported > 0) {
        message.success(`成功导入 ${res.imported} 个工作流`);
      }
    } catch (e) {
      message.error(String(e));
    } finally {
      setImporting(false);
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
        选择 n8n 工作流目录导入
      </Button>
      {result && (
        <Alert
          style={{ marginTop: 8 }}
          type={result.errors.length > 0 ? "warning" : "success"}
          message={
            <span style={{ fontSize: 12 }}>
              导入 {result.imported} 个 · 跳过 {result.skipped} 个
              {result.errors.length > 0 && ` · 错误 ${result.errors.length} 个`}
            </span>
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
  onImport: (jsonData: string) => Promise<string | null>;
}

export const ImportExportModal: React.FC<ImportExportModalProps> = ({
  open,
  onClose,
  onExport,
  onImport,
}) => {
  const [activeTab, setActiveTab] = useState("export");
  const [exportId, setExportId] = useState("");
  const [exportResult, setExportResult] = useState<string | null>(null);
  const [importData, setImportData] = useState("");
  const [isExporting, setIsExporting] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [copied, setCopied] = useState(false);

  const handleExport = async () => {
    if (!exportId.trim()) {
      message.warning("请输入模板 ID");
      return;
    }
    setIsExporting(true);
    setExportResult(null);
    try {
      const result = await onExport(exportId.trim());
      if (result) {
        setExportResult(result);
        message.success("模板导出成功");
      } else {
        message.error("模板不存在或导出失败");
      }
    } catch (error) {
      message.error("导出失败");
    } finally {
      setIsExporting(false);
    }
  };

  const handleImport = async () => {
    if (!importData.trim()) {
      message.warning("请输入或粘贴 JSON 数据");
      return;
    }
    try {
      JSON.parse(importData);
    } catch {
      message.error("JSON 格式无效");
      return;
    }
    setIsImporting(true);
    try {
      const newId = await onImport(importData.trim());
      if (newId) {
        message.success("模板导入成功");
        setImportData("");
        onClose();
      } else {
        message.error("导入失败");
      }
    } catch (error) {
      message.error("导入失败");
    } finally {
      setIsImporting(false);
    }
  };

  const handleCopy = () => {
    if (exportResult) {
      navigator.clipboard.writeText(exportResult);
      setCopied(true);
      message.success("已复制到剪贴板");
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const handleClear = () => {
    setExportId("");
    setExportResult(null);
    setImportData("");
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
      message.error("文件读取失败");
      onError?.(new Error("File read error"));
    };
    reader.readAsText(file as Blob);
  };

  const tabItems = [
    {
      key: "export",
      label: "导出",
      children: (
        <div style={{ padding: "16px 0" }}>
          <div style={{ marginBottom: 16 }}>
            <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 8 }}>
              模板 ID
            </label>
            <Input
              placeholder="输入要导出的模板 ID"
              value={exportId}
              onChange={(e) => setExportId(e.target.value)}
              size="large"
            />
          </div>

          <Button
            type="primary"
            icon={<Download size={14} />}
            onClick={handleExport}
            loading={isExporting}
            style={{ width: "100%", marginBottom: 16 }}
          >
            导出模板
          </Button>

          {exportResult && (
            <>
              <Divider style={{ margin: "16px 0" }} />
              <div>
                <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 8 }}>
                  导出结果 (JSON)
                </label>
                <div style={{ position: "relative" }}>
                  <Input.TextArea
                    value={exportResult}
                    readOnly
                    rows={10}
                    style={{
                      fontFamily: "Monaco, Consolas, monospace",
                      fontSize: 11,
                      background: "#1a1a1a",
                    }}
                  />
                  <Button
                    type="text"
                    icon={copied ? <Check size={14} /> : <Copy size={14} />}
                    onClick={handleCopy}
                    style={{ position: "absolute", top: 8, right: 8 }}
                  >
                    {copied ? "已复制" : "复制"}
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
      label: "导入",
      children: (
        <div style={{ padding: "16px 0" }}>
          <div style={{ marginBottom: 16 }}>
            <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 8 }}>
              上传 JSON 文件
            </label>
            <Upload.Dragger
              accept=".json"
              customRequest={handleFileUpload}
              showUploadList={false}
              style={{ marginBottom: 16 }}
            >
              <p style={{ color: "#666", margin: "16px 0" }}>
                <UploadIcon size={24} color="#666" style={{ marginBottom: 8 }} />
                <br />
                点击或拖拽文件到此处上传
              </p>
            </Upload.Dragger>
          </div>

          <Divider>或</Divider>

          <div style={{ marginBottom: 16 }}>
            <label style={{ display: "block", color: "#999", fontSize: 12, marginBottom: 8 }}>
              粘贴 JSON 数据
            </label>
            <Input.TextArea
              placeholder="粘贴模板 JSON 数据..."
              value={importData}
              onChange={(e) => setImportData(e.target.value)}
              rows={8}
              style={{
                fontFamily: "Monaco, Consolas, monospace",
                fontSize: 11,
                background: "#1a1a1a",
              }}
            />
          </div>

          <Button
            type="primary"
            icon={<UploadIcon size={14} />}
            onClick={handleImport}
            loading={isImporting}
            style={{ width: "100%" }}
          >
            导入模板
          </Button>

          <p style={{ color: "#666", fontSize: 11, marginTop: 12 }}>
            导入将创建一个新的模板副本。导入的模板默认是自定义模板（非预设）。
          </p>

          <Divider style={{ margin: "12px 0", fontSize: 11 }}>n8n 批量导入</Divider>

          <BatchImportN8n />
        </div>
      ),
    },
  ];

  return (
    <Modal
      title="导入/导出模板"
      open={open}
      onCancel={handleClose}
      footer={null}
      width={600}
      destroyOnClose
    >
      <Tabs
        activeKey={activeTab}
        onChange={setActiveTab}
        items={tabItems}
      />
    </Modal>
  );
};
