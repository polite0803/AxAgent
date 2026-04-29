import { invoke } from "@/lib/invoke";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";

interface ProfileData {
  name: string;
  display_name: string;
  is_default: boolean;
}

interface ProfileInfo {
  profile: ProfileData;
}

export function ProfileSelector() {
  const { t } = useTranslation();
  const [profiles, setProfiles] = useState<ProfileInfo[]>([]);
  const [active, setActive] = useState("default");

  const load = async () => {
    try {
      const list = await invoke<ProfileInfo[]>("profile_list");
      setProfiles(list);
      const current = await invoke<ProfileInfo>("profile_active");
      setActive(current.profile.name);
    } catch {}
  };

  useEffect(() => {
    load();
  }, []);

  const handleSwitch = async (name: string) => {
    await invoke("profile_switch", { name });
    setActive(name);
  };

  return (
    <SettingsGroup title={t("settings.profiles", "Profiles")}>
      <div className="flex items-center justify-between py-1">
        <span>{t("settings.activeProfile", "Active Profile")}</span>
        <select
          value={active}
          onChange={(e) => handleSwitch(e.target.value)}
          className="border rounded px-2 py-1 text-sm bg-transparent"
        >
          {profiles.map((p) => (
            <option key={p.profile.name} value={p.profile.name}>
              {p.profile.display_name}
            </option>
          ))}
        </select>
      </div>
    </SettingsGroup>
  );
}
