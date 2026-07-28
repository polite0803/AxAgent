// SPDX-License-Identifier: AGPL-3.0-only

/**
 * AddMemberModal — 添加成员弹窗。
 *
 * 复用 officeStore.addMember + businessRoleStore.roles + sub_agent_list。
 * - agent_id 支持从已注册 SubAgent 选择，也支持手动输入外部 session ID
 * - role 文本域可由 BusinessRole 快捷下拉自动填充（注入 dispatcher prompt）
 * - room_id 下拉项来自当前 fleet 的场景模板房间列表
 */
import { resolveSceneTemplate } from "@/components/office/phaser/sceneTemplates";
import { invoke } from "@/lib/invoke";
import { message } from "@/lib/toast";
import { useBusinessRoleStore } from "@/stores";
import { useOfficeStore } from "@/stores";
import type { AddMemberInput, SubAgent } from "@/types";
import { Button, Form, Input, Modal, Select, Space } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

export interface AddMemberModalProps {
  open: boolean;
  fleetId: string;
  sceneTemplateSlug?: string;
  onClose: () => void;
}

interface FormValues {
  agentId: string;
  agentSlug: string;
  displayName: string;
  role: string;
  roomId: string;
}

export function AddMemberModal({ open, fleetId, sceneTemplateSlug, onClose }: AddMemberModalProps) {
  const { t } = useTranslation();
  const addMember = useOfficeStore((s) => s.addMember);
  const roles = useBusinessRoleStore((s) => s.roles);
  const fetchRoles = useBusinessRoleStore((s) => s.fetchRoles);

  const [subAgents, setSubAgents] = useState<SubAgent[]>([]);
  const [loading, setLoading] = useState(false);
  const [form] = Form.useForm<FormValues>();

  // 初次打开加载 SubAgent 列表与 BusinessRole 列表
  useEffect(() => {
    if (!open) {
      return;
    }
    void (async () => {
      try {
        const list = await invoke<SubAgent[]>("sub_agent_list");
        setSubAgents(list ?? []);
      } catch (e) {
        // sub_agent_list 失败不阻塞表单，用户可手动输入 agent_id
        console.warn("[AddMemberModal] sub_agent_list failed:", e);
        setSubAgents([]);
      }
    })();
    if (roles.length === 0) {
      void fetchRoles();
    }
  }, [open, roles.length, fetchRoles]);

  // 默认房间 = 当前场景模板的 defaultRoomId
  const template = resolveSceneTemplate(sceneTemplateSlug);
  const roomOptions = template.rooms.map((r) => ({
    value: r.id,
    label: t(`office.room.${r.nameKey}`) || r.id,
  }));

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      setLoading(true);
      const input: AddMemberInput = {
        fleetId,
        agentId: values.agentId.trim(),
        agentSlug: values.agentSlug.trim(),
        displayName: values.displayName.trim(),
        role: values.role?.trim() ?? "",
        roomId: values.roomId ?? template.defaultRoomId,
      };
      const member = await addMember(input);
      if (member) {
        message.success(t("office.addMember.success", { name: member.displayName }));
        form.resetFields();
        onClose();
      }
    } catch (e) {
      // 表单校验失败或 IPC 失败
      if (e instanceof Error && e.message) {
        message.error(e.message);
      }
    } finally {
      setLoading(false);
    }
  };

  const handleCancel = () => {
    form.resetFields();
    onClose();
  };

  // 选择 SubAgent 后自动填充 slug / displayName
  const handleSubAgentPick = (agentId: string) => {
    const sub = subAgents.find((a) => a.id === agentId);
    if (!sub) {
      return;
    }
    form.setFieldsValue({
      agentId: sub.id,
      agentSlug: sub.name?.toLowerCase().replace(/\s+/g, "_") ?? sub.id,
      displayName: sub.name ?? sub.id,
    });
  };

  // 选择 BusinessRole 后填充 role 文本域（注入 dispatcher prompt）
  const handleRolePick = (roleId: string) => {
    const role = roles.find((r) => r.id === roleId);
    if (!role) {
      return;
    }
    const roleText = role.systemPrompt
      ? `[${role.name}] ${role.systemPrompt}`
      : `[${role.name}] ${role.responsibilities?.join(" / ") ?? ""}`;
    form.setFieldValue("role", roleText);
  };

  return (
    <Modal
      title={t("office.addMember.title")}
      open={open}
      onCancel={handleCancel}
      footer={
        <Space>
          <Button onClick={handleCancel}>{t("office.addMember.cancel")}</Button>
          <Button type="primary" loading={loading} onClick={handleSubmit}>
            {t("office.addMember.confirm")}
          </Button>
        </Space>
      }
      width={560}
      destroyOnHidden
    >
      <Form
        form={form}
        layout="vertical"
        initialValues={{ roomId: template.defaultRoomId }}
      >
        {/* SubAgent 快捷选择 */}
        {subAgents.length > 0 && (
          <Form.Item label={t("office.addMember.selectAgent")}>
            <Select
              showSearch
              placeholder={t("office.addMember.selectAgentPlaceholder")}
              optionFilterProp="label"
              options={subAgents.map((a) => ({
                value: a.id,
                label: `${a.name ?? a.id} (${a.status})`,
              }))}
              onChange={handleSubAgentPick}
              allowClear
            />
          </Form.Item>
        )}

        <Form.Item
          name="agentId"
          label={t("office.addMember.agentId")}
          rules={[{ required: true, message: t("office.addMember.agentIdRequired") }]}
        >
          <Input placeholder={t("office.addMember.agentIdPlaceholder")} />
        </Form.Item>

        <Form.Item
          name="agentSlug"
          label={t("office.addMember.agentSlug")}
          rules={[{ required: true, message: t("office.addMember.agentSlugRequired") }]}
        >
          <Input placeholder={t("office.addMember.agentSlugPlaceholder")} />
        </Form.Item>

        <Form.Item
          name="displayName"
          label={t("office.addMember.displayName")}
          rules={[{ required: true, message: t("office.addMember.displayNameRequired") }]}
        >
          <Input placeholder={t("office.addMember.displayNamePlaceholder")} />
        </Form.Item>

        {/* BusinessRole 快捷填充 */}
        {roles.length > 0 && (
          <Form.Item label={t("office.addMember.fillFromRole")}>
            <Select
              showSearch
              placeholder={t("office.addMember.fillFromRolePlaceholder")}
              optionFilterProp="label"
              options={roles
                .filter((r) => r.isEnabled)
                .map((r) => ({
                  value: r.id,
                  label: r.name,
                }))}
              onChange={handleRolePick}
              allowClear
            />
          </Form.Item>
        )}

        <Form.Item name="role" label={t("office.addMember.role")}>
          <Input.TextArea
            rows={3}
            placeholder={t("office.addMember.rolePlaceholder")}
          />
        </Form.Item>

        <Form.Item name="roomId" label={t("office.addMember.roomId")}>
          <Select options={roomOptions} />
        </Form.Item>
      </Form>
    </Modal>
  );
}
