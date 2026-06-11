// SPDX-License-Identifier: AGPL-3.0-only

import { ChartPreview } from "@/components/chat/ArtifactPreview/ChartPreview";
import { type ChartGenRequest, generateChart } from "@/lib/chartGenerator";
import { Spin } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface ChartRendererProps {
  option?: Record<string, unknown>;
  request?: ChartGenRequest;
  width?: number | string;
  height?: number | string;
  theme?: "light" | "dark";
}

export function ChartRenderer({
  option: directOption,
  request,
  width,
  height,
  theme = "light",
}: ChartRendererProps) {
  const { t } = useTranslation();
  const [option, setOption] = useState<Record<string, unknown> | null>(
    directOption || null,
  );
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (directOption) {
      /* eslint-disable react-hooks/set-state-in-effect */
      setOption(directOption);
      setLoading(false);
      setError(null);
      /* eslint-enable react-hooks/set-state-in-effect */
      return;
    }

    if (request) {
      setLoading(true);
      setError(null);
      generateChart(request)
        .then((result) => {
          setOption(result.option);
        })
        .catch((err) => {
          setError(String(err));
        })
        .finally(() => {
          setLoading(false);
        });
    }
  }, [directOption, request]);

  if (loading) {
    return (
      <div
        style={{
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          height: height || 400,
        }}
      >
        <Spin />
      </div>
    );
  }

  if (error) {
    return (
      <div style={{ color: "#ff4d4f", padding: 16 }}>
        {t("chat.workflow.chart.renderError", { error })}
      </div>
    );
  }

  if (!option) {
    return null;
  }

  return <ChartPreview option={option} width={width} height={height} theme={theme} />;
}
