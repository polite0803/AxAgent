const fs = require('fs');
const path = require('path');

const srcDir = 'd:/OneManager/AxInvest/src';

function findFiles(dir) {
  const results = [];
  const items = fs.readdirSync(dir, { withFileTypes: true });
  for (const item of items) {
    const fullPath = path.join(dir, item.name);
    if (item.isDirectory() && !item.name.startsWith('.') && item.name !== 'node_modules') {
      results.push(...findFiles(fullPath));
    } else if (/\.(tsx|ts)$/.test(item.name)) {
      try {
        const content = fs.readFileSync(fullPath, 'utf-8');
        if (content.includes('Input.TextArea')) {
          results.push(fullPath);
        }
      } catch(e) {}
    }
  }
  return results;
}

function transformFile(filePath) {
  let content = fs.readFileSync(filePath, 'utf-8');
  const original = content;

  // Step 1: Replace Input.TextArea with TextArea
  content = content.replace(/<Input\.TextArea/g, '<TextArea');
  content = content.replace(/<\/Input\.TextArea>/g, '</TextArea>');

  // Step 2: Add TextArea to antd imports if not already there
  const hasInput = /import\s+\{[^}]*\bInput\b[^}]*\}\s+from\s+['"]antd['"]/.test(original);
  const hasTextArea = /import\s+\{[^}]*\bTextArea\b[^}]*\}\s+from\s+['"]antd['"]/.test(content);

  if (hasInput && !hasTextArea) {
    content = content.replace(
      /(import\s+\{[^}]*\bInput\b[^}]*\}\s+from\s+['"]antd['"])/,
      (match) => match.replace('Input', 'Input, TextArea')
    );
  }

  if (content !== original) {
    fs.writeFileSync(filePath, content, 'utf-8');
    return true;
  }
  return false;
}

const files = findFiles(srcDir);
console.log('Files with Input.TextArea:', files.length);
let modified = 0;
for (const file of files) {
  try {
    if (transformFile(file)) {
      modified++;
      console.log('Modified:', path.relative('d:/OneManager/AxInvest', file));
    }
  } catch(e) {
    console.error('Error:', file, e.message);
  }
}
console.log('\nTotal modified:', modified, '/', files.length);
