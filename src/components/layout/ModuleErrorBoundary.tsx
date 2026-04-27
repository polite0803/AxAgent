import { ReloadOutlined, WarningOutlined } from "@ant-design/icons";
import { Button, theme, Tooltip, Typography } from "antd";
import React from "react";

const { Text } = Typography;

interface ModuleErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

interface ModuleErrorBoundaryProps {
  children: React.ReactNode;
  moduleName: string;
  fallback?: React.ReactNode;
  onReset?: () => void;
  showDetails?: boolean;
}

class ModuleErrorBoundary extends React.Component<
  ModuleErrorBoundaryProps,
  ModuleErrorBoundaryState
> {
  constructor(props: ModuleErrorBoundaryProps) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
    };
  }

  static getDerivedStateFromError(error: Error): Partial<ModuleErrorBoundaryState> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    if (import.meta.env.DEV) {
      console.error(`ModuleErrorBoundary (${this.props.moduleName}) caught an error:`, error, errorInfo);
    }
  }

  handleRetry = () => {
    this.props.onReset?.();
    this.setState({
      hasError: false,
      error: null,
    });
  };

  render() {
    const { hasError, error } = this.state;
    const { children, moduleName, fallback, showDetails = false } = this.props;

    if (hasError) {
      if (fallback) {
        return fallback;
      }

      return (
        <DefaultModuleFallback moduleName={moduleName} error={showDetails ? error : null} onRetry={this.handleRetry} />
      );
    }

    return children;
  }
}

interface DefaultModuleFallbackProps {
  moduleName: string;
  error?: Error | null;
  onRetry: () => void;
}

function DefaultModuleFallback({ moduleName, error, onRetry }: DefaultModuleFallbackProps) {
  const { token } = theme.useToken();

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        padding: "24px",
        backgroundColor: token.colorBgContainer,
        border: `1px solid ${token.colorBorderSecondary}`,
        borderRadius: token.borderRadius,
        minHeight: 120,
      }}
    >
      <WarningOutlined style={{ fontSize: 24, color: token.colorWarning, marginBottom: 8 }} />
      <Text type="secondary" style={{ marginBottom: 12, textAlign: "center" }}>
        {moduleName} encountered an error
      </Text>
      {error && (
        <Text
          type="secondary"
          style={{
            fontSize: 12,
            marginBottom: 12,
            maxWidth: 300,
            textAlign: "center",
            wordBreak: "break-word",
          }}
        >
          {error.message}
        </Text>
      )}
      <Tooltip title="Retry loading this module">
        <Button
          type="text"
          size="small"
          icon={<ReloadOutlined />}
          onClick={onRetry}
        >
          Retry
        </Button>
      </Tooltip>
    </div>
  );
}

export default ModuleErrorBoundary;
export { DefaultModuleFallback };
export type { ModuleErrorBoundaryProps };
