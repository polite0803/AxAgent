import { Alert, Button, Result } from "antd";
import { RefreshCw } from "lucide-react";
import { Component, type ReactNode } from "react";

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
        <Result
          status="warning"
          title={`技能 "${this.props.skillName}" 加载失败`}
          subTitle={this.state.error?.message || "未知错误"}
          extra={
            <Button type="primary" icon={<RefreshCw size={14} />} onClick={this.handleRetry}>
              重试
            </Button>
          }
        />
      );
    }
    return this.props.children;
  }
}

/** 通用错误展示（非 boundary 场景） */
export function SkillErrorFallback({ skillName, error, onRetry }: {
  skillName: string;
  error: string;
  onRetry?: () => void;
}) {
  return (
    <Alert
      type="error"
      showIcon
      message={`技能 "${skillName}" 加载失败`}
      description={error}
      action={onRetry && <Button size="small" onClick={onRetry} icon={<RefreshCw size={12} />}>重试</Button>}
      style={{ margin: 16 }}
    />
  );
}

/** 通用加载骨架 */
export function SkillLoadingSkeleton() {
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
      <div>加载技能组件...</div>
    </div>
  );
}
