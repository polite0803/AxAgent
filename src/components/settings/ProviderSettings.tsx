import { useProviderStore, useUIStore } from "@/stores";
import { Spin, theme } from "antd";
import { lazy, Suspense, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ProviderList } from "./ProviderList";

const ProviderDetail = lazy(() => import("./ProviderDetail").then((m) => ({ default: m.ProviderDetail })));

export function ProviderSettings() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const fetchProviders = useProviderStore((s) => s.fetchProviders);
  const selectedProviderId = useUIStore((s) => s.selectedProviderId);

  useEffect(() => {
    fetchProviders();
  }, [fetchProviders]);

  return (
    <div className="flex h-full">
      <div className="w-64 shrink-0 pt-2" style={{ borderRight: "1px solid var(--border-color)" }}>
        <ProviderList />
      </div>
      <div className="min-w-0 flex-1 overflow-y-auto p-4 pt-4">
        {selectedProviderId
          ? (
            <Suspense
              fallback={
                <div className="flex h-full items-center justify-center">
                  <Spin />
                </div>
              }
            >
              <ProviderDetail providerId={selectedProviderId} />
            </Suspense>
          )
          : (
            <div className="flex h-full items-center justify-center" style={{ color: token.colorTextSecondary }}>
              <p>{t("settings.selectProvider")}</p>
            </div>
          )}
      </div>
    </div>
  );
}
