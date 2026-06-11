// SPDX-License-Identifier: AGPL-3.0-only

export interface ParsedHtml {
  html: string;
  css: string;
  js: string;
  full: string;
}

export function parseHtmlContent(content: string): ParsedHtml {
  const parser = new DOMParser();
  const doc = parser.parseFromString(content, "text/html");

  const css = Array.from(doc.querySelectorAll("style"))
    .map((el) => el.textContent || "")
    .join("\n");

  const js = Array.from(doc.querySelectorAll("script:not([src])"))
    .map((el) => el.textContent || "")
    .join("\n");

  doc.querySelectorAll("style, script:not([src])").forEach((el) => el.remove());
  const html = doc.body?.innerHTML || content;

  return { html, css, js, full: content };
}

let _sandboxEnforced = false;

export function enableHtmlCompose() {
  _sandboxEnforced = true;
}

export function composeHtml(parts: Partial<ParsedHtml>): string {
  if (!_sandboxEnforced) {
    return "<!-- HTML composition disabled: call enableHtmlCompose() from sandbox context first -->";
  }
  // ⚠️ SECURITY WARNING: The output of this function contains raw HTML, CSS, and JS
  // which may include user-generated or AI-generated content. This output MUST only be
  // rendered inside a sandboxed iframe (sandbox="" attribute) to prevent XSS attacks,
  // data exfiltration, and other security risks. Never inject this output directly into
  // the host page's DOM or use it with dangerouslySetInnerHTML outside an iframe.
  const html = parts.html || "";
  const css = parts.css || "";
  const js = parts.js || "";

  return `<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; padding: 16px; }
${css}
</style>
</head>
<body>
${html}
<script>
try { ${js} } catch(e) { var _pre = document.createElement('pre'); _pre.style.color = 'red'; _pre.textContent = 'Error: ' + e.message; document.body.appendChild(_pre); }
</script>
</body>
</html>`;
}

export function isChartOption(content: string): boolean {
  try {
    const obj = JSON.parse(content);
    return !!(
      obj.series
      || obj.xAxis
      || obj.yAxis
      || obj.polar
      || obj.radiusAxis
    );
  } catch {
    return false;
  }
}
