import { Modal, type ModalProps, Typography } from "antd";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

export interface BaseModalProps extends Omit<ModalProps, "onOk"> {
  /** 确认按钮加载状态 */
  confirming?: boolean;
  /** 错误信息（显示在 footer 区域） */
  error?: string | null;
  /** 确认回调 */
  onOk?: () => void | Promise<void>;
  children: ReactNode;
}

/**
 * 共享 Modal 基础组件
 *
 * 封装 Ant Design Modal 的通用模式：
 * - 统一的 loading / error 状态
 * - 标准化的确认/取消按钮
 */
export function BaseModal({
  confirming = false,
  error = null,
  onOk,
  children,
  okText,
  cancelText,
  ...rest
}: BaseModalProps) {
  const { t } = useTranslation();
  return (
    <Modal
      {...rest}
      confirmLoading={confirming}
      onOk={async () => {
        if (onOk) {
          try {
            await onOk();
          } catch {
            // 错误由调用方通过 error prop 处理
          }
        }
      }}
      okText={okText ?? t("common.confirm")}
      cancelText={cancelText ?? t("common.cancel")}
    >
      {children}
      {error && (
        <Text
          type="danger"
          style={{ display: "block", marginTop: 12, fontSize: 12 }}
        >
          {error}
        </Text>
      )}
    </Modal>
  );
}
