import { Badge, Tag } from "antd";
import React, { memo } from "react";
import { Handle, type NodeProps, Position } from "reactflow";

interface ParallelNodeData {
  id: string;
  type: string;
  title: string;
  description?: string;
  color: string;
  nodeType: string;
  enabled: boolean;
  branches?: Array<{
    id: string;
    title: string;
    steps: string[];
  }>;
  waitForAll?: boolean;
}

const ParallelNodeComponent: React.FC<NodeProps<ParallelNodeData>> = ({ data, selected }) => {
  const color = "#fa8c16";
  const branches = data.branches || [];
  const waitForAll = data.waitForAll ?? true;

  return (
    <div
      style={{
        minWidth: 220,
        maxWidth: 280,
        opacity: data.enabled ? 1 : 0.5,
        filter: data.enabled ? "none" : "grayscale(100%)",
      }}
    >
      <div
        style={{
          background: "#1e1e1e",
          border: `2px solid ${selected ? "#1890ff" : color}`,
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: selected ? `0 0 0 2px ${color}40` : "none",
          transition: "all 0.2s",
        }}
      >
        <div
          style={{
            padding: "8px 12px",
            borderBottom: `1px solid ${color}30`,
            display: "flex",
            alignItems: "center",
            gap: 8,
            background: `${color}15`,
          }}
        >
          <span style={{ fontSize: 14 }}>⚡</span>
          <span
            style={{
              fontSize: 11,
              color: color,
              fontWeight: 600,
            }}
          >
            并行分支
          </span>
          <Tag
            style={{
              margin: 0,
              fontSize: 9,
              padding: "0 4px",
              background: `${color}30`,
              border: "none",
              color: "#fff",
            }}
          >
            {waitForAll ? "等待全部" : "任一完成"}
          </Tag>
        </div>

        <div style={{ padding: "10px 12px" }}>
          <div
            style={{
              fontSize: 13,
              color: "#fff",
              fontWeight: 500,
              marginBottom: 8,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {data.title}
          </div>

          {branches.length > 0
            ? (
              <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                {branches.slice(0, 4).map((branch, index) => (
                  <div
                    key={branch.id || index}
                    style={{
                      fontSize: 10,
                      color: "#aaa",
                      padding: "4px 8px",
                      background: "#252525",
                      borderRadius: 4,
                      borderLeft: `3px solid ${color}`,
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      overflow: "hidden",
                    }}
                  >
                    <span
                      style={{
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                        flex: 1,
                      }}
                    >
                      {branch.title || `分支 ${index + 1}`}
                    </span>
                    {branch.steps && (
                      <Badge
                        count={branch.steps.length}
                        size="small"
                        style={{
                          backgroundColor: color,
                          fontSize: 8,
                        }}
                      />
                    )}
                  </div>
                ))}
                {branches.length > 4 && (
                  <div
                    style={{
                      fontSize: 9,
                      color: "#666",
                      textAlign: "center",
                    }}
                  >
                    +{branches.length - 4} 更多分支
                  </div>
                )}
              </div>
            )
            : (
              <div
                style={{
                  fontSize: 10,
                  color: "#666",
                  textAlign: "center",
                  padding: 8,
                  background: "#252525",
                  borderRadius: 4,
                }}
              >
                点击添加分支
              </div>
            )}
        </div>
      </div>

      <Handle
        type="target"
        position={Position.Top}
        style={{
          background: color,
          border: "none",
          width: 8,
          height: 8,
        }}
      />

      <div style={{ display: "flex", justifyContent: "center", gap: 4, marginTop: 4 }}>
        {branches.length > 0
          ? (
            branches.slice(0, 5).map((_, index) => (
              <Handle
                key={index}
                type="source"
                position={Position.Bottom}
                id={`branch-${index}`}
                style={{
                  background: color,
                  border: "none",
                  width: 6,
                  height: 6,
                  position: "relative",
                  left: "auto",
                  right: "auto",
                  top: "auto",
                }}
              />
            ))
          )
          : (
            <Handle
              type="source"
              position={Position.Bottom}
              style={{
                background: color,
                border: "none",
                width: 8,
                height: 8,
              }}
            />
          )}
      </div>
    </div>
  );
};

export const ParallelNode = memo(ParallelNodeComponent);
