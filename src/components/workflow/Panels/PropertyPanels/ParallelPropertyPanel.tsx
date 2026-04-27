import { useWorkflowEditorStore } from "@/stores";
import { Button, Divider, Input, Select, Switch, Tag } from "antd";
import { GripVertical, Plus, Trash2, X } from "lucide-react";
import React from "react";
import type { Branch, ParallelNode, WorkflowNode } from "../../types";
import { BasePropertyPanel } from "./BasePropertyPanel";

interface ParallelPropertyPanelProps {
  node: WorkflowNode;
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onDelete: () => void;
}

export const ParallelPropertyPanel: React.FC<ParallelPropertyPanelProps> = ({ node, onUpdate, onDelete }) => {
  const parallelNode = node as ParallelNode;
  const config = parallelNode.config || {
    branches: [],
    wait_for_all: true,
  };

  const { nodes } = useWorkflowEditorStore();

  const getNodeLabel = (nodeId: string) => {
    const found = nodes.find(n => n.id === nodeId);
    return found ? `${found.title || found.id} (${found.type})` : nodeId;
  };

  const getAvailableNodes = (excludeIds: string[]) => {
    return nodes.filter(n => !excludeIds.includes(n.id));
  };

  const handleAddBranch = () => {
    const newBranch: Branch = {
      id: `branch-${Date.now()}`,
      title: `分支 ${config.branches.length + 1}`,
      steps: [],
    };
    onUpdate({
      config: {
        ...config,
        branches: [...config.branches, newBranch],
      },
    });
  };

  const handleUpdateBranch = (index: number, updates: Partial<Branch>) => {
    const newBranches = [...config.branches];
    newBranches[index] = { ...newBranches[index], ...updates };
    onUpdate({
      config: {
        ...config,
        branches: newBranches,
      },
    });
  };

  const handleDeleteBranch = (index: number) => {
    const newBranches = config.branches.filter((_, i) => i !== index);
    onUpdate({
      config: {
        ...config,
        branches: newBranches,
      },
    });
  };

  const handleAddStepToBranch = (branchIndex: number, nodeId: string) => {
    const branch = config.branches[branchIndex];
    if (!branch.steps.includes(nodeId)) {
      handleUpdateBranch(branchIndex, { steps: [...branch.steps, nodeId] });
    }
  };

  const handleRemoveStepFromBranch = (branchIndex: number, nodeId: string) => {
    const branch = config.branches[branchIndex];
    handleUpdateBranch(branchIndex, { steps: branch.steps.filter(id => id !== nodeId) });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <label style={{ color: "#999", fontSize: 11 }}>等待全部分支完成</label>
        <Switch
          size="small"
          checked={config.wait_for_all}
          onChange={(checked) =>
            onUpdate({
              config: {
                ...config,
                wait_for_all: checked,
              },
            })}
        />
      </div>
      <div style={{ color: "#666", fontSize: 10 }}>
        {config.wait_for_all
          ? "将等待所有分支完成后继续"
          : "任一分支完成即继续执行"}
      </div>

      <div>
        <label style={{ display: "block", color: "#999", fontSize: 11, marginBottom: 4 }}>超时时间 (秒)</label>
        <Input
          type="number"
          value={config.timeout ?? ""}
          onChange={(e) =>
            onUpdate({
              config: {
                ...config,
                timeout: e.target.value ? parseInt(e.target.value) : undefined,
              },
            })}
          size="small"
          placeholder="不设置"
        />
      </div>

      <div>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
          <label style={{ color: "#999", fontSize: 11 }}>
            分支 ({config.branches.length})
          </label>
          <Button
            type="dashed"
            size="small"
            icon={<Plus size={12} />}
            onClick={handleAddBranch}
          >
            添加分支
          </Button>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {config.branches.map((branch, index) => {
            const availableNodes = getAvailableNodes(branch.steps);
            return (
              <div
                key={branch.id || index}
                style={{
                  padding: 8,
                  background: "#1e1e1e",
                  borderRadius: 6,
                  border: "1px solid #333",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 4, marginBottom: 8 }}>
                  <GripVertical size={12} color="#666" />
                  <Input
                    value={branch.title}
                    onChange={(e) => handleUpdateBranch(index, { title: e.target.value })}
                    size="small"
                    placeholder="分支名称"
                    style={{ flex: 1 }}
                  />
                  <Button
                    type="text"
                    danger
                    size="small"
                    icon={<Trash2 size={12} />}
                    onClick={() => handleDeleteBranch(index)}
                  />
                </div>

                <div style={{ display: "flex", flexDirection: "column", gap: 4, paddingLeft: 20 }}>
                  <label style={{ fontSize: 10, color: "#888" }}>步骤:</label>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
                    {branch.steps.map((stepId) => (
                      <Tag
                        key={stepId}
                        closable
                        onClose={() => handleRemoveStepFromBranch(index, stepId)}
                        style={{ background: "#2a2a2a", border: "1px solid #444", color: "#ddd" }}
                        closeIcon={<X size={10} />}
                      >
                        {getNodeLabel(stepId)}
                      </Tag>
                    ))}
                    {branch.steps.length === 0 && <span style={{ fontSize: 10, color: "#666" }}>暂无步骤</span>}
                  </div>
                  {availableNodes.length > 0 && (
                    <Select
                      placeholder="添加步骤"
                      size="small"
                      style={{ width: "100%", marginTop: 4 }}
                      onChange={(nodeId) => handleAddStepToBranch(index, nodeId)}
                      options={availableNodes.map(n => ({
                        value: n.id,
                        label: `${n.title || n.id} (${n.type})`,
                      }))}
                    />
                  )}
                </div>
              </div>
            );
          })}

          {config.branches.length === 0 && (
            <div style={{ color: "#666", fontSize: 11, textAlign: "center", padding: 16 }}>
              点击"添加分支"创建第一个并行分支
            </div>
          )}
        </div>
      </div>

      <Divider style={{ margin: "8px 0", borderColor: "#333" }} />

      <div style={{ borderTop: "1px solid #333", paddingTop: 12, marginTop: 4 }}>
        <BasePropertyPanel node={node} onUpdate={onUpdate} onDelete={onDelete} />
      </div>
    </div>
  );
};
