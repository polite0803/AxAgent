// SPDX-License-Identifier: AGPL-3.0-only

import i18n from "@/i18n";
import { CHAT_ICON_COLORS } from "@/lib/iconColors";
import { ProviderIcon } from "@lobehub/icons";
import Tavily from "@lobehub/icons/es/Tavily";
import { Search as SearchIcon } from "lucide-react";

/** Renders the correct icon for a given SearchProviderType */
export function SearchProviderTypeIcon({
  type,
  size = 20,
}: {
  type: string;
  size?: number;
}) {
  switch (type) {
    case "tavily":
      return <Tavily.Color size={size} />;
    case "zhipu":
      return <ProviderIcon provider="zhipu" size={size} type="color" />;
    case "bocha":
      return (
        <img
          src="/icons/bocha.ico"
          alt="Bocha"
          style={{ width: size, height: size }}
        />
      );
    default:
      return <SearchIcon size={size - 2} color={CHAT_ICON_COLORS.Search} />;
  }
}

// eslint-disable-next-line react-refresh/only-export-components
export const PROVIDER_TYPE_LABELS: Record<string, string> = {
  tavily: i18n.t("searchProvider.tavily"),
  zhipu: i18n.t("searchProvider.zhipu"),
  bocha: i18n.t("searchProvider.bocha"),
};
