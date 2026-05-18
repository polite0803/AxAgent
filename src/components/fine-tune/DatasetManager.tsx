import { useFineTuneStore } from "@/stores/devtools/fineTuneStore";
import { DeleteOutlined, FileTextOutlined, PlusOutlined } from "@ant-design/icons";
import { Button, Card, Form, Input, message, Modal, Popconfirm, Space, Table } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Column } = Table;

export function DatasetManager() {
  const { t } = useTranslation();
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
        title={t("datasetManager.title")}
        extra={
          <Button type="primary" icon={<PlusOutlined />} onClick={() => setCreateModalVisible(true)}>
            Create Dataset
          </Button>
        }
      >
        {datasets.length === 0
          ? (
            <div className="text-center py-8 text-zinc-500">
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
              <Column title={t("datasetManager.name")} dataIndex="name" key="name" />
              <Column title={t("datasetManager.description")} dataIndex="description" key="description" ellipsis />
              <Column title={t("datasetManager.samples")} dataIndex="num_samples" key="num_samples" />
              <Column
                title={t("datasetManager.created")}
                dataIndex="created_at"
                key="created_at"
                render={(date: string) => new Date(date).toLocaleDateString()}
              />
              <Column
                title={t("datasetManager.action")}
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
                      okText={t("datasetManager.yes")}
                      cancelText={t("datasetManager.no")}
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
        title={t("datasetManager.createNew")}
        open={createModalVisible}
        onCancel={() => setCreateModalVisible(false)}
        footer={null}
      >
        <Form form={form} onFinish={handleCreateDataset} layout="vertical">
          <Form.Item
            name="name"
            label={t("devtools.fineTune.datasetName")}
            rules={[{ required: true, message: "Please input dataset name" }]}
          >
            <Input name="name" placeholder={t("devtools.fineTune.datasetNamePlaceholder")} />
          </Form.Item>
          <Form.Item
            name="description"
            label={t("devtools.fineTune.description")}
            rules={[{ required: true, message: "Please input description" }]}
          >
            <Input.TextArea name="description" placeholder={t("devtools.fineTune.datasetDescPlaceholder")} rows={3} />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit">
                {t("common.create")}
              </Button>
              <Button onClick={() => setCreateModalVisible(false)}>{t("common.cancel")}</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title={t("devtools.fineTune.addSample")}
        open={addSampleModalVisible}
        onCancel={() => setAddSampleModalVisible(false)}
        footer={null}
      >
        <Form form={sampleForm} onFinish={handleAddSample} layout="vertical">
          <Form.Item
            name="input"
            label={t("devtools.fineTune.input")}
            rules={[{ required: true, message: "Please input the sample input" }]}
          >
            <Input.TextArea name="input" placeholder={t("devtools.fineTune.inputPlaceholder")} rows={3} />
          </Form.Item>
          <Form.Item
            name="output"
            label={t("devtools.fineTune.output")}
            rules={[{ required: true, message: "Please input the sample output" }]}
          >
            <Input.TextArea name="output" placeholder={t("devtools.fineTune.outputPlaceholder")} rows={3} />
          </Form.Item>
          <Form.Item name="systemPrompt" label={t("devtools.fineTune.systemPromptOptional")}>
            <Input.TextArea name="systemPrompt" placeholder={t("devtools.fineTune.systemPromptPlaceholder")} rows={2} />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit">
                Add Sample
              </Button>
              <Button onClick={() => setAddSampleModalVisible(false)}>{t("datasetManager.cancel")}</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
