import { Drawer } from "antd";
import { useTranslation } from "react-i18next";
import { CodeRefactorConfigPanel } from "./CodeRefactorConfigPanel";

interface Props {
  open: boolean;
  onClose: () => void;
  workflowId: string;
}

export function CodeRefactorSettingsModal({ open, onClose, workflowId }: Props) {
  const { t } = useTranslation();

  const titleMap: Record<string, string> = {
    "wf-eng-refactor": t("opc.refactor.settings.titles.fullRefactor"),
    "wf-eng-refactor-lite": t("opc.refactor.settings.titles.liteRefactor"),
    "wf-eng-tech-debt": t("opc.refactor.settings.titles.techDebt"),
  };

  return (
    <Drawer
      title={titleMap[workflowId] ?? t("opc.refactor.settings.title")}
      placement="right"
      width={720}
      open={open}
      onClose={onClose}
      destroyOnHidden
    >
      <CodeRefactorConfigPanel workflowId={workflowId} />
    </Drawer>
  );
}
