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
        .ant-bubble-end .ant-bubble-content {
          width: auto;
          max-width: 100%;
          margin-inline-start: auto;
        }
        .ant-bubble,
        .ant-bubble-content-wrapper,
        .ant-bubble-body {
          min-width: 0;
          max-width: 100%;
        }
        .ant-bubble-footer {
          margin-block-start: 4px !important;
        }
        .ant-bubble-start .ant-bubble-body {
          width: 100%;
        }
        .ant-bubble-content {
          overflow: hidden;
          min-width: 0;
        }
        .ant-bubble-content .markstream-react {
          overflow: hidden;
          min-width: 0;
        }
        .ant-bubble-content .ant-think,
        .ant-bubble-content .ant-think-content,
        .ant-bubble-content .ant-think-description {
          max-width: 100%;
          min-width: 0;
          overflow: hidden;
        }
        .ant-bubble-content .code-block-node,
        .ant-bubble-content .code-block-container {
          overflow-x: auto;
          max-width: 100%;
          min-width: 0 !important;
          width: 100%;
          box-sizing: border-box;
        }
        .bubble-compact .ant-bubble {
          margin-bottom: 4px;
        }
        .bubble-compact .ant-bubble-content {
          padding: 6px 10px;
        }
        .context-clear-bubble.ant-bubble {
          width: 100%;
          padding-inline-end: 0 !important;
          padding-inline-start: 0 !important;
        }
        .context-clear-bubble .ant-bubble-content-wrapper {
          flex: 1;
        }
        .bubble-minimal .ant-bubble-content {
          background: transparent !important;
          box-shadow: none !important;
          border: none !important;
          padding: 4px 0;
        }
      `}
    </style>
  );
}
