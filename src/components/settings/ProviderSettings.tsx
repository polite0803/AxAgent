// SPDX-License-Identifier: AGPL-3.0-only

import { useProviderStore, useUIStore } from "@/stores";
import { Button, Spin, theme } from "antd";
import { ArrowLeft } from "lucide-react";
import { lazy, Suspense, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ProviderList } from "./ProviderList";

const ProviderDetail = lazy(() => import("./ProviderDetail").then((m) => ({ default: m.ProviderDetail })));

export function ProviderSettings() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const fetchProviders = useProviderStore((s) => s.fetchProviders);
  const selectedProviderId = useUIStore((s) => s.selectedProviderId);
  const setSelectedProviderId = useUIStore((s) => s.setSelectedProviderId);
  const deviceLayout = useUIStore((s) => s.deviceLayout);
  const isSmall = deviceLayout === "mobile" || deviceLayout === "tablet";

  useEffect(() => {
    fetchProviders();
  }, [fetchProviders]);

  /* 小屏：列表和详情二选一显示（详情页显示返回按钮） */
  const showList = !isSmall || !selectedProviderId;
  const showDetail = !isSmall || !!selectedProviderId;

  return (
    <div className="flex h-full">
      {showList && (
        <div
          className={`${isSmall ? "flex-1 min-w-0" : "w-64 shrink-0"} pt-2`}
          style={!isSmall ? { borderRight: "1px solid var(--border-color)" } : undefined}
        >
          <ProviderList />
        </div>
      )}
      {showDetail && (
        <div className="min-w-0 flex-1 overflow-y-auto p-4 pt-4">
          {selectedProviderId
            ? (
              <>
                {isSmall && (
                  <Button
                    type="text"
                    icon={<ArrowLeft size={16} />}
                    onClick={() => setSelectedProviderId(null)}
                    style={{ marginBottom: 8 }}
                  >
                    {t("common.back")}
                  </Button>
                )}
                <Suspense
                  fallback={
                    <div className="flex h-full items-center justify-center">
                      <Spin />
                    </div>
                  }
                >
                  <ProviderDetail providerId={selectedProviderId} />
                </Suspense>
              </>
            )
            : !isSmall && (
              <div
                className="flex h-full items-center justify-center"
                style={{ color: token.colorTextSecondary }}
              >
                <p>{t("settings.selectProvider")}</p>
              </div>
            )}
        </div>
      )}
    </div>
  );
}
