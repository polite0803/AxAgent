import { memo, useCallback, useEffect, useRef } from "react";

interface ReactPreviewProps {
  code: string;
  css?: string;
  onError?: (error: string) => void;
}

export const ReactPreview = memo(function ReactPreview({
  code,
  css,
  onError,
}: ReactPreviewProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null);

  const buildSrcDoc = useCallback(() => {
    return `<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-eval' https://unpkg.com; style-src 'unsafe-inline';">
<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; padding: 16px; }
${css || ""}
</style>
<script src="https://unpkg.com/react@18/umd/react.development.js"></script>
<script src="https://unpkg.com/react-dom@18/umd/react-dom.development.js"></script>
<!-- TODO: Add SRI integrity hashes for all unpkg scripts -->
<script src="https://unpkg.com/@babel/standalone@7.25.0/babel.min.js"></script>
</head>
<body>
<div id="root"></div>
<script>
var _lastPostMessage = 0;
window.onerror = function(msg, src, line, col, err) {
  var now = Date.now();
  if (now - _lastPostMessage < 500) { return; }
  _lastPostMessage = now;
  window.parent.postMessage({ type: 'react-preview-error', message: String(msg) }, window.location.origin);
};
try {
  var transformed = Babel.transform(${JSON.stringify(code)}, {
    presets: ['react'],
    filename: 'component.tsx'
  });
  var fn = new Function('React', 'ReactDOM', transformed.code);
  fn(React, ReactDOM);
} catch(e) {
  document.getElementById('root').innerHTML = '<pre style="color:red;padding:16px">' + e.message + '</pre>';
  var now2 = Date.now();
  if (now2 - _lastPostMessage >= 500) {
    _lastPostMessage = now2;
    window.parent.postMessage({ type: 'react-preview-error', message: e.message }, window.location.origin);
  }
}
</script>
</body>
</html>`;
  }, [code, css]);

  useEffect(() => {
    if (iframeRef.current) {
      iframeRef.current.srcdoc = buildSrcDoc();
    }
  }, [buildSrcDoc]);

  useEffect(() => {
    const handler = (event: MessageEvent) => {
      // 校验消息来源为当前 iframe，防止其他窗口/iframe 伪造消息
      if (event.source !== iframeRef.current?.contentWindow) {
        return;
      }
      if (event.data?.type === "react-preview-error") {
        onError?.(event.data.message);
      }
    };
    window.addEventListener("message", handler);
    return () => window.removeEventListener("message", handler);
  }, [onError]);

  return (
    <iframe
      ref={iframeRef}
      sandbox="allow-scripts"
      title="React Preview"
      style={{
        width: "100%",
        height: "100%",
        border: "none",
        background: "#fff",
        borderRadius: 8,
      }}
    />
  );
});
