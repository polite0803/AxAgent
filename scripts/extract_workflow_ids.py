"""提取 generated.rs 中所有的工作流 ID。"""
import re

with open(r'd:\OneManager\AxInvest\src-tauri\crates\analysis-engine\src\opc\domain\generated.rs', 'r', encoding='utf-8') as f:
    content = f.read()

ids = re.findall(r'DomainWorkflowDef::new\("(wf-[^"]+)"', content)
print('\n'.join(sorted(ids)))
print(f"\n总计: {len(ids)} 个工作流")
