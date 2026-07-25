// SPDX-License-Identifier: AGPL-3.0-only
/**
 * G6 截图持仓诊断（Screenshot Portfolio Diagnosis）前端类型定义
 *
 * 与后端 `axagent_entities::screenshot_diagnoses::Model` /
 * `axagent_stock_analysis::screenshot_diagnosis::{ScreenshotPosition, RiskDiagnosis, ...}` 对齐。
 * 后端使用 #[serde(rename_all = "camelCase")]，前端类型按 camelCase 命名。
 */

/** 截图识别出的单条持仓 */
export interface ScreenshotPosition {
  /** 股票代码（A 股 6 位 / 港股 5 位+后缀 / 美股字母） */
  code: string;
  /** 股票名称 */
  name: string;
  /** 持仓数量（股） */
  qty: number;
  /** 成本价 */
  costPrice: number;
  /** 当前市值（元） */
  marketValue: number;
  /** 权重（百分比，0-100） */
  weight: number;
}

/** 集中度风险 */
export interface ConcentrationRisk {
  /** top 1 持仓代码 */
  top1Code: string;
  /** top 1 权重（百分比） */
  top1Weight: number;
  /** high / medium / low */
  level: string;
  /** 自然语言描述 */
  narrative: string;
}

/** 重复持仓 */
export interface OverlapPosition {
  code: string;
  count: number;
  mergedWeight: number;
}

/** 弱势仓位 */
export interface WeakExposure {
  codes: string[];
  totalWeight: number;
  narrative: string;
}

/** 核心仓位集中度（top 3） */
export interface CoreConcentration {
  top3Codes: string[];
  top3Weight: number;
  /** high / medium / low */
  level: string;
  narrative: string;
}

/** 风险诊断 schema（7 项指标） */
export interface RiskDiagnosis {
  /** 集中度风险 */
  concentrationRisk: ConcentrationRisk;
  /** 重复持仓 */
  overlapPositions: OverlapPosition[];
  /** 防御仓位占比 */
  defenseRatio: number;
  /** 美股敞口占比 */
  usExposure: number;
  /** 弱势仓位 */
  weakExposure: WeakExposure;
  /** 同一标的重复出现 */
  repeatedPositions: string[];
  /** 核心仓位集中度 */
  coreConcentration: CoreConcentration;
}

/** 截图诊断记录（与后端 screenshot_diagnoses::Model 对齐） */
export interface ScreenshotDiagnosis {
  id: string;
  /** 截图 SHA256（去重用） */
  imageHash?: string | null;
  /** 截图本地存储路径 */
  imagePath?: string | null;
  /** 缩略图 base64 */
  imageThumbnailBase64?: string | null;
  /** 原图宽度 */
  imageWidth?: number | null;
  /** 原图高度 */
  imageHeight?: number | null;
  /** 截图来源 App（同花顺 / 东方财富 / 雪球 / 通达信 / 其他） */
  sourceApp?: string | null;
  /** OCR 提取的完整文本（debug 用） */
  ocrText?: string | null;
  /** 结构化持仓 JSON 数组字符串 */
  positionsJson: string;
  /** 总市值 */
  totalMarketValue: number;
  /** 风险诊断 JSON 字符串 */
  diagnosisJson: string;
  /** LLM 自然语言诊断说明 */
  narrative: string;
  /** 建议动作 JSON 数组字符串 */
  recommendedActions: string;
  /** 来源工作流执行 ID */
  sourceWorkflowExecutionId?: string | null;
  /** 使用的 LLM provider ID */
  providerId?: string | null;
  /** 使用的 LLM model ID */
  modelId?: string | null;
  /** active / archived / failed */
  status: string;
  /** 失败原因 */
  errorMessage?: string | null;
  /** 创建时间戳（ms） */
  createdAt: number;
  /** 更新时间戳（ms） */
  updatedAt: number;
}

// ── 命令入参 DTO ────────────────────────────────────────────────────────

export interface CreateDiagnosisFromImageInput {
  /** 截图 base64（支持 data:image/png;base64,XXX 或裸 base64） */
  imageBase64: string;
  /** 截图来源 App（可选） */
  sourceApp?: string | null;
  /** LLM 供应商 ID */
  providerId: string;
  /** 视觉模型 ID */
  modelId: string;
}

export interface UpdateDiagnosisInput {
  diagnosisId: string;
  narrative?: string;
  recommendedActions?: string[];
  status?: string;
  errorMessage?: string | null;
}

export interface ConvertToPaperPortfolioInput {
  diagnosisId: string;
  name: string;
  sourceEvent: string;
}
