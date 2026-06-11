// SPDX-License-Identifier: AGPL-3.0-only

import { Button, Result } from "antd";
import i18next from "i18next";
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

export class ErrorBoundary extends React.Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
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
          title={i18next.t("errorBoundary.title")}
          subTitle={this.state.error?.message
            || i18next.t("errorBoundary.unexpectedError")}
          extra={
            <Button type="primary" onClick={this.handleReset}>
              {i18next.t("errorBoundary.tryAgain")}
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

export function PageErrorBoundary({
  children,
  title = "Page Error",
}: PageErrorBoundaryProps) {
  return (
    <ErrorBoundary
      fallback={
        <div className="flex items-center justify-center h-full">
          <Result
            status="error"
            title={title}
            subTitle={i18next.t("errorBoundary.pageError")}
            extra={
              <Button type="primary" onClick={() => window.location.reload()}>
                {i18next.t("errorBoundary.refreshPage")}
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
