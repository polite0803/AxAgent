// SPDX-License-Identifier: AGPL-3.0-only

import { useTaskShapeApprovalStore } from "@/stores";
import { Button, Modal, Progress, Tag, Typography } from "antd";
import { AlertTriangle, CheckCircle, XCircle } from "lucide-react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

export function TaskShapeApprovalModal() {
  const { t } = useTranslation();
  const pendingApproval = useTaskShapeApprovalStore((s) => s.pendingApproval);
  const loading = useTaskShapeApprovalStore((s) => s.loading);
  const approve = useTaskShapeApprovalStore((s) => s.approve);
  const reject = useTaskShapeApprovalStore((s) => s.reject);

  if (!pendingApproval) { return null; }

  return (
    <Modal
      open
      title={
        <span className="flex items-center gap-2">
          <AlertTriangle size={18} className="text-red-500" />
          {t("taskShapeApproval.title")}
        </span>
      }
      closable={false}
      maskClosable={false}
      footer={
        <div className="flex justify-end gap-2">
          <Button
            danger
            icon={<XCircle size={14} />}
            loading={loading}
            onClick={() => reject(pendingApproval.approvalId)}
          >
            {t("taskShapeApproval.reject")}
          </Button>
          <Button
            type="primary"
            icon={<CheckCircle size={14} />}
            loading={loading}
            onClick={() => approve(pendingApproval.approvalId)}
          >
            {t("taskShapeApproval.approve")}
          </Button>
        </div>
      }
      width={520}
    >
      <div className="space-y-3">
        {/* 输入预览 */}
        <div>
          <Text type="secondary" className="text-xs">
            {t("taskShapeApproval.inputPreview")}
          </Text>
          <Paragraph
            className="bg-gray-50 dark:bg-gray-900 rounded p-2 mt-1 mb-0"
            ellipsis={{ rows: 3, expandable: true }}
            copyable
          >
            {pendingApproval.inputPreview}
          </Paragraph>
        </div>

        {/* 两轴评分 */}
        <div className="flex gap-4">
          <div className="flex-1">
            <Text type="secondary" className="text-xs">
              {t("cognitiveRoute.taskShape.mergeScore")}
            </Text>
            <Progress
              percent={Math.round(pendingApproval.mergeScore * 100)}
              size="small"
              status="active"
            />
          </div>
          <div className="flex-1">
            <Text type="secondary" className="text-xs">
              {t("cognitiveRoute.taskShape.splitScore")}
            </Text>
            <Progress
              percent={Math.round(pendingApproval.splitScore * 100)}
              size="small"
              status="exception"
            />
          </div>
        </div>

        {/* 证据 */}
        {pendingApproval.evidence.length > 0 && (
          <div>
            <Text type="secondary" className="text-xs">
              {t("cognitiveRoute.taskShape.evidence")}
            </Text>
            <div className="flex flex-wrap gap-1 mt-1">
              {pendingApproval.evidence.map((e, i) => (
                <Tag key={i} color="red">
                  {e}
                </Tag>
              ))}
            </div>
          </div>
        )}
      </div>
    </Modal>
  );
}
