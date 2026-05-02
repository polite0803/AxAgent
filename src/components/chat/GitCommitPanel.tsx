import { invoke } from "@/lib/invoke";
import { Button, Card, Divider, Empty, Input, message, Space, Tag, Typography } from "antd";
import { Check, FileText, GitBranch, GitCommit, Loader2, Minus, Plus, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;
const { TextArea } = Input;

interface FileDiff {
  path: string;
  insertions: number;
  deletions: number;
  status: "added" | "modified" | "deleted" | "renamed";
}

interface GitDiffSummary {
  files: FileDiff[];
  total_insertions: number;
  total_deletions: number;
  total_files: number;
}

interface GitCommitPanelProps {
  repoPath?: string;
  onCommitSuccess?: (commitHash: string) => void;
}

export function GitCommitPanel({ repoPath, onCommitSuccess }: GitCommitPanelProps) {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);
  const [diff, setDiff] = useState<GitDiffSummary | null>(null);
  const [commitMessage, setCommitMessage] = useState("");
  const [generatedMessage, setGeneratedMessage] = useState("");
  const [currentBranch, setCurrentBranch] = useState<string>("");
  const [staged, setStaged] = useState(false);

  useEffect(() => {
    if (repoPath) {
      loadGitInfo();
    }
  }, [repoPath]);

  const loadGitInfo = async () => {
    setLoading(true);
    try {
      const branch = await invoke<string>("get_current_branch", { repoPath: repoPath || "." });
      setCurrentBranch(branch);
    } catch {
      setCurrentBranch("main");
    }

    try {
      const diffData = await invoke<GitDiffSummary>("get_staged_diff", { repoPath: repoPath || "." });
      setDiff(diffData);
      setStaged(diffData.total_files > 0);
    } catch {
      setDiff(null);
      setStaged(false);
    }

    setLoading(false);
  };

  const generateCommitMessage = async () => {
    if (!repoPath) { return; }
    setLoading(true);
    try {
      const message_text = await invoke<string>("generate_commit_context", { repoPath });
      setGeneratedMessage(message_text);
      if (!commitMessage) {
        setCommitMessage(message_text);
      }
    } catch (e) {
      message.error(t("chat.git.generateMessageError"));
    }
    setLoading(false);
  };

  const createCommit = async () => {
    if (!commitMessage.trim()) {
      message.warning(t("chat.git.emptyMessage"));
      return;
    }
    setLoading(true);
    try {
      const result = await invoke<string>("git_commit", { repoPath: repoPath || ".", message: commitMessage });
      message.success(t("chat.git.commitSuccess"));
      setCommitMessage("");
      setGeneratedMessage("");
      setStaged(false);
      setDiff(null);
      onCommitSuccess?.(result);
      await loadGitInfo();
    } catch (e) {
      message.error(t("chat.git.commitError", { error: String(e) }));
    }
    setLoading(false);
  };

  const getStatusIcon = (status: FileDiff["status"]) => {
    switch (status) {
      case "added":
        return <Plus size={12} className="text-green-500" />;
      case "deleted":
        return <Minus size={12} className="text-red-500" />;
      case "modified":
        return <FileText size={12} className="text-blue-500" />;
      case "renamed":
        return <GitBranch size={12} className="text-purple-500" />;
    }
  };

  const getStatusColor = (status: FileDiff["status"]): string => {
    switch (status) {
      case "added":
        return "green";
      case "deleted":
        return "red";
      case "modified":
        return "blue";
      case "renamed":
        return "purple";
    }
  };

  if (!repoPath) {
    return (
      <Card size="small">
        <Empty description={t("chat.git.noRepo")} />
      </Card>
    );
  }

  return (
    <Card size="small" className="git-commit-panel">
      <div className="flex items-center justify-between mb-3">
        <Space>
          <GitBranch size={16} className="text-blue-500" />
          <Text strong>{t("chat.git.title")}</Text>
          <Tag>{currentBranch}</Tag>
        </Space>
        <Button
          type="text"
          size="small"
          icon={<RefreshCw size={14} className={loading ? "animate-spin" : ""} />}
          onClick={loadGitInfo}
          disabled={loading}
        />
      </div>

      {diff && diff.total_files > 0
        ? (
          <>
            <div className="mb-3">
              <Text type="secondary" className="text-xs">
                {t("chat.git.stagedChanges", { count: diff.total_files })}
              </Text>
              <div className="flex gap-3 mt-1">
                <Tag color="green">+{diff.total_insertions}</Tag>
                <Tag color="red">-{diff.total_deletions}</Tag>
              </div>
            </div>

            <div className="max-h-40 overflow-auto mb-3 border border-gray-200 dark:border-gray-700 rounded">
              {diff.files.map((file, index) => (
                <div
                  key={index}
                  className="flex items-center justify-between px-2 py-1 text-xs border-b border-gray-100 dark:border-gray-800 last:border-b-0"
                >
                  <Space size="small">
                    {getStatusIcon(file.status)}
                    <Text className="font-mono">{file.path}</Text>
                  </Space>
                  <Space size="small">
                    <Tag color={getStatusColor(file.status)} className="text-xs">
                      {file.status}
                    </Tag>
                    {file.insertions > 0 && <Text className="text-green-500">+{file.insertions}</Text>}
                    {file.deletions > 0 && <Text className="text-red-500">-{file.deletions}</Text>}
                  </Space>
                </div>
              ))}
            </div>

            <Divider className="my-2" />

            <div className="mb-3">
              <div className="flex items-center justify-between mb-1">
                <Text type="secondary" className="text-xs">{t("chat.git.commitMessage")}</Text>
                <Button
                  type="link"
                  size="small"
                  icon={loading ? <Loader2 size={12} className="animate-spin" /> : <Check size={12} />}
                  onClick={generateCommitMessage}
                  disabled={loading}
                >
                  {t("chat.git.generateMessage")}
                </Button>
              </div>
              <TextArea
                value={commitMessage}
                onChange={(e) => setCommitMessage(e.target.value)}
                placeholder={t("chat.git.messagePlaceholder")}
                rows={3}
                maxLength={500}
                showCount
              />
              {generatedMessage && generatedMessage !== commitMessage && (
                <Paragraph type="secondary" className="text-xs mt-1">
                  {t("chat.git.generatedSuggestion")}: {generatedMessage.slice(0, 100)}...
                </Paragraph>
              )}
            </div>

            <Button
              type="primary"
              icon={<GitCommit size={14} />}
              onClick={createCommit}
              loading={loading}
              block
              disabled={!commitMessage.trim() || !staged}
            >
              {t("chat.git.createCommit")}
            </Button>
          </>
        )
        : (
          <Empty
            description={t("chat.git.noStagedChanges")}
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        )}
    </Card>
  );
}

interface GitBranchPanelProps {
  repoPath?: string;
  baseBranch?: string;
  headBranch?: string;
}

export function GitBranchPanel({ repoPath, baseBranch, headBranch }: GitBranchPanelProps) {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);
  const [diff, setDiff] = useState<GitDiffSummary | null>(null);
  const [prDescription, setPrDescription] = useState("");
  const [commits, setCommits] = useState<string[]>([]);

  useEffect(() => {
    if (repoPath && baseBranch && headBranch) {
      loadBranchDiff();
    }
  }, [repoPath, baseBranch, headBranch]);

  const loadBranchDiff = async () => {
    if (!repoPath || !baseBranch || !headBranch) { return; }
    setLoading(true);
    try {
      const diffData = await invoke<GitDiffSummary>("get_branch_diff", {
        repoPath,
        baseBranch,
        headBranch,
      });
      setDiff(diffData);

      const commitList = await invoke<string[]>("get_branch_commits", {
        repoPath,
        branch: headBranch,
      });
      setCommits(commitList);

      const prDesc = await invoke<string>("generate_pr_context", {
        repoPath,
        baseBranch,
        headBranch,
      });
      setPrDescription(prDesc);
    } catch (e) {
      console.error("Failed to load branch diff:", e);
    }
    setLoading(false);
  };

  if (!repoPath) {
    return (
      <Card size="small">
        <Empty description={t("chat.git.noRepo")} />
      </Card>
    );
  }

  return (
    <Card size="small" className="git-branch-panel">
      <div className="flex items-center justify-between mb-3">
        <Space>
          <GitBranch size={16} className="text-purple-500" />
          <Text strong>{t("chat.git.branchComparison")}</Text>
        </Space>
        <Button
          type="text"
          size="small"
          icon={<RefreshCw size={14} className={loading ? "animate-spin" : ""} />}
          onClick={loadBranchDiff}
          disabled={loading}
        />
      </div>

      <div className="flex items-center gap-2 mb-3">
        <Tag color="blue">{baseBranch || "base"}</Tag>
        <Text type="secondary">←</Text>
        <Tag color="green">{headBranch || "head"}</Tag>
      </div>

      {diff && (
        <>
          <div className="mb-3">
            <Text type="secondary" className="text-xs">
              {t("chat.git.filesChanged", { count: diff.total_files })}
            </Text>
            <div className="flex gap-3 mt-1">
              <Tag color="green">+{diff.total_insertions}</Tag>
              <Tag color="red">-{diff.total_deletions}</Tag>
            </div>
          </div>

          {commits.length > 0 && (
            <div className="mb-3">
              <Text type="secondary" className="text-xs block mb-1">{t("chat.git.commits")}:</Text>
              <div className="max-h-24 overflow-auto">
                {commits.map((commit, index) => (
                  <Text key={index} className="text-xs font-mono block">
                    {commit.slice(0, 7)}
                  </Text>
                ))}
              </div>
            </div>
          )}

          {prDescription && (
            <div className="mb-3">
              <Text type="secondary" className="text-xs block mb-1">{t("chat.git.prDescription")}:</Text>
              <div className="p-2 bg-gray-50 dark:bg-gray-800 rounded text-xs">
                <Paragraph className="text-xs mb-0" ellipsis={{ rows: 4, expandable: true }}>
                  {prDescription}
                </Paragraph>
              </div>
            </div>
          )}
        </>
      )}
    </Card>
  );
}

export default GitCommitPanel;
