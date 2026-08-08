import yaml, subprocess

result = subprocess.run(['git', 'show', 'HEAD:config/opc/domains/engineering/workflows/wf-eng-refactor.yaml'], capture_output=True, text=True, cwd='d:/OneManager/AxInvest')
data = yaml.safe_load(result.stdout)
steps = data.get('steps', [])
user_input_steps = [s for s in steps if s.get('user_input')]
print(f'Total steps: {len(steps)}')
print(f'Steps with user_input: {len(user_input_steps)}')
if user_input_steps:
    s = user_input_steps[0]
    print(f'Sample user_input from step "{s["id"]}":')
    ui = s['user_input']
    print(f'  enabled: {ui.get("enabled")}')
    print(f'  mode: {ui.get("mode")}')
    print(f'  prompt: {ui.get("prompt", "")[:80]}...')
    fields = ui.get('fields', [])
    print(f'  fields count: {len(fields)}')
    if fields:
        f = fields[0]
        print(f'  first field: {f.get("name")} ({f.get("type")})')

# 检查所有领域的 user_input 使用情况
import os
all_yaml_files = []
for root, dirs, files in os.walk('d:/OneManager/AxInvest/config/opc/domains'):
    for f in files:
        if f.endswith('.yaml') and 'workflows' in root:
            all_yaml_files.append(os.path.join(root, f))

# 由于我们删除了文件，改用 git
result = subprocess.run(['git', 'ls-tree', '-r', '--name-only', 'HEAD', 'config/opc/domains'], capture_output=True, text=True, cwd='d:/OneManager/AxInvest')
all_files = [f for f in result.stdout.strip().split('\n') if '/workflows/' in f and f.endswith('.yaml')]

total_ui = 0
total_steps = 0
for f in all_files:
    r = subprocess.run(['git', 'show', f'HEAD:{f}'], capture_output=True, text=True, cwd='d:/OneManager/AxInvest')
    try:
        data = yaml.safe_load(r.stdout)
        steps = data.get('steps', [])
        total_steps += len(steps)
        ui_count = sum(1 for s in steps if s.get('user_input'))
        if ui_count > 0:
            total_ui += ui_count
            wf_id = data.get('id', f)
            print(f'  {wf_id}: {ui_count} user_input steps')
    except:
        pass

print(f'\nTotal steps across all workflows: {total_steps}')
print(f'Total user_input steps: {total_ui}')