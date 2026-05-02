import { invoke } from "@/lib/invoke";
import { DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import { Button, Input, Modal, Typography } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";

const { Text } = Typography;

interface ProfileData {
  name: string;
  display_name: string;
  is_default: boolean;
  created_at: number;
}

interface ProfileInfo {
  profile: ProfileData;
  config_path: string;
  db_path: string;
  sessions_path: string;
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

export function ProfileManager() {
  const { t } = useTranslation();
  const [profiles, setProfiles] = useState<ProfileInfo[]>([]);
  const [modalOpen, setModalOpen] = useState(false);
  const [newName, setNewName] = useState("");
  const [newDisplayName, setNewDisplayName] = useState("");

  const load = async () => {
    try {
      const list = await invoke<ProfileInfo[]>("profile_list");
      setProfiles(list);
    } catch {}
  };

  useEffect(() => {
    load();
  }, []);

  const handleCreate = async () => {
    if (!newName.trim()) { return; }
    await invoke("profile_create", {
      name: newName,
      displayName: newDisplayName || newName,
    });
    setNewName("");
    setNewDisplayName("");
    setModalOpen(false);
    load();
  };

  const handleDelete = async (name: string) => {
    await invoke("profile_delete", { name });
    load();
  };

  return (
    <div className="space-y-3">
      <div className="flex justify-between items-center">
        <Text strong>{t("settings.profileManager", "Profile Manager")}</Text>
        <Button
          size="small"
          type="primary"
          icon={<PlusOutlined />}
          onClick={() => setModalOpen(true)}
        >
          {t("settings.newProfile", "New")}
        </Button>
      </div>

      {profiles.map((p) => (
        <div
          key={p.profile.name}
          className="flex justify-between items-center p-3 border rounded"
        >
          <div>
            <Text strong>{p.profile.display_name}</Text>
            <br />
            <Text type="secondary" className="text-xs">
              {p.profile.name}
            </Text>
          </div>
          {!p.profile.is_default && (
            <Button
              size="small"
              danger
              icon={<DeleteOutlined />}
              onClick={() => handleDelete(p.profile.name)}
            >
              {t("common.delete", "Delete")}
            </Button>
          )}
        </div>
      ))}

      <Modal
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleCreate}
        title={t("settings.createProfile", "Create Profile")}
      >
        <div className="space-y-3 py-2">
          <Input
            placeholder={t("settings.profileName", "Name (alphanumeric, hyphens, underscores)")}
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
          />
          <Input
            placeholder={t("settings.profileDisplayName", "Display Name")}
            value={newDisplayName}
            onChange={(e) => setNewDisplayName(e.target.value)}
          />
        </div>
      </Modal>
    </div>
  );
}
