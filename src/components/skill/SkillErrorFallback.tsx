import { Alert, Button, Result } from "antd";
import { RefreshCw } from "lucide-react";
import { Component, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

interface SkillErrorBoundaryProps {
  skillName: string;
  children: ReactNode;
}

interface SkillErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class SkillErrorBoundary extends Component<SkillErrorBoundaryProps, SkillErrorBoundaryState> {
  state: SkillErrorBoundaryState = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): SkillErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error(`[SkillErrorBoundary] ${this.props.skillName}:`, error, info);
  }

  handleRetry = () => {
    this.setState({ hasError: false, error: null });
  };

  render() {
    if (this.state.hasError) {
      return (
        <SkillErrorBoundaryInner
          skillName={this.props.skillName}
          error={this.state.error}
          onRetry={this.handleRetry}
        />
      );
    }
    return this.props.children;
  }
}

/** Class 组件内不能直接 useTranslation，拆出函数组件 */
function SkillErrorBoundaryInner({ skillName, error, onRetry }: {
  skillName: string;
  error: Error | null;
  onRetry: () => void;
}) {
  const { t } = useTranslation();
  return (
    <Result
      status="warning"
      title={t("skill.loadFailed", { name: skillName })}
      subTitle={error?.message || t("skill.unknownError")}
      extra={
        <Button type="primary" icon={<RefreshCw size={14} />} onClick={onRetry}>
          {t("skill.retry")}
        </Button>
      }
    />
  );
}

/** 通用错误展示（非 boundary 场景） */
export function SkillErrorFallback({ skillName, error, onRetry }: {
  skillName: string;
  error: string;
  onRetry?: () => void;
}) {
  const { t } = useTranslation();
  return (
    <Alert
      type="error"
      showIcon
      message={t("skill.loadFailed", { name: skillName })}
      description={error}
      action={onRetry && (
        <Button size="small" onClick={onRetry} icon={<RefreshCw size={12} />}>
          {t("skill.retry")}
        </Button>
      )}
      style={{ margin: 16 }}
    />
  );
}

/** 通用加载骨架 */
export function SkillLoadingSkeleton() {
  const { t } = useTranslation();
  return (
    <div style={{ padding: 24, textAlign: "center", color: "var(--color-text-secondary)" }}>
      <div
        style={{
          width: 32,
          height: 32,
          border: "3px solid #eee",
          borderTopColor: "var(--color-primary)",
          borderRadius: "50%",
          animation: "spin 0.8s linear infinite",
          margin: "0 auto 12px",
        }}
      />
      <div>{t("skill.loading")}</div>
    </div>
  );
}
