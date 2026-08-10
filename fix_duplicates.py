import re

with open(r'd:\OneManager\AxInvest\src\lib\workflowLayout.ts', 'r', encoding='utf-8') as f:
    lines = f.readlines()

# 1. 删除第一个重复的 missing_end_node key（第77-78行，索引76-77）
# 先找到所有包含 missing_end_node 的行
missing_end_node_lines = []
for i, line in enumerate(lines):
    if '"workflow.layout.validate.missing_end_node"' in line:
        missing_end_node_lines.append(i)

print(f"Found 'missing_end_node' keys at lines: {[x+1 for x in missing_end_node_lines]}")

# 如果有重复的key，删除第一个（保留第二个带参数的版本）
if len(missing_end_node_lines) >= 2:
    first_key_line = missing_end_node_lines[0]
    # 删除这个key和它的值（通常是下一行）
    # 检查下一行是否是值
    if first_key_line + 1 < len(lines) and ':' not in lines[first_key_line + 1]:
        # 下一行是值，删除两行
        del lines[first_key_line:first_key_line + 2]
        print(f"Deleted lines {first_key_line + 1}-{first_key_line + 2}")
    else:
        # 值在同一行或其他情况
        del lines[first_key_line]
        print(f"Deleted line {first_key_line + 1}")

# 2. 删除第一个重复的 hasEndNode 变量声明
has_end_node_lines = []
for i, line in enumerate(lines):
    if 'const hasEndNode = nodes.some' in line:
        has_end_node_lines.append(i)

print(f"Found 'hasEndNode' declarations at lines: {[x+1 for x in has_end_node_lines]}")

# 如果有重复，删除第一个（保留第二个更完善的版本）
if len(has_end_node_lines) >= 2:
    first_decl_line = has_end_node_lines[0]
    # 删除从声明开始到检查结束的所有行
    # 查找这个检查块的结束（通过找到下一个注释块或逻辑块）
    block_start = first_decl_line
    # 向下查找包含 hasNonTriggerNodes 的行
    block_end = first_decl_line
    for j in range(first_decl_line, min(first_decl_line + 20, len(lines))):
        if 'hasNonTriggerNodes' in lines[j] or 'nonTriggerNodes' in lines[j]:
            # 继续找后续的 push 调用
            for k in range(j, min(j + 15, len(lines))):
                if 'issues.push' in lines[k]:
                    block_end = k
                    break
            break
        elif k == len(lines) - 1:
            block_end = j
    
    # 更简单的方法：直接删除到下一个主要注释（如 "// ── 2."）
    for j in range(first_decl_line, min(first_decl_line + 30, len(lines))):
        if j > first_decl_line and ('// ── 2.' in lines[j] or '// ── 3.' in lines[j] or '// ── 9.' in lines[j]):
            block_end = j - 1  # 不删除注释行
            break
        elif j == len(lines) - 1:
            block_end = j
    
    # 删除整个块
    if block_end > block_start:
        del lines[block_start:block_end + 1]
        print(f"Deleted hasEndNode block from line {block_start + 1} to {block_end + 1}")

with open(r'd:\OneManager\AxInvest\src\lib\workflowLayout.ts', 'w', encoding='utf-8') as f:
    f.writelines(lines)

print("完成！")
