"""将 mod.rs 中的手写 DomainAdapterFactory 实现替换为 include 宏。"""

import sys

MOD_RS = r"d:\OneManager\AxInvest\src-tauri\crates\analysis-engine\src\opc\domain\mod.rs"

with open(MOD_RS, "r", encoding="utf-8") as f:
    lines = f.readlines()

# 保留前 417 行（索引 0-416），然后添加 include!，再从第 1620 行开始保留（索引 1619+）
before = lines[:417]  # 包含第 417 行（索引 416）
after = lines[1619:]  # 从第 1620 行开始（索引 1619）

include_line = '\ninclude!("generated.rs");\n'

new_content = ''.join(before) + include_line + ''.join(after)

with open(MOD_RS, "w", encoding="utf-8") as f:
    f.write(new_content)

print(f"完成！前 {len(before)} 行 + include! + 后 {len(after)} 行")