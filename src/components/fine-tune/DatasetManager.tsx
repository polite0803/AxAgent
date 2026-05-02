import { useFineTuneStore } from "@/stores/devtools/fineTuneStore";
import { DeleteOutlined, FileTextOutlined, PlusOutlined } from "@ant-design/icons";
import { Button, Card, Form, Input, message, Modal, Popconfirm, Space, Table } from "antd";
import { useEffect, useState } from "react";

const { Column } = Table;

export function DatasetManager() {
  const {
    datasets,
    isLoading,
    fetchDatasets,
    createDataset,
    deleteDataset,
    addSample,
  } = useFineTuneStore();

  const [createModalVisible, setCreateModalVisible] = useState(false);
  const [addSampleModalVisible, setAddSampleModalVisible] = useState(false);
  const [selectedDatasetId, setSelectedDatasetId] = useState<string | null>(null);
  const [form] = Form.useForm();
  const [sampleForm] = Form.useForm();

  useEffect(() => {
    fetchDatasets();
  }, [fetchDatasets]);

  const handleCreateDataset = async (values: { name: string; description: string }) => {
    const dataset = await createDataset(values.name, values.description);
    if (dataset) {
      message.success("Dataset created successfully");
      setCreateModalVisible(false);
      form.resetFields();
    }
  };

  const handleDeleteDataset = async (id: string) => {
    await deleteDataset(id);
    message.success("Dataset deleted");
  };

  const handleAddSample = async (values: { input: string; output: string; systemPrompt?: string }) => {
    if (selectedDatasetId) {
      await addSample(selectedDatasetId, values.input, values.output, values.systemPrompt);
      message.success("Sample added successfully");
      setAddSampleModalVisible(false);
      sampleForm.resetFields();
    }
  };

  const openAddSampleModal = (datasetId: string) => {
    setSelectedDatasetId(datasetId);
    setAddSampleModalVisible(true);
  };

  return (
    <div className="p-4">
      <Card
        title="Fine-Tune Dataset Manager"
        extra={
          <Button type="primary" icon={<PlusOutlined />} onClick={() => setCreateModalVisible(true)}>
            Create Dataset
          </Button>
        }
      >
        {datasets.length === 0
          ? (
            <div className="text-center py-8 text-gray-500">
              No datasets found. Create one to get started.
            </div>
          )
          : (
            <Table
              dataSource={datasets}
              rowKey="id"
              loading={isLoading}
              pagination={false}
            >
              <Column title="Name" dataIndex="name" key="name" />
              <Column title="Description" dataIndex="description" key="description" ellipsis />
              <Column title="Samples" dataIndex="num_samples" key="num_samples" />
              <Column
                title="Created"
                dataIndex="created_at"
                key="created_at"
                render={(date: string) => new Date(date).toLocaleDateString()}
              />
              <Column
                title="Action"
                key="action"
                render={(_: unknown, record: { id: string }) => (
                  <Space>
                    <Button
                      size="small"
                      icon={<FileTextOutlined />}
                      onClick={() => openAddSampleModal(record.id)}
                    >
                      Add Sample
                    </Button>
                    <Popconfirm
                      title="Delete this dataset?"
                      onConfirm={() => handleDeleteDataset(record.id)}
                      okText="Yes"
                      cancelText="No"
                    >
                      <Button size="small" danger icon={<DeleteOutlined />}>
                        Delete
                      </Button>
                    </Popconfirm>
                  </Space>
                )}
              />
            </Table>
          )}
      </Card>

      <Modal
        title="Create New Dataset"
        open={createModalVisible}
        onCancel={() => setCreateModalVisible(false)}
        footer={null}
      >
        <Form form={form} onFinish={handleCreateDataset} layout="vertical">
          <Form.Item
            name="name"
            label="Dataset Name"
            rules={[{ required: true, message: "Please input dataset name" }]}
          >
            <Input placeholder="Enter dataset name" />
          </Form.Item>
          <Form.Item
            name="description"
            label="Description"
            rules={[{ required: true, message: "Please input description" }]}
          >
            <Input.TextArea placeholder="Enter dataset description" rows={3} />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit">
                Create
              </Button>
              <Button onClick={() => setCreateModalVisible(false)}>Cancel</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title="Add Sample to Dataset"
        open={addSampleModalVisible}
        onCancel={() => setAddSampleModalVisible(false)}
        footer={null}
      >
        <Form form={sampleForm} onFinish={handleAddSample} layout="vertical">
          <Form.Item
            name="input"
            label="Input"
            rules={[{ required: true, message: "Please input the sample input" }]}
          >
            <Input.TextArea placeholder="Enter input text" rows={3} />
          </Form.Item>
          <Form.Item
            name="output"
            label="Output"
            rules={[{ required: true, message: "Please input the sample output" }]}
          >
            <Input.TextArea placeholder="Enter output text" rows={3} />
          </Form.Item>
          <Form.Item name="systemPrompt" label="System Prompt (optional)">
            <Input.TextArea placeholder="Enter system prompt" rows={2} />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit">
                Add Sample
              </Button>
              <Button onClick={() => setAddSampleModalVisible(false)}>Cancel</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
