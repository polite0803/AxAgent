import { useFineTuneStore } from "@/stores/devtools/fineTuneStore";
import { RocketOutlined, SettingOutlined } from "@ant-design/icons";
import { Button, Card, Divider, Form, message, Select, Slider, Space } from "antd";
import { useEffect, useState } from "react";

interface LoRAConfigForm {
  datasetId: string;
  baseModel: string;
  rank: number;
  alpha: number;
  learningRate: number;
  batchSize: number;
  epochs: number;
}

export function LoRAConfig() {
  const {
    datasets,
    baseModels,
    fetchDatasets,
    fetchBaseModels,
    createTrainingJob,
  } = useFineTuneStore();

  const [form] = Form.useForm<LoRAConfigForm>();
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    fetchDatasets();
    fetchBaseModels();
  }, [fetchDatasets, fetchBaseModels]);

  const handleSubmit = async (values: LoRAConfigForm) => {
    setIsSubmitting(true);
    try {
      const job = await createTrainingJob(values.datasetId, values.baseModel, {
        rank: values.rank,
        alpha: values.alpha,
        learning_rate: values.learningRate,
        batch_size: values.batchSize,
        epochs: values.epochs,
      });

      if (job) {
        message.success("Training job created successfully");
        form.resetFields();
      }
    } catch (error) {
      message.error("Failed to create training job");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="p-4">
      <Card
        title={
          <Space>
            <SettingOutlined />
            <span>LoRA Training Configuration</span>
          </Space>
        }
      >
        <Form
          form={form}
          onFinish={handleSubmit}
          layout="vertical"
          initialValues={{
            rank: 8,
            alpha: 16,
            learningRate: 0.0002,
            batchSize: 4,
            epochs: 3,
          }}
        >
          <Divider>Training Setup</Divider>

          <Form.Item
            name="datasetId"
            label="Dataset"
            rules={[{ required: true, message: "Please select a dataset" }]}
          >
            <Select placeholder="Select a dataset">
              {datasets.map((dataset) => (
                <Select.Option key={dataset.id} value={dataset.id}>
                  {dataset.name} ({dataset.num_samples} samples)
                </Select.Option>
              ))}
            </Select>
          </Form.Item>

          <Form.Item
            name="baseModel"
            label="Base Model"
            rules={[{ required: true, message: "Please select a base model" }]}
          >
            <Select placeholder="Select a base model">
              {baseModels.map((model) => (
                <Select.Option key={model.model_id} value={model.model_id}>
                  {model.name} ({model.size_gb} GB)
                </Select.Option>
              ))}
            </Select>
          </Form.Item>

          <Divider>LoRA Parameters</Divider>

          <Form.Item
            name="rank"
            label="Rank"
            extra="Higher rank = more parameters, better quality, slower training"
          >
            <Slider min={2} max={64} marks={{ 2: "2", 8: "8", 16: "16", 32: "32", 64: "64" }} />
          </Form.Item>

          <Form.Item
            name="alpha"
            label="Alpha"
            extra="Scaling factor for LoRA weights"
          >
            <Slider min={1} max={128} marks={{ 1: "1", 16: "16", 32: "32", 64: "64", 128: "128" }} />
          </Form.Item>

          <Form.Item
            name="learningRate"
            label="Learning Rate"
          >
            <Slider
              min={0.00001}
              max={0.001}
              step={0.00001}
              marks={{
                0.00001: "0.00001",
                0.0001: "0.0001",
                0.001: "0.001",
              }}
              tooltip={{ formatter: (value) => value?.toFixed(5) }}
            />
          </Form.Item>

          <Form.Item
            name="batchSize"
            label="Batch Size"
          >
            <Slider
              min={1}
              max={16}
              marks={{ 1: "1", 4: "4", 8: "8", 16: "16" }}
            />
          </Form.Item>

          <Form.Item
            name="epochs"
            label="Epochs"
          >
            <Slider
              min={1}
              max={10}
              marks={{ 1: "1", 3: "3", 5: "5", 10: "10" }}
            />
          </Form.Item>

          <Divider />

          <Form.Item>
            <Space>
              <Button
                type="primary"
                htmlType="submit"
                icon={<RocketOutlined />}
                loading={isSubmitting}
              >
                Create Training Job
              </Button>
              <Button onClick={() => form.resetFields()}>Reset</Button>
            </Space>
          </Form.Item>
        </Form>
      </Card>
    </div>
  );
}
