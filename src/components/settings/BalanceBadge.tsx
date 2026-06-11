// SPDX-License-Identifier: AGPL-3.0-only

import { Tooltip } from "@/components/layout/Tooltip";
import { useProviderStore } from "@/stores";
import { Badge, Spin } from "antd";
import { Wallet } from "lucide-react";
import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface BalanceData {
  is_available: boolean;
  balance_infos: Array<{ currency: string; total_balance: string }>;
}

export const BalanceBadge: React.FC = () => {
  const { t } = useTranslation();
  const fetchBalance = useProviderStore((s) => (s as any).fetchBalance); // eslint-disable-line @typescript-eslint/no-explicit-any
  const [balance, setBalance] = useState<BalanceData | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      setLoading(true);
      const data = await fetchBalance();
      if (!cancelled) {
        setBalance(data);
        setLoading(false);
      }
    };
    // Check every 5 minutes
    load();
    const interval = setInterval(load, 5 * 60 * 1000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, [fetchBalance]);

  if (!balance || loading) { return null; }

  const cnyInfo = balance.balance_infos.find((b: BalanceData["balance_infos"][number]) => b.currency === "CNY");
  const usdInfo = balance.balance_infos.find((b: BalanceData["balance_infos"][number]) => b.currency === "USD");
  const total = cnyInfo?.total_balance || usdInfo?.total_balance || "";
  const currency = cnyInfo ? "CNY" : usdInfo ? "USD" : "";
  const isLow = cnyInfo
    ? parseFloat(cnyInfo.total_balance) < 1
    : usdInfo
    ? parseFloat(usdInfo.total_balance) < 0.15
    : false;

  return (
    <Tooltip title={t("settings.provider.deepseekBalance")}>
      <Badge
        count={loading ? <Spin size="small" /> : undefined}
        size="small"
        style={{ backgroundColor: isLow ? "#ff4d4f" : "#52c41a" }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 4, padding: "2px 8px", cursor: "pointer" }}>
          <Wallet size={14} />
          <span style={{ fontSize: 11, fontWeight: 500 }}>
            {total} {currency}
          </span>
        </div>
      </Badge>
    </Tooltip>
  );
};
