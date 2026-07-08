import { createContext, useContext } from "react";

/**
 * 股票分析页内部上下文：让右侧栏子面板能够"打开数据源设置"等页面级动作。
 * 由 <StockAnalysisPage> 提供；脱离该页面渲染时降级为 no-op。
 */
export interface StockAnalysisPageActions {
  /** 打开股票分析设置（默认聚焦"数据源"Tab） */
  openDataSourceSettings: () => void;
}

const noop = () => {};

export const StockAnalysisPageContext = createContext<StockAnalysisPageActions>({
  openDataSourceSettings: noop,
});

export function useStockAnalysisPage(): StockAnalysisPageActions {
  return useContext(StockAnalysisPageContext);
}
