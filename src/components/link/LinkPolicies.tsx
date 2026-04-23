import { useTranslation } from 'react-i18next';
import { Card, Form, Select, InputNumber, Switch, Button, App, Empty } from 'antd';
import { Save } from 'lucide-react';
import { useEffect } from 'react';
import { useGatewayLinkStore } from '@/stores';
import type { GatewayLink } from '@/types';

interface LinkPoliciesProps {
  link: GatewayLink;
}

export function LinkPolicies({ link }: LinkPoliciesProps) {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const policy = useGatewayLinkStore((s) => s.policy);
  const fetchPolicy = useGatewayLinkStore((s) => s.fetchPolicy);
  const savePolicy = useGatewayLinkStore((s) => s.savePolicy);
  const [form] = Form.useForm();

  useEffect(() => {
    if (policy) {
      form.setFieldsValue({
        route_strategy: policy.route_strategy,
        model_fallback_enabled: policy.model_fallback_enabled,
        global_rpm: policy.global_rpm,
        per_model_rpm: policy.per_model_rpm,
        token_limit_per_minute: policy.token_limit_per_minute,
        key_rotation_strategy: policy.key_rotation_strategy,
        key_failover_enabled: policy.key_failover_enabled,
      });
    }
  }, [policy, form]);

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      await savePolicy(link.id, values);
      message.success(t('link.policySaved'));
    } catch {
      message.error(t('link.policySaveFailed'));
    }
  };

  if (!policy) {
    return (
      <Empty description={t('link.noPolicy')} image={Empty.PRESENTED_IMAGE_SIMPLE}>
        <Button type="primary" onClick={() => fetchPolicy(link.id)}>
          {t('link.loadPolicy')}
        </Button>
      </Empty>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      <Form
        form={form}
        layout="vertical"
        initialValues={{
          route_strategy: policy.route_strategy,
          model_fallback_enabled: policy.model_fallback_enabled,
          global_rpm: policy.global_rpm,
          per_model_rpm: policy.per_model_rpm,
          token_limit_per_minute: policy.token_limit_per_minute,
          key_rotation_strategy: policy.key_rotation_strategy,
          key_failover_enabled: policy.key_failover_enabled,
        }}
      >
        <Card size="small" title={t('link.routingPolicy')}>
          <Form.Item name="route_strategy" label={t('link.routeStrategy')}>
            <Select>
              <Select.Option value="round_robin">{t('link.roundRobin')}</Select.Option>
              <Select.Option value="least_latency">{t('link.leastLatency')}</Select.Option>
              <Select.Option value="weighted">{t('link.weighted')}</Select.Option>
            </Select>
          </Form.Item>
          <Form.Item name="model_fallback_enabled" label={t('link.modelFallback')} valuePropName="checked">
            <Switch />
          </Form.Item>
        </Card>

        <Card size="small" title={t('link.rateLimiting')}>
          <Form.Item name="global_rpm" label={t('link.globalRpm')}>
            <InputNumber min={1} style={{ width: '100%' }} placeholder={t('link.unlimited')} />
          </Form.Item>
          <Form.Item name="per_model_rpm" label={t('link.perModelRpm')}>
            <InputNumber min={1} style={{ width: '100%' }} placeholder={t('link.unlimited')} />
          </Form.Item>
          <Form.Item name="token_limit_per_minute" label={t('link.tokenLimitPerMinute')}>
            <InputNumber min={1} style={{ width: '100%' }} placeholder={t('link.unlimited')} />
          </Form.Item>
        </Card>

        <Card size="small" title={t('link.keyManagement')}>
          <Form.Item name="key_rotation_strategy" label={t('link.keyRotation')}>
            <Select>
              <Select.Option value="sequential">{t('link.sequential')}</Select.Option>
              <Select.Option value="random">{t('link.random')}</Select.Option>
            </Select>
          </Form.Item>
          <Form.Item name="key_failover_enabled" label={t('link.keyFailover')} valuePropName="checked">
            <Switch />
          </Form.Item>
        </Card>
      </Form>

      <div className="flex justify-end">
        <Button
          type="primary"
          icon={<Save size={14} />}
          onClick={handleSave}
        >
          {t('link.savePolicy')}
        </Button>
      </div>
    </div>
  );
}
