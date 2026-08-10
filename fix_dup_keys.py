# -*- coding: utf-8 -*-
import sys

with open('src/lib/workflowLayout.ts', 'r', encoding='utf-8') as f:
    content = f.read()

# 计算重复的 missing_end_node key
# 找到所有出现位置
key = '"workflow.layout.validate.missing_end_node"'
positions = []
idx = 0
while True:
    idx = content.find(key, idx)
    if idx == -1:
        break
    positions.append(idx)
    idx += len(key)

print(f"Found {len(positions)} occurrences of '{key}'")

# 如果有重复的 key 声明（在对象字面量中），删除第一个
# 第一个应该是在对象定义中
if len(positions) >= 2:
    # 找到第一个 key 及其值
    first_pos = positions[0]
    # 找到这个 key 后面的值（直到换行+逗号或换行+空格）
    # 向前找，确定删除范围
    # 从 first_pos 开始，向后找到第二个 key 的位置
    second_pos = positions[1]
    
    # 但我们只想删除第一个 key 的两行（key + value）
    # 找到第一个 key 所在行的起始位置
    line_start = content.rfind('\n', 0, first_pos) + 1
    
    # 找到第一个 key 值所在行的结束位置
    # 从 first_pos 找，直到找到下一个逗号后跟换行
    value_end = content.find(',\n', first_pos)
    if value_end != -1 and value_end < second_pos:
        # 但需要处理可能的多行值
        # 简单方法：删除从第一个 key 到下一个 key 之前的内容
        # 但保留第二个 key
    
        # 从第一个 key 开始，删除到第二个 key 之前的内容（但保留第二个 key）
        # 实际上，我们只需要删除第一个 key 的两行
        
        # 更精确：找到第一个 key 所在的两行
        # 从 first_pos 开始，找到换行符
        first_newline = content.find('\n', first_pos)
        if first_newline != -1:
            # 第一行是 key，第二行是 value
            second_line_end = content.find('\n', first_newline + 1)
            if second_line_end == -1:
                second_line_end = len(content)
            
            # 删除从 line_start 到 second_line_end 的内容
            content = content[:line_start] + content[second_line_end + 1:]
            print(f"Deleted lines from position {line_start} to {second_line_end}")

# 现在检查是否还有重复
positions2 = []
idx = 0
while True:
    idx = content.find(key, idx)
    if idx == -1:
        break
    positions2.append(idx)
    idx += len(key)

print(f"After fix: Found {len(positions2)} occurrences of '{key}'")

with open('src/lib/workflowLayout.ts', 'w', encoding='utf-8') as f:
    f.write(content)

print("Done!")
