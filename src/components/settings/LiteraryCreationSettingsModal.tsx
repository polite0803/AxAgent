import { LiteraryCreationSettings } from "@/components/settings/LiteraryCreationSettings";
import { Drawer } from "antd";
import { useTranslation } from "react-i18next";

export function LiteraryCreationSettingsModal(
  { open, onClose, defaultTab }: { open: boolean; onClose: () => void; defaultTab?: string },
) {
  const { t } = useTranslation();
  return (
    <Drawer
      title={t("literaryCreation.title")}
      placement="right"
      width={720}
      open={open}
      onClose={onClose}
      destroyOnHidden
    >
      <LiteraryCreationSettings defaultTab={defaultTab} />
    </Drawer>
  );
}
