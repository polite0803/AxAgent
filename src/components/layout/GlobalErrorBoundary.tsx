import { invoke, logIpcError } from "@/lib/invoke";
import { CheckOutlined, CopyOutlined, ReloadOutlined } from "@ant-design/icons";
import { Button, Space, theme, Typography } from "antd";
import i18next from "i18next";
import React from "react";

const { Text, Paragraph } = Typography;

interface ErrorFallbackProps {
  error: Error;
  errorInfo?: React.ErrorInfo;
  onRetry: () => void;
}

function ErrorFallback({ error, errorInfo, onRetry }: ErrorFallbackProps) {
  const { token } = theme.useToken();
  const [copied, setCopied] = React.useState(false);

  const errorDetails = React.useMemo(() => {
    const stack = errorInfo?.componentStack || error.stack || "";
    return `Error: ${error.message}\n\nStack Trace:\n${stack}`;
  }, [error, errorInfo]);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(errorDetails);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (e) {
      console.error("Failed to copy error details:", e);
    }
  };

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        minHeight: "100vh",
        padding: "48px 24px",
        backgroundColor: token.colorBgContainer,
      }}
    >
      <div
        style={{
          maxWidth: 600,
          width: "100%",
          textAlign: "center",
        }}
      >
        <div style={{ marginBottom: 24 }}>
          <div
            style={{
              fontSize: 64,
              marginBottom: 16,
            }}
          >
            💥
          </div>
          <Text
            strong
            style={{ fontSize: 24, display: "block", marginBottom: 8 }}
          >
            {i18next.t("errorBoundary.somethingWentWrong")}
          </Text>
          <Text type="secondary">
            {i18next.t("errorBoundary.unexpectedError")}
          </Text>
        </div>

        <Space size="middle" style={{ marginBottom: 32 }}>
          <Button
            type="primary"
            icon={<ReloadOutlined />}
            onClick={onRetry}
            size="large"
          >
            {i18next.t("errorBoundary.retry")}
          </Button>
          <Button
            icon={copied ? <CheckOutlined /> : <CopyOutlined />}
            onClick={handleCopy}
            size="large"
          >
            {copied
              ? i18next.t("errorBoundary.copied")
              : i18next.t("errorBoundary.copyError")}
          </Button>
        </Space>

        <div
          style={{
            backgroundColor: token.colorBgElevated,
            border: `1px solid ${token.colorBorderSecondary}`,
            borderRadius: token.borderRadius,
            padding: 16,
            textAlign: "left",
          }}
        >
          <Text
            type="secondary"
            style={{ fontSize: 12, display: "block", marginBottom: 8 }}
          >
            {i18next.t("errorBoundary.errorDetails")}
          </Text>
          <Paragraph
            code
            style={{
              margin: 0,
              fontSize: 12,
              maxHeight: 200,
              overflow: "auto",
              whiteSpace: "pre-wrap",
              wordBreak: "break-all",
            }}
          >
            {error.message}
          </Paragraph>
        </div>
      </div>
    </div>
  );
}

interface GlobalErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: React.ErrorInfo | null;
  retryKey: number;
}

interface GlobalErrorBoundaryProps {
  children: React.ReactNode;
  FallbackComponent?: React.ComponentType<ErrorFallbackProps>;
}

class GlobalErrorBoundary extends React.Component<
  GlobalErrorBoundaryProps,
  GlobalErrorBoundaryState
> {
  constructor(props: GlobalErrorBoundaryProps) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
      retryKey: 0,
    };
  }

  static getDerivedStateFromError(
    error: Error,
  ): Partial<GlobalErrorBoundaryState> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    this.setState({ errorInfo });

    // Log error to console in development
    if (import.meta.env.DEV) {
      console.error("GlobalErrorBoundary caught an error:", error, errorInfo);
    }

    try {
      invoke("telemetry_report_error", {
        error: {
          message: error.message,
          stack: error.stack || "",
          componentStack: errorInfo.componentStack || "",
          url: window.location.href,
          timestamp: Date.now(),
        },
      }).catch(logIpcError("telemetry_report_error"));
    } catch {
      // Error reporting itself failed, nothing we can do
    }
  }

  handleRetry = () => {
    this.setState((prev) => ({
      hasError: false,
      error: null,
      errorInfo: null,
      retryKey: prev.retryKey + 1,
    }));
  };

  render() {
    const { hasError, error, errorInfo, retryKey } = this.state;
    const { children, FallbackComponent } = this.props;

    if (hasError && error) {
      const Fallback = FallbackComponent || ErrorFallback;
      return (
        <Fallback
          error={error}
          errorInfo={errorInfo ?? undefined}
          onRetry={this.handleRetry}
        />
      );
    }

    return <React.Fragment key={retryKey}>{children}</React.Fragment>;
  }
}

export { GlobalErrorBoundary };
export { ErrorFallback };
export type { ErrorFallbackProps, GlobalErrorBoundaryProps };
