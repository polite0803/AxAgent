import { invoke } from "@/lib/invoke";
import { DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import { Button, Input, Modal, Typography } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ProfileInfo } from "./ProfileSelector";

const { Text } = Typography;

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
    if (!newName.trim()) {
      return;
    }
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
        <Text strong>{t("settings.profileManager")}</Text>
        <Button
          size="small"
          type="primary"
          icon={<PlusOutlined />}
          onClick={() => setModalOpen(true)}
        >
          {t("settings.newProfile")}
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
              {t("common.delete")}
            </Button>
          )}
        </div>
      ))}

      <Modal
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleCreate}
        title={t("settings.createProfile")}
      >
        <div className="space-y-3 py-2">
          <Input
            id="profile-manager-input-108"
            placeholder={t("settings.profileName")}
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
          />
          <Input
            id="profile-manager-input-109"
            placeholder={t("settings.profileDisplayName")}
            value={newDisplayName}
            onChange={(e) => setNewDisplayName(e.target.value)}
          />
        </div>
      </Modal>
    </div>
  );
}
