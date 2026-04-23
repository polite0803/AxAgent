/** Information about a single local tool. */
export type LocalToolInfo = {
  toolName: string;
  description: string;
};

/** Information about a local tool group (for UI display). */
export type LocalToolGroupInfo = {
  groupId: string;
  groupName: string;
  enabled: boolean;
  tools: LocalToolInfo[];
};
