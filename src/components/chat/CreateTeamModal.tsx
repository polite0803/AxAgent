import { MinusCircleOutlined, PlusOutlined } from "@ant-design/icons";
import { Button, Form, Input, Modal, Select, Space } from "antd";
import { Users } from "lucide-react";

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
          创建团队
        </span>
      }
      open={open}
      onOk={handleOk}
      onCancel={handleCancel}
      confirmLoading={loading}
      okText="创建团队"
      cancelText="取消"
      destroyOnClose
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
          label="团队名称"
          rules={[{ required: true, message: "请输入团队名称" }]}
        >
          <Input placeholder="例如：前端开发组、数据分析组" />
        </Form.Item>

        <Form.Item label="队友列表">
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
                      rules={[{ required: true, message: "请输入队友名称" }]}
                      style={{ marginBottom: 0 }}
                    >
                      <Input placeholder="队友名称" style={{ width: 200 }} />
                    </Form.Item>

                    <Form.Item
                      {...rest}
                      name={[name, "backendType"]}
                      rules={[{ required: true, message: "请选择后端类型" }]}
                      style={{ marginBottom: 0 }}
                    >
                      <Select
                        placeholder="后端类型"
                        style={{ width: 140 }}
                        options={[
                          { label: "进程内 (InProcess)", value: "InProcess" },
                          { label: "子进程 (SubProcess)", value: "SubProcess" },
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
                  添加队友
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
