import { memo, useEffect, useRef } from "react";

interface ChartPreviewProps {
  option: Record<string, unknown>;
  width?: number | string;
  height?: number | string;
  theme?: "light" | "dark";
}

export const ChartPreview = memo(function ChartPreview({
  option,
  width,
  height,
  theme = "light",
}: ChartPreviewProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null);

  useEffect(() => {
    if (iframeRef.current) {
      const bgColor = theme === "dark" ? "#1e1e1e" : "#ffffff";
      const textColor = theme === "dark" ? "#ccc" : "#333";

      iframeRef.current.srcdoc = `<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<script src="https://cdn.jsdelivr.net/npm/echarts@5/dist/echarts.min.js"><\/script>
<style>
  body { margin: 0; background: ${bgColor}; }
  #chart { width: 100%; height: 100%; }
</style>
</head>
<body>
<div id="chart"></div>
<script>
var chart = echarts.init(document.getElementById('chart'), null, { renderer: 'canvas' });
var option = ${JSON.stringify(option)};
option.color = option.color || ['#5470c6','#91cc75','#fac858','#ee6666','#73c0de','#3ba272'];
if (!option.textStyle) option.textStyle = { color: '${textColor}' };
chart.setOption(option);
window.addEventListener('resize', function() { chart.resize(); });
<\/script>
</body>
</html>`;
    }
  }, [option, theme]);

  return (
    <iframe
      ref={iframeRef}
      sandbox="allow-scripts"
      style={{
        width: width || "100%",
        height: height || 400,
        border: "none",
        borderRadius: 8,
      }}
    />
  );
});
