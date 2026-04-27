import { Button, Result } from "antd";
import React from "react";

interface ErrorBoundaryState {
  hasError: boolean;
  error?: Error;
}

interface ErrorBoundaryProps {
  children: React.ReactNode;
  fallback?: React.ReactNode;
  onReset?: () => void;
}

export class ErrorBoundary extends React.Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error("ErrorBoundary caught an error:", error, errorInfo);
  }

  handleReset = () => {
    this.setState({ hasError: false, error: undefined });
    this.props.onReset?.();
  };

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }
      return (
        <Result
          status="error"
          title="Something went wrong"
          subTitle={this.state.error?.message || "An unexpected error occurred"}
          extra={
            <Button type="primary" onClick={this.handleReset}>
              Try Again
            </Button>
          }
        />
      );
    }

    return this.props.children;
  }
}

interface PageErrorBoundaryProps {
  children: React.ReactNode;
  title?: string;
}

export function PageErrorBoundary({ children, title = "Page Error" }: PageErrorBoundaryProps) {
  return (
    <ErrorBoundary
      fallback={
        <div className="flex items-center justify-center h-full">
          <Result
            status="error"
            title={title}
            subTitle="This page encountered an error. Please try refreshing."
            extra={
              <Button type="primary" onClick={() => window.location.reload()}>
                Refresh Page
              </Button>
            }
          />
        </div>
      }
    >
      {children}
    </ErrorBoundary>
  );
}
