export function StreamingStyles() {
  return (
    <style>
      {`
        @keyframes axagent-think-spin {
          from {
            transform: rotate(0deg);
          }
          to {
            transform: rotate(360deg);
          }
        }
        @keyframes axagent-stream-dot-bounce {
          0%, 80%, 100% {
            transform: translateY(0);
            opacity: 0.45;
          }
          40% {
            transform: translateY(-3px);
            opacity: 1;
          }
        }
        .axagent-streaming-dots {
          display: inline-flex;
          align-items: center;
          gap: 4px;
          min-height: 16px;
        }
        .axagent-streaming-dots span {
          width: 6px;
          height: 6px;
          border-radius: 999px;
          background: currentColor;
          animation: axagent-stream-dot-bounce 1s ease-in-out infinite;
        }
        .axagent-streaming-dots span:nth-child(2) {
          animation-delay: 0.15s;
        }
        .axagent-streaming-dots span:nth-child(3) {
          animation-delay: 0.3s;
        }
      `}
    </style>
  );
}

export function BubbleStyleOverrides() {
  return (
    <style>
      {`
        .msg-row.user .msg-content {
          width: auto;
          max-width: 100%;
          margin-inline-start: auto;
        }
        .msg-row,
        .msg-body {
          min-width: 0;
          max-width: 100%;
        }
        .msg-footer {
          margin-block-start: 4px !important;
        }
        .msg-row.assistant .msg-body {
          width: 100%;
        }
        .msg-content {
          overflow: hidden;
          min-width: 0;
        }
        .msg-content .markstream-react {
          overflow: hidden;
          min-width: 0;
        }
        .msg-content .think-block,
        .msg-content .think-body,
        .msg-content .think-header {
          max-width: 100%;
          min-width: 0;
          overflow: hidden;
        }
        .msg-content .code-block-node,
        .msg-content .code-block-container {
          overflow-x: auto;
          max-width: 100%;
          min-width: 0 !important;
          width: 100%;
          box-sizing: border-box;
        }
        .bubble-compact .msg-row {
          margin-bottom: 4px;
        }
        .bubble-compact .msg-content {
          padding: 6px 10px;
        }
        .context-clear-bubble.msg-row {
          width: 100%;
          padding-inline-end: 0 !important;
          padding-inline-start: 0 !important;
        }
        .bubble-minimal .msg-content {
          background: transparent !important;
          box-shadow: none !important;
          border: none !important;
          padding: 4px 0;
        }
      `}
    </style>
  );
}
