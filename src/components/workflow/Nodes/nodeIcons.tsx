// SPDX-License-Identifier: AGPL-3.0-only

import {
  AppstoreOutlined,
  AuditOutlined,
  BranchesOutlined,
  BulbOutlined,
  CheckCircleOutlined,
  ClockCircleOutlined,
  CodeOutlined,
  CommentOutlined,
  DatabaseOutlined,
  FileSearchOutlined,
  FileTextOutlined,
  FlagOutlined,
  FolderOutlined,
  FunctionOutlined,
  GlobalOutlined,
  HddOutlined,
  InteractionOutlined,
  LinkOutlined,
  MailOutlined,
  MergeOutlined,
  MinusOutlined,
  NotificationOutlined,
  PartitionOutlined,
  ProjectOutlined,
  RetweetOutlined,
  RobotOutlined,
  SearchOutlined,
  SendOutlined,
  SwapOutlined,
  TagsOutlined,
  TeamOutlined,
  ThunderboltOutlined,
  ToolOutlined,
} from "@ant-design/icons";
import type { ComponentType, CSSProperties, ReactNode } from "react";

/**
 * 节点类型 → antd 单色图标映射。
 * 替代原先各组件内联的 emoji，保证跨平台渲染一致、可随主题着色。
 */
export const NODE_ICONS: Record<string, ComponentType<{ style?: CSSProperties }>> = {
  trigger: ThunderboltOutlined,
  agent: RobotOutlined,
  llm: BulbOutlined,
  llmClassifier: TagsOutlined,
  condition: BranchesOutlined,
  switch: SwapOutlined,
  parallel: PartitionOutlined,
  loop: RetweetOutlined,
  debate: CommentOutlined,
  swarm: TeamOutlined,
  merge: MergeOutlined,
  aggregator: FunctionOutlined,
  delay: ClockCircleOutlined,
  tool: ToolOutlined,
  code: CodeOutlined,
  subWorkflow: ProjectOutlined,
  workflowRef: LinkOutlined,
  documentParser: FileSearchOutlined,
  vectorRetrieve: SearchOutlined,
  storage: HddOutlined,
  databaseQuery: DatabaseOutlined,
  httpRequest: GlobalOutlined,
  validation: CheckCircleOutlined,
  notification: NotificationOutlined,
  approval: AuditOutlined,
  fileOperation: FolderOutlined,
  dataTransformer: InteractionOutlined,
  webhookSend: SendOutlined,
  logging: FileTextOutlined,
  email: MailOutlined,
  end: FlagOutlined,
  _phaseSeparator: MinusOutlined,
  groupFrame: AppstoreOutlined,
};

const FALLBACK = AppstoreOutlined;

export function nodeIconFor(type: string): ReactNode {
  const Cmp = NODE_ICONS[type] || FALLBACK;
  return <Cmp style={{ fontSize: 13 }} />;
}
