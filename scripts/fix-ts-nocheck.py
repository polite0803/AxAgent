#!/usr/bin/env python3
"""Remove @ts-nocheck and restore NodeProps<XxxData> generics."""

import os
import re

NODES_DIR = "D:/OneManager/AxAgent/src/components/workflow/Nodes"

# Map: filename -> data type name
DATA_TYPES = {
    "AgentNode.tsx": "AgentNodeData",
    "AggregatorNode.tsx": "AggregatorNodeData",
    "ApprovalNode.tsx": "ApprovalNodeData",
    "BaseNode.tsx": "BaseNodeData",
    "CodeNode.tsx": "CodeNodeData",
    "ConditionNode.tsx": "ConditionNodeData",
    "ContainerNode.tsx": "ContainerNodeData",
    "DatabaseQueryNode.tsx": "DatabaseQueryNodeData",
    "DataTransformerNode.tsx": "DataTransformerNodeData",
    "DebateNode.tsx": "DebateNodeData",
    "DelayNode.tsx": "DelayNodeData",
    "DocumentParserNode.tsx": "DocumentParserNodeData",
    "EmailNode.tsx": "EmailNodeData",
    "EndNode.tsx": "EndNodeData",
    "FileOperationNode.tsx": "FileOperationNodeData",
    "GroupFrameNode.tsx": "GroupFrameData",
    "HttpRequestNode.tsx": "HttpRequestNodeData",
    "LlmClassifierNode.tsx": "LlmClassifierNodeData",
    "LLMNode.tsx": "LLMNodeData",
    "LoggingNode.tsx": "LoggingNodeData",
    "LoopNode.tsx": "LoopNodeData",
    "MergeNode.tsx": "MergeNodeData",
    "NotificationNode.tsx": "NotificationNodeData",
    "ParallelNode.tsx": "ParallelNodeData",
    "PhaseSeparatorNode.tsx": "PhaseSeparatorNodeData",
    "StorageNode.tsx": "StorageNodeData",
    "SubWorkflowNode.tsx": "SubWorkflowNodeData",
    "SwarmNode.tsx": "SwarmNodeData",
    "SwitchNode.tsx": "SwitchNodeData",
    "ToolNode.tsx": "ToolNodeData",
    "TriggerNode.tsx": "TriggerNodeData",
    "ValidationNode.tsx": "ValidationNodeData",
    "VectorRetrieveNode.tsx": "VectorRetrieveNodeData",
    "WebhookSendNode.tsx": "WebhookSendNodeData",
}

for fname, dtype in DATA_TYPES.items():
    fp = os.path.join(NODES_DIR, fname)
    if not os.path.exists(fp):
        print(f"SKIP {fname} (not found)")
        continue
    
    with open(fp, "r", encoding="utf-8") as f:
        content = f.read()
    
    original = content
    
    # Remove @ts-nocheck
    content = content.replace("// @ts-nocheck\n", "")
    
    # Replace `NodeProps` with `NodeProps<Dtype>` (only where it's used as a component type)
    content = re.sub(
        r'React\.FC<NodeProps>\(',
        f'React.FC<NodeProps<{dtype}>>(',
        content,
    )
    
    if content != original:
        with open(fp, "w", encoding="utf-8") as f:
            f.write(content)
        print(f"FIXED {fname} ({dtype})")
    else:
        print(f"OK   {fname}")

print("\nAll done")
