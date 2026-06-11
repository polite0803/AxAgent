// SPDX-License-Identifier: AGPL-3.0-only

import { SourceManager } from "@/components/settings/SourceManager";
import { useTranslation } from "react-i18next";

export function KnowledgeHubPage() {
  const { t } = useTranslation();

  return (
    <div className="kb-layout">
      <div className="kb-header">
        <div className="kb-header-title">{t("nav.knowledge")}</div>
      </div>
      <div className="kb-body">
        <SourceManager />
      </div>
    </div>
  );
}
