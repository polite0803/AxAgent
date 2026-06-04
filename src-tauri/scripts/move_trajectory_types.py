#!/usr/bin/env python3
"""Move trajectory DTOs and trait defs into harness, update re-exports and imports."""

import os
import re

HARNESS_SRC = "D:/OneManager/AxAgent/src-tauri/crates/harness/src"
TRAJECTORY_SRC = "D:/OneManager/AxAgent/src-tauri/crates/trajectory/src"
TRAJECTORY_RS = os.path.join(TRAJECTORY_SRC, "trajectory.rs")

# Step 1: Create harness/src/trajectory_types.rs with DTOs only (no impl blocks)
with open(TRAJECTORY_RS, 'r', encoding='utf-8') as f:
    content = f.read()

# Extract only structs/enums + derives, no impl blocks
# Keep: struct definitions, enum definitions, type aliases
# Remove: impl blocks, methods, default implementations

lines = content.split('\n')
output = []
in_impl = False
impl_depth = 0
skip_rest_of_struct = False

for line in lines:
    stripped = line.strip()
    
    # Skip impl blocks
    if stripped.startswith('impl ') and stripped.endswith('{'):
        in_impl = True
        impl_depth = 1
        continue
    
    if in_impl:
        if '{' in stripped:
            impl_depth += stripped.count('{')
        if '}' in stripped:
            impl_depth -= stripped.count('}')
            if impl_depth <= 0:
                in_impl = False
        continue
    
    # Skip standalone functions (not inside impl blocks - not present in trajectory.rs)
    if stripped.startswith('fn ') or stripped.startswith('pub fn '):
        skip_rest_of_struct = True
    
    if skip_rest_of_struct:
        if stripped == '':
            skip_rest_of_struct = False
        continue
    
    output.append(line)

trajectory_dtos = '\n'.join(output)

# Step 2: Read the auto_tool DTOs and the trait
with open(os.path.join(TRAJECTORY_SRC, "auto_tool.rs"), 'r', encoding='utf-8') as f:
    auto_tool = f.read()

# Extract GeneratedTool, ToolCreationRequest structs, and LlmToolProvider trait (without impls)
extracted = []
for section in ["pub struct GeneratedTool", "pub struct ToolCreationRequest", "pub trait LlmToolProvider"]:
    idx = auto_tool.find(section)
    if idx >= 0:
        # Extract from section start to end of the struct/trait
        snippet = auto_tool[idx:]
        # Find the closing } that ends this struct/trait
        depth = 0
        end = 0
        for i, ch in enumerate(snippet):
            if ch == '{':
                depth += 1
            elif ch == '}':
                depth -= 1
                if depth < 0:
                    end = i + 1
                    break
        extracted.append(snippet[:end])

# Step 3: Read skill_evolution.rs for LlmEvolutionProvider trait and DTOs
with open(os.path.join(TRAJECTORY_SRC, "skill_evolution.rs"), 'r', encoding='utf-8') as f:
    skill_ev = f.read()

for section in ["pub struct LlmMutationRequest", "pub struct LlmMutationResponse", "pub type LlmMutationFuture", "pub trait LlmEvolutionProvider"]:
    idx = skill_ev.find(section)
    if idx >= 0:
        snippet = skill_ev[idx:]
        depth = 0
        end = 0
        for i, ch in enumerate(snippet):
            if ch == '{':
                depth += 1
            elif ch == '}':
                depth -= 1
                if depth < 0:
                    end = i + 1
                    break
        extracted.append(snippet[:end])

# Step 4: Read rl.rs for LlmJudge
with open(os.path.join(TRAJECTORY_SRC, "rl.rs"), 'r', encoding='utf-8') as f:
    rl = f.read()

for section in ["pub type LlmJudgeFuture", "pub trait LlmJudge"]:
    idx = rl.find(section)
    if idx >= 0:
        snippet = rl[idx:]
        depth = 0
        end = 0
        for i, ch in enumerate(snippet):
            if ch == '{':
                depth += 1
            elif ch == '}':
                depth -= 1
                if depth < 0:
                    end = i + 1
                    break
        extracted.append(snippet[:end])

# Step 5: Read text_grad.rs for LlmTextGradProvider
with open(os.path.join(TRAJECTORY_SRC, "text_grad.rs"), 'r', encoding='utf-8') as f:
    tg = f.read()

idx = tg.find("pub trait LlmTextGradProvider")
if idx >= 0:
    snippet = tg[idx:]
    depth = 0
    end = 0
    for i, ch in enumerate(snippet):
        if ch == '{':
            depth += 1
        elif ch == '}':
            depth -= 1
            if depth < 0:
                end = i + 1
                break
    extracted.append(snippet[:end])

# Step 6: Read process_reward.rs for PrmLlmProvider + RewardCategory + StepReward
with open(os.path.join(TRAJECTORY_SRC, "process_reward.rs"), 'r', encoding='utf-8') as f:
    pr = f.read()

for section in ["pub enum RewardCategory", "pub struct StepReward", "pub trait PrmLlmProvider"]:
    idx = pr.find(section)
    if idx >= 0:
        snippet = pr[idx:]
        depth = 0
        end = 0
        for i, ch in enumerate(snippet):
            if ch == '{':
                depth += 1
            elif ch == '}':
                depth -= 1
                if depth < 0:
                    end = i + 1
                    break
        extracted.append(snippet[:end])

# Step 7: Read the harness lib.rs to find insertion point for the new module
harness_lib = os.path.join(HARNESS_SRC, "lib.rs")
with open(harness_lib, 'r', encoding='utf-8') as f:
    lib_content = f.read()

# Build the trajectory_types.rs content
trajectory_types_content = """//! 轨迹（Trajectory）数据类型和 LLM 桥接契约
//!
//! 由 `axagent-trajectory` 实现，`axagent-agent` 等消费方使用。
//! 纯数据 DTO + trait 接口定义，**不含业务逻辑**。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

pub use crate::types::MessageRole;

""" + trajectory_dtos + "\n\n" + "\n\n".join(extracted)

# Write the harness module
with open(os.path.join(HARNESS_SRC, "trajectory_types.rs"), 'w', encoding='utf-8') as f:
    f.write(trajectory_types_content)

print(f"Created harness/src/trajectory_types.rs ({len(trajectory_types_content)} chars)")

# Step 8: Add pub mod trajectory_types to harness lib.rs
if "pub mod trajectory_types" not in lib_content:
    # Insert after the existing type modules (after rag_config)
    insert_point = lib_content.find("pub mod rag_config;")
    if insert_point >= 0:
        insert_point = lib_content.find("\n", insert_point) + 1
        lib_content = lib_content[:insert_point] + "pub mod trajectory_types;\n" + lib_content[insert_point:]
        
        with open(harness_lib, 'w', encoding='utf-8') as f:
            f.write(lib_content)
        print("Added pub mod trajectory_types to harness lib.rs")

# Step 9: Update trajectory crate's lib.rs to re-export from harness
trajectory_lib = os.path.join(TRAJECTORY_SRC, "lib.rs")
with open(trajectory_lib, 'r', encoding='utf-8') as f:
    traj_lib = f.read()

# Add re-export after existing module declarations
insert_point = traj_lib.find("pub mod trajectory_compressor;")
if insert_point >= 0:
    insert_point = traj_lib.find("\n", insert_point) + 1
    traj_lib = traj_lib[:insert_point] + """
// ── 共享类型由 axagent-harness 定义（trajectory_types），本模块仅做 re-export ──
pub use axagent_harness::trajectory_types::{
    GeneratedTool, LlmEvolutionProvider, LlmJudge, LlmJudgeFuture, LlmMutationFuture,
    LlmMutationRequest, LlmMutationResponse, LlmTextGradProvider, LlmToolProvider,
    MessageRole, PrmLlmProvider, RewardCategory, StepReward, ToolCall,
    ToolCreationRequest, ToolResult, Trajectory, TrajectoryOutcome, TrajectoryQuality,
    TrajectoryStep,
};

""" + traj_lib[insert_point:]

with open(trajectory_lib, 'w', encoding='utf-8') as f:
    f.write(traj_lib)
print("Updated trajectory lib.rs with re-exports from harness")

# Step 10: Update trajectory's trajectory.rs to re-export from harness
new_traj_rs = """//! 核心轨迹数据结构 —— 由 axagent-harness 定义，本模块 re-export 并添加业务方法

// 纯 DTO 通过 axagent-harness 提供
pub use axagent_harness::trajectory_types::{
    MessageRole, RewardSignal, ToolCall, ToolResult, Trajectory, TrajectoryOutcome,
    TrajectoryQuality, TrajectoryStep,
};
"""
# Append the impl blocks from the original file
# Extract the impl blocks
with open(TRAJECTORY_RS, 'r', encoding='utf-8') as f:
    orig = f.read()

# Find impl blocks
impl_blocks = []
idx = 0
while True:
    idx = orig.find("impl ", idx)
    if idx < 0:
        break
    # Find the end of this impl block
    brace_depth = 0
    start = idx
    for i in range(idx, len(orig)):
        if orig[i] == '{':
            brace_depth += 1
        elif orig[i] == '}':
            brace_depth -= 1
            if brace_depth == 0:
                end = i + 1
                impl_blocks.append(orig[start:end])
                idx = end
                break

# Add all impl blocks
new_traj_rs += "\n\n// ── 业务方法 ──\n"
for block in impl_blocks:
    new_traj_rs += "\n" + block + "\n"

with open(TRAJECTORY_RS, 'w', encoding='utf-8') as f:
    f.write(new_traj_rs)
print(f"Updated trajectory.rs ({len(new_traj_rs)} chars)")

print("\nDone! Now need to fix imports in agent and compile.")
