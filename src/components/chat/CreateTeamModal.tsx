import { MinusCircleOutlined, PlusOutlined } from "@ant-design/icons";
import { Button, Form, Input, Modal, Select, Space } from "antd";
import { Users } from "lucide-react";
import { useTranslation } from "react-i18next";

export type TeammateBackendType = "InProcess" | "SubProcess";

export interface TeammateConfig {
  name: string;
  backendType: TeammateBackendType;
}

export interface CreateTeamData {
  teamName: string;
  teammates: TeammateConfig[];
}

interface CreateTeamModalProps {
  open: boolean;
  onCancel: () => void;
  onCreate: (data: CreateTeamData) => void;
  loading?: boolean;
}

export function CreateTeamModal({
  open,
  onCancel,
  onCreate,
  loading = false,
}: CreateTeamModalProps) {
  const [form] = Form.useForm<CreateTeamData>();
  const { t } = useTranslation();

  const handleOk = async () => {
    try {
      const values = await form.validateFields();
      onCreate(values);
      form.resetFields();
    } catch {
      // 表单校验失败，不做处理
    }
  };

  const handleCancel = () => {
    form.resetFields();
    onCancel();
  };

  return (
    <Modal
      title={
        <span className="flex items-center gap-2">
          <Users size={18} />
          {t("createTeam.createTeam")}
        </span>
      }
      open={open}
      onOk={handleOk}
      onCancel={handleCancel}
      confirmLoading={loading}
      okText={t("createTeam.createTeam")}
      cancelText={t("common.cancel")}
      destroyOnHidden
      width={560}
    >
      <Form
        form={form}
        layout="vertical"
        initialValues={{
          teamName: "",
          teammates: [{ name: "", backendType: "InProcess" }],
        }}
        style={{ marginTop: 16 }}
      >
        <Form.Item
          name="teamName"
          label={t("createTeam.teamName")}
          rules={[{ required: true, message: t("createTeam.teamNameRequired") }]}
        >
          <Input name="teamName" placeholder={t("createTeam.teamNamePlaceholder")} />
        </Form.Item>

        <Form.Item label={t("createTeam.memberList")}>
          <Form.List name="teammates">
            {(fields, { add, remove }) => (
              <>
                {fields.map(({ key, name, ...rest }) => (
                  <Space
                    key={key}
                    align="baseline"
                    style={{ display: "flex", marginBottom: 8 }}
                  >
                    <Form.Item
                      {...rest}
                      name={[name, "name"]}
                      rules={[{ required: true, message: t("createTeam.memberNameRequired") }]}
                      style={{ marginBottom: 0 }}
                    >
                      <Input id="create-team-modal-input-12" placeholder={t("createTeam.memberNamePlaceholder")} style={{ width: 200 }} />
                    </Form.Item>

                    <Form.Item
                      {...rest}
                      name={[name, "backendType"]}
                      rules={[{ required: true, message: t("createTeam.backendTypeRequired") }]}
                      style={{ marginBottom: 0 }}
                    >
                      <Select
                        placeholder={t("createTeam.backendTypePlaceholder")}
                        style={{ width: 140 }}
                        options={[
                          { label: t("createTeam.inProcess"), value: "InProcess" },
                          { label: t("createTeam.subProcess"), value: "SubProcess" },
                        ]}
                      />
                    </Form.Item>

                    {fields.length > 1 && (
                      <Button
                        type="text"
                        danger
                        icon={<MinusCircleOutlined />}
                        onClick={() => remove(name)}
                        style={{ marginBottom: 0 }}
                      />
                    )}
                  </Space>
                ))}

                <Button
                  type="dashed"
                  onClick={() => add({ name: "", backendType: "InProcess" })}
                  icon={<PlusOutlined />}
                  block
                >
                  {t("createTeam.addTeammate")}
                </Button>
              </>
            )}
          </Form.List>
        </Form.Item>
      </Form>
    </Modal>
  );
}

export default CreateTeamModal;
