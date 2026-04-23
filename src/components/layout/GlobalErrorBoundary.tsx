import React from 'react';
import { Button, Typography, Space, theme } from 'antd';
import { ReloadOutlined, CopyOutlined, CheckOutlined } from '@ant-design/icons';

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
    const stack = errorInfo?.componentStack || error.stack || '';
    return `Error: ${error.message}\n\nStack Trace:\n${stack}`;
  }, [error, errorInfo]);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(errorDetails);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (e) {
      console.error('Failed to copy error details:', e);
    }
  };

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: '100vh',
        padding: '48px 24px',
        backgroundColor: token.colorBgContainer,
      }}
    >
      <div
        style={{
          maxWidth: 600,
          width: '100%',
          textAlign: 'center',
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
          <Text strong style={{ fontSize: 24, display: 'block', marginBottom: 8 }}>
            Something went wrong
          </Text>
          <Text type="secondary">
            The application encountered an unexpected error. Please try again or contact support.
          </Text>
        </div>

        <Space size="middle" style={{ marginBottom: 32 }}>
          <Button type="primary" icon={<ReloadOutlined />} onClick={onRetry} size="large">
            Retry
          </Button>
          <Button
            icon={copied ? <CheckOutlined /> : <CopyOutlined />}
            onClick={handleCopy}
            size="large"
          >
            {copied ? 'Copied!' : 'Copy Error'}
          </Button>
        </Space>

        <div
          style={{
            backgroundColor: token.colorBgElevated,
            border: `1px solid ${token.colorBorderSecondary}`,
            borderRadius: token.borderRadius,
            padding: 16,
            textAlign: 'left',
          }}
        >
          <Text type="secondary" style={{ fontSize: 12, display: 'block', marginBottom: 8 }}>
            Error Details
          </Text>
          <Paragraph
            code
            style={{
              margin: 0,
              fontSize: 12,
              maxHeight: 200,
              overflow: 'auto',
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-all',
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
    };
  }

  static getDerivedStateFromError(error: Error): Partial<GlobalErrorBoundaryState> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    this.setState({ errorInfo });
    
    // Log error to console in development
    if (import.meta.env.DEV) {
      console.error('GlobalErrorBoundary caught an error:', error, errorInfo);
    }

    // TODO: Send error to error reporting service
    // reportError(error, errorInfo);
  }

  handleRetry = () => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
    });
  };

  render() {
    const { hasError, error, errorInfo } = this.state;
    const { children, FallbackComponent } = this.props;

    if (hasError && error) {
      const Fallback = FallbackComponent || ErrorFallback;
      return <Fallback error={error} errorInfo={errorInfo ?? undefined} onRetry={this.handleRetry} />;
    }

    return children;
  }
}

export default GlobalErrorBoundary;
export { ErrorFallback };
export type { ErrorFallbackProps, GlobalErrorBoundaryProps };
