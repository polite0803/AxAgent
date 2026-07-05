// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, logIpcError } from "@/lib/invoke";
import { Select } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

export function AgentProfileSelect({
  value,
  onChange,
}: {
  value: string;
  onChange: (profileId: string) => void;
}) {
  const { t } = useTranslation();
  const [profiles, setProfiles] = useState<{ id: string; name: string }[]>([]);

  useEffect(() => {
    invoke<{ id: string; name: string }[]>("list_agent_profiles")
      .then(setProfiles)
      .catch(logIpcError("AgentProfileSelect: load profiles"));
  }, []);

  return (
    <Select
      size="small"
      style={{ minWidth: 120 }}
      value={value || undefined}
      onChange={(v) => onChange(v)}
      placeholder={t("chat.workflow.agentProfileRole")}
      options={profiles.map((p) => ({ value: p.id, label: p.name }))}
      allowClear
    />
  );
}
