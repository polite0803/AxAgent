#!/usr/bin/env python3
"""Fix antd deprecation warnings:
1. InputNumber addonAfter -> Space.Compact
2. message static calls -> App.useApp() pattern
"""
import re, os, sys

REPO = 'd:/OneManager/AxAgent'
sys.stdout.reconfigure(encoding='utf-8')

def read_file(path):
    with open(path, 'r', encoding='utf-8') as f:
        return f.read()

def write_file(path, content):
    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)

# ═══════════════════════════════════════════════
# FIX 1: InputNumber addonAfter -> Space.Compact
# ═══════════════════════════════════════════════

def fix_addon_after_in_file(filepath):
    content = read_file(filepath)
    original = content
    
    # Single-line self-closing: <InputNumber ... addonAfter="X" />
    content = re.sub(
        r'( *)(<InputNumber\b[^>]*?) addonAfter=("[^"]*")([^>]*?)/>',
        lambda m: _fix_single_addon(m, filepath),
        content
    )
    
    # Multi-line: addonAfter at end of attribute line, /> on next line(s)
    lines = content.split('\n')
    new_lines = []
    i = 0
    open_count = 0  # Track <Space.Compact nesting if already processed
    
    while i < len(lines):
        line = lines[i]
        
        # Skip lines that already have Space.Compact (already processed)
        if '<Space.Compact>' in line:
            new_lines.append(line)
            i += 1
            continue
        if '</Space.Compact>' in line:
            new_lines.append(line)
            i += 1
            continue
            
        # Check if this has addonAfter but wasn't caught by single-line regex
        if 'addonAfter=' in line and '<InputNumber' in line and '/>' not in line:
            new_lines = _fix_multiline_addon(lines, i, new_lines, filepath)
            # Find where we ended up
            i = len(new_lines)
            while i < len(lines) and i < len(new_lines):
                i += 1
            if i >= len(lines):
                break
            continue
            
        new_lines.append(line)
        i += 1
    
    result = '\n'.join(new_lines)
    
    if result != original:
        # Ensure Space is imported
        result = ensure_antd_import(result, 'Space')
        write_file(filepath, result)
        return True
    return False

def _fix_single_addon(m, filepath):
    indent = m.group(1)
    before = m.group(2)
    addon_val = m.group(3).strip('"')
    rest = m.group(4) or ''
    return f'{indent}<Space.Compact>\n{before}{rest}/>\n{indent}  <span>{addon_val}</span>\n{indent}</Space.Compact>'

def _fix_multiline_addon(lines, start_idx, new_lines, filepath):
    """Handle multi-line InputNumber with addonAfter. Find /> end."""
    indent = lines[start_idx][:len(lines[start_idx]) - len(lines[start_idx].lstrip())]
    has_close = any(lines[j].strip().endswith('/>') for j in range(start_idx, min(start_idx + 15, len(lines))))
    
    if not has_close:
        new_lines.append(lines[start_idx])
        return new_lines
    
    addon_val = None
    # Extract addon value
    m = re.search(r'addonAfter=("[^"]*"|{[^}]+})', lines[start_idx])
    if m:
        raw = m.group(1)
        addon_val = raw.strip('"').strip('{}').strip()
    
    new_lines.append(f'{indent}<Space.Compact>')
    
    # Find end of InputNumber tag
    end_idx = start_idx
    while end_idx < len(lines):
        stripped = lines[end_idx].strip()
        if stripped == '':
            end_idx += 1
            continue
        if stripped.endswith('/>') or stripped.endswith('>'):
            break
        end_idx += 1
    
    if end_idx >= len(lines):
        end_idx = start_idx + 2  # fallback
    
    for j in range(start_idx, end_idx + 1):
        modified = lines[j].replace(f' addonAfter={m.group(1)}', '') if m else lines[j]
        new_lines.append(modified)
    
    new_lines.append(f'{indent}  <span>{addon_val or "?"}</span>')
    new_lines.append(f'{indent}</Space.Compact>')
    return new_lines

def ensure_antd_import(content, component):
    """Add component to existing antd import if missing."""
    def repl(m):
        existing = m.group(1)
        if component not in existing:
            existing = existing.rstrip() + ', ' + component
        return f'{existing}}} from \'antd\''
    return re.sub(
        r'(import\s+\{[^}]+\}\s*from\s*[\'"]antd[\'"])',
        lambda m: _add_to_import(m, component),
        content
    )

def _add_to_import(m, component):
    """Add component to antd import statement."""
    text = m.group(1)
    # Extract the inner part
    inner = re.search(r'\{([^}]+)\}', text)
    if inner and component not in inner.group(1):
        new_inner = inner.group(1).rstrip() + ', ' + component
        return text.replace(inner.group(0), '{' + new_inner + '}')
    return text

# ═══════════════════════════════════════════════
# FIX 2: message static -> App.useApp()
# ═══════════════════════════════════════════════

def fix_message_in_file(filepath):
    """Migrate static message.X() to App.useApp() pattern."""
    content = read_file(filepath)
    original = content
    
    # Skip if already uses App.useApp or if it's a store/lib file
    # (stores can't use hooks)
    if 'useApp' in content:
        return False
    
    # Check if file uses message static calls
    if not re.search(r'\bmessage\.(success|error|info|warning|loading)\s*\(', content):
        return False
    
    # Check if file is a React component (has export function/const or React.FC)
    is_component = bool(re.search(
        r'export\s+(default\s+)?function\s+\w+|export\s+const\s+\w+\s*[=:]\s*(React\.)?FC\b',
        content
    ))
    
    if not is_component:
        # Could still be a component file without those patterns
        # Check for JSX and function/const at the top level
        has_jsx = bool(re.search(r'<[A-Z]\w+[^>]*>', content))
        has_fn = bool(re.search(r'(function|const)\s+\w+\s*[=(]', content))
        if not (has_jsx and has_fn):
            return False  # Probably a store or lib file
    
    # Update the antd import
    content = _update_message_import(content)
    if content is None:
        return False
    
    # Add const { message } = App.useApp() in component body
    content = _add_useapp_hook(content, filepath)
    if content is None:
        return False
    
    if content != original:
        write_file(filepath, content)
        return True
    return False

def _update_message_import(content):
    """Replace import { message } from 'antd' with import { App } from 'antd'.
    For combined imports like import { message, Button }, change to import { App, Button }."""
    
    # Pattern: import { ...message... } from 'antd'
    def replace_import(m):
        full = m.group(0)
        inner = m.group(1)
        
        # Check if 'message' is the only thing imported or part of a list
        new_inner = inner.replace('message', '').strip()
        # Clean up double commas, trailing/leading commas
        new_inner = re.sub(r',\s*,', ',', new_inner)
        new_inner = re.sub(r'^\s*,\s*', '', new_inner)
        new_inner = re.sub(r'\s*,\s*$', '', new_inner)
        
        # Add App if not present
        if 'App' not in new_inner:
            if new_inner:
                new_inner += ', App'
            else:
                new_inner = 'App'
        
        return f'{{{new_inner}}} from \'antd\''
    
    new_content = re.sub(
        r'\{([^}]*message[^}]*)\}\s+from\s+[\'"]antd[\'"]',
        replace_import,
        content
    )
    
    if new_content == content:
        # Maybe it uses default import: import message from 'antd'
        if re.search(r'^import\s+message\s+from\s+[\'"]antd[\'"]', content, re.MULTILINE):
            new_content = re.sub(
                r'^import\s+message\s+from\s+[\'"]antd[\'"]',
                r"import { App } from 'antd'",
                content,
                flags=re.MULTILINE
            )
        else:
            print(f'  WARN: Cannot update import in {filepath}')
            return None
    
    return new_content

def _add_useapp_hook(content, filepath):
    """Add const { message } = App.useApp() after component function opening brace."""
    # Match the first function/arrow component body
    # Try patterns in order:
    
    # Pattern 1: export function Foo(args) {
    # Pattern 2: export const Foo: React.FC<Props> = (args) => {
    # Pattern 3: const Foo = (args) => {
    # Pattern 4: function Foo(args) {
    
    patterns = [
        # Named function export
        (r'(export\s+)?(async\s+)?function\s+\w+\s*\([^)]*\)\s*\{', 0),
        # Arrow function with export
        (r'(export\s+)?(default\s+)?const\s+\w+\s*[=:]\s*(React\.)?FC\s*<[^>]*>\s*=\s*\([^)]*\)\s*=>\s*\{', 0),
        (r'(export\s+)?(default\s+)?const\s+\w+\s*=\s*\([^)]*\)\s*=>\s*\{', 0),
    ]
    
    hook_code = '  const { message } = App.useApp();'
    
    for pattern, _ in patterns:
        # Check if hook already exists
        if 'message = App.useApp()' in content or 'useMessage' in content:
            return content
            
        m = re.search(pattern, content)
        if m:
            match_end = m.end()
            # Insert hook after the opening brace
            insert_pos = match_end
            # Find the actual newline after the brace
            brace_pos = content.rfind('{', 0, match_end)
            line_end = content.find('\n', brace_pos)
            if line_end > 0:
                insert_pos = line_end + 1
                # Check indentation of next line
                rest = content[insert_pos:]
                indent_match = re.match(r'(\s*)', rest)
                indent = '  '
                new_content = content[:insert_pos] + indent + hook_code + '\n' + content[insert_pos:]
                return new_content
    
    print(f'  WARN: Cannot find component start in {filepath}')
    return None

# ═══════════════════════════════════════════════
# MAIN
# ═══════════════════════════════════════════════

# ADDONAFTER FILES
addon_files = [
    'src/components/settings/AboutPage.tsx',
    'src/components/settings/BackupCenter.tsx',
    'src/components/settings/McpServerSettings.tsx',
    'src/components/settings/ProviderDetail.tsx',
    'src/components/settings/SearchProviderSettings.tsx',
    'src/components/settings/WebDavSync.tsx',
    'src/components/workflow/Panels/PropertyPanels/DatabaseQueryPropertyPanel.tsx',
    'src/components/workflow/Panels/PropertyPanels/HttpRequestPropertyPanel.tsx',
]

print('=== Fix 1: InputNumber addonAfter → Space.Compact ===')
fixed = 0
for f in addon_files:
    fp = os.path.join(REPO, f)
    if os.path.exists(fp):
        if fix_addon_after_in_file(fp):
            print(f'  ✅ {f}')
            fixed += 1
        else:
            print(f'  ➖ {f} (no change)')
    else:
        print(f'  ❌ {f} (not found)')
print(f'Fixed: {fixed}/{len(addon_files)}')

# MESSAGE STATIC FILES
# Files with static message usage that don't have useApp yet
message_files_raw = [
    'src/components/chat/ArtifactPanel.tsx',
    'src/components/chat/BrowserAutomationPanel.tsx',
    'src/components/chat/CategoryManagerModal.tsx',
    'src/components/chat/ComputerControlPanel.tsx',
    'src/components/chat/ImageGenPanel.tsx',
    'src/components/chat/PluginMarketplace.tsx',
    'src/components/chat/SkillCreateEditModal.tsx',
    'src/components/chat/SkillProposalPanel.tsx',
    'src/components/chat/SteerInput.tsx',
    'src/components/chat/TaskPanel.tsx',
    'src/components/chat/TeammatePanel.tsx',
    'src/components/chat/WorkflowProgressPanel.tsx',
    'src/components/common/CopyButton.tsx',
    'src/components/common/PasteButton.tsx',
    'src/components/gateway/GatewayKeys.tsx',
    'src/components/gateway/GatewayOverview.tsx',
    'src/components/gateway/GatewaySettings.tsx',
    'src/components/llm-wiki/SyncStatus.tsx',
    'src/components/settings/AcpSettings.tsx',
    'src/components/settings/AgentProfileManager.tsx',
    'src/components/settings/CloudWorkspaceSelector.tsx',
    'src/components/settings/DashboardPluginsSettings.tsx',
    'src/components/settings/DynamicPagesSettings.tsx',
    'src/components/settings/McpServerSettings.tsx',
    'src/components/settings/SkillsHubSettings.tsx',
    'src/components/settings/ThemeManager.tsx',
    'src/components/settings/ToolSemanticCheck.tsx',
    'src/components/settings/WebhookSettings.tsx',
    'src/components/skill/FrontendEditorModal.tsx',
    'src/components/wiki/IngestPanel.tsx',
    'src/components/wiki/LintReport.tsx',
    'src/components/wiki/VersionHistoryPanel.tsx',
    'src/components/wiki/WikiDetailPanel.tsx',
    'src/components/workflow/Panels/PropertyPanels/AgentPropertyPanel.tsx',
    'src/components/workflow/SemanticCheckModal.tsx',
    'src/components/workflow/Templates/ImportExportModal.tsx',
    'src/components/workflow/Templates/TemplateList.tsx',
    'src/components/workflow/Templates/VersionHistoryModal.tsx',
    'src/components/workflow/WorkflowEditor.tsx',
    'src/pages/DevTools/BenchmarkRunner.tsx',
    'src/pages/DynamicUIManagerPage.tsx',
    'src/pages/IngestPage.tsx',
    'src/pages/SettingsPage.tsx',
    'src/pages/TerminalPage.tsx',
    'src/pages/WikiEditorPage.tsx',
    'src/pages/WikiGraphPage.tsx',
    'src/pages/WorkflowMarketplace.tsx',
]

# Filter out files that already have useApp
import subprocess
result = subprocess.run(
    ['grep', '-rln', 'useApp\\|App\\.useApp', 'src/', '--include=*.tsx', '--include=*.ts'],
    cwd=REPO, capture_output=True, text=True
)
already_done = set(result.stdout.strip().split('\n')) if result.stdout.strip() else set()

message_files = []
for f in message_files_raw:
    fp = os.path.join(REPO, f)
    if os.path.exists(fp):
        if fp in already_done or os.path.basename(fp).replace('\\', '/').replace('/', '_') in already_done:
            continue
        # Check if file actually has message static calls
        content = read_file(fp)
        if re.search(r'\bmessage\.(success|error|info|warning|loading)\s*\(', content):
            message_files.append(f)
            continue
    else:
        print(f'  ⚠️  Not found: {f}')

print(f'\n=== Fix 2: message static → App.useApp() ===')
print(f'Files to process: {len(message_files)}')

fixed_msg = 0
warn_files = []
for f in message_files:
    fp = os.path.join(REPO, f)
    try:
        if fix_message_in_file(fp):
            print(f'  ✅ {f}')
            fixed_msg += 1
        else:
            print(f'  ➖ {f} (no change/skip)')
    except Exception as e:
        print(f'  ❌ {f}: {e}')
        warn_files.append(f)

print(f'\nFixed message: {fixed_msg}/{len(message_files)}')
if warn_files:
    print(f'Skipped due to errors: {len(warn_files)}')
print('\nDone!')
