import React, { useEffect } from "react";
import { useRLStore } from "@/stores/devtools/rlStore";
import { Card, Table, Button, Space, Tag, Modal, Form, Input, Select, message } from "antd";
import { PlusOutlined, DeleteOutlined, PlayCircleOutlined } from "@ant-design/icons";

const { Column } = Table;

export function PolicyDashboard() {
  const {
    policies,
    stats,
    isLoading,
    fetchPolicies,
    fetchStats,
    createPolicy,
    deletePolicy,
    trainPolicy,
  } = useRLStore();

  const [form] = Form.useForm();
  const [modalVisible, setModalVisible] = React.useState(false);

  useEffect(() => {
    fetchPolicies?.();
    fetchStats?.();
  }, [fetchPolicies, fetchStats]);

  const handleCreatePolicy = async (values: { name: string; policyType: string; modelId: string }) => {
    const policy = await createPolicy?.(values.name, values.policyType, values.modelId);
    if (policy) {
      message.success("Policy created successfully");
      setModalVisible(false);
      form.resetFields();
    }
  };

  const handleDeletePolicy = async (policyId: string) => {
    Modal.confirm({
      title: "Delete Policy",
      content: "Are you sure you want to delete this policy?",
      onOk: async () => {
        await deletePolicy?.(policyId);
        message.success("Policy deleted");
      },
    });
  };

  const handleTrainPolicy = async (policyId: string) => {
    await trainPolicy?.(policyId);
    message.info("Training started");
  };

  return (
    <div className="p-4">
      <Card
        title="RL Policy Dashboard"
        extra={
          <Button type="primary" icon={<PlusOutlined />} onClick={() => setModalVisible(true)}>
            Create Policy
          </Button>
        }
      >
        {stats && (
          <div className="grid grid-cols-3 gap-4 mb-4">
            <Card size="small">
              <div className="text-2xl font-bold">{stats.total_policies}</div>
              <div className="text-gray-500">Total Policies</div>
            </Card>
            <Card size="small">
              <div className="text-2xl font-bold">{stats.total_experiences}</div>
              <div className="text-gray-500">Total Experiences</div>
            </Card>
            <Card size="small">
              <div className="text-2xl font-bold">{stats.avg_reward.toFixed(2)}</div>
              <div className="text-gray-500">Average Reward</div>
            </Card>
          </div>
        )}

        <Table
          dataSource={policies}
          rowKey="id"
          loading={isLoading}
          pagination={false}
        >
          <Column title="Name" dataIndex="name" key="name" />
          <Column title="Type" dataIndex="policy_type" key="policy_type" render={(type: string) => (
            <Tag color="blue">{type}</Tag>
          )} />
          <Column title="Experiences" dataIndex="total_experiences" key="total_experiences" />
          <Column title="Avg Reward" dataIndex="avg_reward" key="avg_reward" render={(r: number) => r.toFixed(2)} />
          <Column
            title="Action"
            key="action"
            render={(_: unknown, record: { id: string }) => (
              <Space>
                <Button
                  size="small"
                  icon={<PlayCircleOutlined />}
                  onClick={() => handleTrainPolicy(record.id)}
                >
                  Train
                </Button>
                <Button
                  size="small"
                  danger
                  icon={<DeleteOutlined />}
                  onClick={() => handleDeletePolicy(record.id)}
                />
              </Space>
            )}
          />
        </Table>
      </Card>

      <Modal
        title="Create New Policy"
        open={modalVisible}
        onCancel={() => setModalVisible(false)}
        footer={null}
      >
        <Form form={form} onFinish={handleCreatePolicy} layout="vertical">
          <Form.Item
            name="name"
            label="Policy Name"
            rules={[{ required: true, message: "Please input policy name" }]}
          >
            <Input placeholder="Enter policy name" />
          </Form.Item>
          <Form.Item
            name="policyType"
            label="Policy Type"
            rules={[{ required: true, message: "Please select policy type" }]}
          >
            <Select placeholder="Select policy type">
              <Select.Option value="tool_selection">Tool Selection</Select.Option>
              <Select.Option value="task_decomposition">Task Decomposition</Select.Option>
              <Select.Option value="error_recovery">Error Recovery</Select.Option>
            </Select>
          </Form.Item>
          <Form.Item
            name="modelId"
            label="Model ID"
            rules={[{ required: true, message: "Please input model ID" }]}
          >
            <Input placeholder="Enter model ID" />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit">
                Create
              </Button>
              <Button onClick={() => setModalVisible(false)}>Cancel</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
