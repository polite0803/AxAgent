// SPDX-License-Identifier: AGPL-3.0-only

import { InvestHub } from "@/components/invest/InvestHub";

/** 投资业务统一入口页面 — 薄包装，实际逻辑在 InvestHub */
export function InvestPage() {
  return <InvestHub />;
}
