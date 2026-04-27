import { Alert, Button, Card, Progress, Statistic, Timeline, Tooltip, Typography } from "antd";
import {
  AlertCircle,
  CheckCircle,
  Clock,
  RefreshCw,
  SkipForward,
  AlertTriangle,
  XCircle,
  Info,
  RotateCcw,
} from "lucide-react";
import { useEffect, useState } from "react";

const { Text } = Typography;

interface RecoveryAttempt {
  attempt_number: number;
  error: string;
  strategy: string;
  delay_ms?: number;
  success: boolean;
  message?: string;
}

interface RecoveryResult {
  success: boolean;
  recovered: boolean;
  strategy_used: string;
  attempts_made: number;
  final_error?: string;
  recovery_time_ms: number;
}

interface ErrorRecoveryPanelProps {
  error?: string;
  errorType?: "transient" | "recoverable" | "unrecoverable" | "unknown";
  onRecoveryStart?: () => void;
  onRecoveryComplete?: (result: RecoveryResult) => void;
  isRecovering?: boolean;
  initialAttempts?: RecoveryAttempt[];
  initialResult?: RecoveryResult | null;
}

const errorTypeColors: Record<string, string> = {
  transient: "blue",
  recoverable: "orange",
  unrecoverable: "red",
  unknown: "gray",
};

const errorTypeDescriptions: Record<string, string> = {
  transient: "Temporary error - retry may resolve",
  recoverable: "Recoverable error - can be fixed with adjustment",
  unrecoverable: "Unrecoverable error - should fail",
  unknown: "Unknown error type",
};

const strategyIcons: Record<string, React.ReactNode> = {
  Retry: <RefreshCw size={14} />,
  AdjustAndRetry: <RotateCcw size={14} />,
  Fallback: <SkipForward size={14} />,
  SkipTask: <SkipForward size={14} />,
  Fail: <XCircle size={14} />,
};

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}

function ErrorTypeTag({ type }: { type: string }) {
  return (
    <Tooltip title={errorTypeDescriptions[type] || "Unknown"}>
      <span>
        <AlertCircle size={14} className={`mr-1 text-${errorTypeColors[type]}-500`} />
        <span className="capitalize">{type}</span>
      </span>
    </Tooltip>
  );
}

function AttemptItem({ attempt }: { attempt: RecoveryAttempt }) {
  return (
    <Timeline.Item
      dot={
        attempt.success ? (
          <CheckCircle size={16} className="text-green-500" />
        ) : (
          <XCircle size={16} className="text-red-500" />
        )
      }
    >
      <div className="flex items-start justify-between">
        <div>
          <Text strong>Attempt {attempt.attempt_number}</Text>
          <div className="flex items-center gap-2 mt-1">
            <span className="flex items-center gap-1">
              {strategyIcons[attempt.strategy] || <Info size={14} />}
              <Text type="secondary" className="text-sm">
                {attempt.strategy}
              </Text>
            </span>
            {attempt.delay_ms && (
              <span className="flex items-center gap-1 text-gray-400">
                <Clock size={12} />
                <Text type="secondary" className="text-xs">
                  {formatDuration(attempt.delay_ms)}
                </Text>
              </span>
            )}
          </div>
        </div>
        {attempt.success && attempt.message && (
          <CheckCircle size={14} className="text-green-500" />
        )}
      </div>

      {!attempt.success && attempt.error && (
        <Alert
          type="error"
          message={attempt.error}
          className="mt-2"
          style={{ fontSize: "12px" }}
        />
      )}

      {attempt.success && attempt.message && (
        <Alert
          type="success"
          message={attempt.message}
          className="mt-2"
          style={{ fontSize: "12px" }}
        />
      )}
    </Timeline.Item>
  );
}

export function ErrorRecoveryPanel({
  error,
  errorType,
  onRecoveryStart,
  onRecoveryComplete,
  isRecovering: initialIsRecovering = false,
  initialAttempts = [],
  initialResult = null,
}: ErrorRecoveryPanelProps) {
  const [isRecovering, setIsRecovering] = useState(initialIsRecovering);
  const [attempts, setAttempts] = useState<RecoveryAttempt[]>(initialAttempts);
  const [result, setResult] = useState<RecoveryResult | null>(initialResult);
  const [currentAttempt, setCurrentAttempt] = useState(0);

  useEffect(() => {
    if (isRecovering && currentAttempt > 0) {
      const timer = setTimeout(() => {
        const newAttempt: RecoveryAttempt = {
          attempt_number: currentAttempt,
          error: error || "Unknown error",
          strategy: "Retry",
          delay_ms: Math.pow(2, currentAttempt - 1) * 1000,
          success: currentAttempt >= 3,
          message: currentAttempt >= 3 ? "Recovery successful" : undefined,
        };

        setAttempts((prev) => [...prev, newAttempt]);

        if (newAttempt.success) {
          setIsRecovering(false);
          const recoveryResult: RecoveryResult = {
            success: true,
            recovered: true,
            strategy_used: "Retry",
            attempts_made: currentAttempt,
            recovery_time_ms: 5000,
          };
          setResult(recoveryResult);
          onRecoveryComplete?.(recoveryResult);
        } else if (currentAttempt >= 5) {
          setIsRecovering(false);
          const recoveryResult: RecoveryResult = {
            success: false,
            recovered: false,
            strategy_used: "Retry",
            attempts_made: currentAttempt,
            final_error: "Max retry attempts reached",
            recovery_time_ms: 5000,
          };
          setResult(recoveryResult);
          onRecoveryComplete?.(recoveryResult);
        } else {
          setCurrentAttempt((prev) => prev + 1);
        }
      }, 1000);

      return () => clearTimeout(timer);
    }
  }, [isRecovering, currentAttempt, error, onRecoveryComplete]);

  const handleStartRecovery = () => {
    setIsRecovering(true);
    setAttempts([]);
    setResult(null);
    setCurrentAttempt(1);
    onRecoveryStart?.();
  };

  const handleReset = () => {
    setIsRecovering(false);
    setAttempts([]);
    setResult(null);
    setCurrentAttempt(0);
  };

  if (!error && !result) {
    return (
      <Card size="small" className="error-recovery-panel">
        <div className="flex items-center justify-center h-32 text-gray-400">
          <AlertCircle size={24} className="mr-2" />
          <Text type="secondary">No error to recover from</Text>
        </div>
      </Card>
    );
  }

  const showInitialError = error && !isRecovering && attempts.length === 0;

  return (
    <Card
      size="small"
      className="error-recovery-panel"
      title={
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <AlertTriangle size={16} className="text-orange-500" />
            <span>Error Recovery</span>
            {errorType && <ErrorTypeTag type={errorType} />}
          </div>
          {isRecovering && (
            <RefreshCw size={14} className="animate-spin text-blue-500" />
          )}
        </div>
      }
      extra={
        !isRecovering && !result && (
          <Button
            type="primary"
            size="small"
            icon={<RefreshCw size={14} />}
            onClick={handleStartRecovery}
          >
            Recover
          </Button>
        )
      }
    >
      {showInitialError && (
        <Alert
          type="error"
          message="Error Occurred"
          description={
            <div>
              <p className="mb-2">{error}</p>
              <Text type="secondary" className="text-sm">
                {errorTypeDescriptions[errorType || "unknown"]}
              </Text>
            </div>
          }
          className="mb-4"
        />
      )}

      {isRecovering && attempts.length === 0 && (
        <Alert
          type="info"
          message="Recovery in Progress"
          description={
            <div>
              <p>Attempting to recover from error...</p>
              <Progress percent={currentAttempt * 20} size="small" />
            </div>
          }
          className="mb-4"
        />
      )}

      {attempts.length > 0 && (
        <div className="mb-4">
          <div className="flex items-center justify-between mb-2">
            <Text strong>Recovery Attempts</Text>
            <Text type="secondary" className="text-sm">
              {attempts.filter((a) => a.success).length} / {attempts.length} succeeded
            </Text>
          </div>

          <Timeline className="mt-4">
            {attempts.map((attempt) => (
              <AttemptItem key={attempt.attempt_number} attempt={attempt} />
            ))}
          </Timeline>
        </div>
      )}

      {result && (
        <div className="border-t pt-4">
          <div className="grid grid-cols-3 gap-4 mb-4">
            <Statistic
              title="Status"
              value={result.success ? "Success" : "Failed"}
              valueStyle={{
                color: result.success ? "#3f8600" : "#cf1322",
              }}
              prefix={
                result.success ? (
                  <CheckCircle size={16} />
                ) : (
                  <XCircle size={16} />
                )
              }
            />
            <Statistic
              title="Attempts"
              value={result.attempts_made}
              suffix={`/ ${result.strategy_used}`}
            />
            <Statistic
              title="Time"
              value={formatDuration(result.recovery_time_ms)}
              suffix={result.recovered ? "" : ""}
            />
          </div>

          {result.final_error && (
            <Alert
              type={result.success ? "success" : "error"}
              message={result.final_error}
              className="mb-4"
            />
          )}

          {!isRecovering && (
            <Button
              type="default"
              size="small"
              icon={<RotateCcw size={14} />}
              onClick={handleReset}
            >
              Reset
            </Button>
          )}
        </div>
      )}

      {isRecovering && (
        <div className="flex items-center justify-between mt-4">
          <Text type="secondary" className="text-sm">
            Attempting recovery...
          </Text>
          <Progress
            type="circle"
            percent={Math.min(currentAttempt * 20, 100)}
            size={40}
            strokeColor={result?.success ? "#3f8600" : "#1890ff"}
          />
        </div>
      )}
    </Card>
  );
}

export function useErrorRecovery() {
  const [error, setError] = useState<string | null>(null);
  const [errorType, setErrorType] = useState<string | null>(null);
  const [isRecovering, setIsRecovering] = useState(false);
  const [result, setResult] = useState<RecoveryResult | null>(null);
  const [attempts, setAttempts] = useState<RecoveryAttempt[]>([]);

  const startRecovery = (err: string, type?: string) => {
    setError(err);
    setErrorType(type || null);
    setIsRecovering(true);
    setResult(null);
    setAttempts([]);
  };

  const reset = () => {
    setError(null);
    setErrorType(null);
    setIsRecovering(false);
    setResult(null);
    setAttempts([]);
  };

  return {
    error,
    errorType,
    isRecovering,
    result,
    attempts,
    startRecovery,
    reset,
    RecoveryPanel: ErrorRecoveryPanel,
  };
}
