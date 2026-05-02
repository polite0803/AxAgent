import { invoke, isTauri } from "@/lib/invoke";
import { useBackupStore, useSettingsStore } from "@/stores";
import {
  App,
  Button,
  Checkbox,
  Divider,
  Form,
  Input,
  InputNumber,
  Modal,
  Popconfirm,
  Select,
  Switch,
  Tag,
  TimePicker,
  Typography,
} from "antd";
import dayjs from "dayjs";
import { Calendar, Clock, Edit2, History, Pause, Play, Plus, RefreshCw, Rocket, Trash2, Zap } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";

const { Text } = Typography;

const rowStyle: React.CSSProperties = { padding: "4px 0" };

type Weekday = "monday" | "tuesday" | "wednesday" | "thursday" | "friday" | "saturday" | "sunday";
type ScheduleType = "interval" | "daily" | "weekly" | "monthly" | "advanced";

interface TimeRange {
  start_hour: number;
  start_minute: number;
  end_hour: number;
  end_minute: number;
}

interface ScheduleConfig {
  schedule_type: ScheduleType;
  weekdays: Weekday[];
  time_ranges: TimeRange[];
  interval_seconds: number | null;
  exclude_holidays: boolean;
  exclude_custom_dates: string[];
  month_day: number | null;
}

interface TaskConfig {
  timeout_seconds: number;
  retry_on_failure: boolean;
  max_retries: number;
  retry_delay_seconds: number;
  notification_enabled: boolean;
  run_on_startup: boolean;
}

interface ScheduledTask {
  id: string;
  name: string;
  description: string;
  task_type: "custom";
  cron_expression: string | null;
  interval_seconds: number | null;
  next_run_at: string;
  last_run_at: string | null;
  last_result: { success: boolean; output: string; error: string | null; duration_ms: number } | null;
  status: "active" | "paused" | "disabled";
  config: TaskConfig;
  schedule_config: ScheduleConfig;
  created_at: string;
  updated_at: string;
}

interface TaskFormData {
  name: string;
  description: string;
  template_type?: string;
  workflow_id?: string | null;
  schedule_type: ScheduleType;
  interval_hours: number;
  weekdays: Weekday[];
  time_ranges: { start: dayjs.Dayjs; end: dayjs.Dayjs }[];
  exclude_holidays: boolean;
  exclude_custom_dates: string[];
  month_day: number | null;
}

interface TaskTemplate {
  template_type: string;
  name: string;
  description: string;
  schedule_config: ScheduleConfig;
  workflow_id: string | null;
}

export function SchedulerSettings() {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const inTauri = isTauri();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const backupSettings = useBackupStore((s) => s.backupSettings);
  const updateBackupSettings = useBackupStore((s) => s.updateBackupSettings);

  interface ExecutionRecord {
    id: string;
    task_id: string;
    started_at: string;
    completed_at: string | null;
    success: boolean;
    output: string | null;
    error: string | null;
    duration_ms: number;
  }

  const [customTasks, setCustomTasks] = useState<ScheduledTask[]>([]);
  const [taskTemplates, setTaskTemplates] = useState<TaskTemplate[]>([]);
  const [taskModalOpen, setTaskModalOpen] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(null);
  const [editingTask, setEditingTask] = useState<ScheduledTask | null>(null);
  const [form] = Form.useForm<TaskFormData>();
  const [loading, setLoading] = useState(false);
  const [executing, setExecuting] = useState<Record<string, boolean>>({});
  const [historyMap, setHistoryMap] = useState<Record<string, ExecutionRecord[]>>({});
  const [expandedHistory, setExpandedHistory] = useState<Record<string, boolean>>({});

  const loadCustomTasks = async () => {
    try {
      const tasks = await invoke<ScheduledTask[]>("list_scheduled_tasks");
      setCustomTasks(tasks.filter(t => t.task_type === "custom"));
    } catch (e) {
      console.warn("Failed to load scheduled tasks:", e);
    }
  };

  const loadTemplates = async () => {
    try {
      const templates = await invoke<TaskTemplate[]>("get_scheduled_task_templates");
      setTaskTemplates(templates);
    } catch (e) {
      console.warn("Failed to load task templates:", e);
    }
  };

  useEffect(() => {
    if (inTauri) {
      loadCustomTasks();
      loadTemplates();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [inTauri]);

  const handleAutoBackupChange = async (enabled: boolean) => {
    if (!backupSettings) { return; }
    const newSettings = { ...backupSettings, enabled };
    await updateBackupSettings(newSettings);
    message.success(t("settings.scheduler.saved"));
  };

  const handleBackupIntervalChange = async (intervalHours: number | null) => {
    if (!backupSettings || !intervalHours) { return; }
    const newSettings = { ...backupSettings, intervalHours };
    await updateBackupSettings(newSettings);
    message.success(t("settings.scheduler.saved"));
  };

  const handleWebdavSyncChange = async (syncEnabled: boolean) => {
    saveSettings({ webdav_sync_enabled: syncEnabled });
    if (inTauri) {
      try {
        await invoke("restart_webdav_sync");
      } catch (e) {
        console.warn("Failed to restart WebDAV sync:", e);
      }
    }
    message.success(t("settings.scheduler.saved"));
  };

  const handleWebdavIntervalChange = async (syncIntervalMinutes: number) => {
    saveSettings({ webdav_sync_interval_minutes: syncIntervalMinutes });
    if (inTauri) {
      try {
        await invoke("restart_webdav_sync");
      } catch (e) {
        console.warn("Failed to restart WebDAV sync:", e);
      }
    }
    message.success(t("settings.scheduler.saved"));
  };

  const handleClosedLoopChange = (closedLoopEnabled: boolean) => {
    saveSettings({ closed_loop_enabled: closedLoopEnabled });
    message.success(t("settings.scheduler.saved"));
  };

  const handleClosedLoopIntervalChange = (closedLoopIntervalMinutes: number) => {
    saveSettings({ closed_loop_interval_minutes: closedLoopIntervalMinutes });
    message.success(t("settings.scheduler.saved"));
  };

  const webdavIntervalOptions = [
    { value: 15, label: t("settings.scheduler.minutes", { count: 15 }) },
    { value: 30, label: t("settings.scheduler.minutes", { count: 30 }) },
    { value: 60, label: t("settings.scheduler.hour") },
    { value: 120, label: t("settings.scheduler.hours", { count: 2 }) },
    { value: 360, label: t("settings.scheduler.hours", { count: 6 }) },
    { value: 720, label: t("settings.scheduler.hours", { count: 12 }) },
    { value: 1440, label: t("settings.scheduler.hours", { count: 24 }) },
  ];

  const closedLoopIntervalOptions = [
    { value: 1, label: t("settings.scheduler.minutes", { count: 1 }) },
    { value: 5, label: t("settings.scheduler.minutes", { count: 5 }) },
    { value: 10, label: t("settings.scheduler.minutes", { count: 10 }) },
    { value: 15, label: t("settings.scheduler.minutes", { count: 15 }) },
    { value: 30, label: t("settings.scheduler.minutes", { count: 30 }) },
    { value: 60, label: t("settings.scheduler.hour") },
  ];

  const weekdayOptions = [
    { value: "monday" as Weekday, label: t("settings.scheduler.monday") },
    { value: "tuesday" as Weekday, label: t("settings.scheduler.tuesday") },
    { value: "wednesday" as Weekday, label: t("settings.scheduler.wednesday") },
    { value: "thursday" as Weekday, label: t("settings.scheduler.thursday") },
    { value: "friday" as Weekday, label: t("settings.scheduler.friday") },
    { value: "saturday" as Weekday, label: t("settings.scheduler.saturday") },
    { value: "sunday" as Weekday, label: t("settings.scheduler.sunday") },
  ];

  const scheduleTypeOptions = [
    { value: "interval" as ScheduleType, label: t("settings.scheduler.scheduleInterval") },
    { value: "daily" as ScheduleType, label: t("settings.scheduler.scheduleDaily") },
    { value: "weekly" as ScheduleType, label: t("settings.scheduler.scheduleWeekly") },
    { value: "monthly" as ScheduleType, label: t("settings.scheduler.scheduleMonthly") },
    { value: "advanced" as ScheduleType, label: t("settings.scheduler.scheduleAdvanced") },
  ];

  const monthDayOptions = Array.from({ length: 31 }, (_, i) => ({
    value: i + 1,
    label: t("settings.scheduler.dayOfMonth", { day: i + 1 }),
  }));

  const getStatusColor = (status: string) => {
    switch (status) {
      case "active":
        return "green";
      case "paused":
        return "orange";
      case "disabled":
        return "red";
      default:
        return "default";
    }
  };

  const getStatusText = (status: string) => {
    switch (status) {
      case "active":
        return t("settings.scheduler.taskActive");
      case "paused":
        return t("settings.scheduler.taskPaused");
      case "disabled":
        return t("settings.scheduler.taskDisabled");
      default:
        return status;
    }
  };

  const formatNextRun = (dateStr: string) => {
    try {
      const date = new Date(dateStr);
      return date.toLocaleString();
    } catch {
      return dateStr;
    }
  };

  const parseScheduleConfig = (task: ScheduledTask): Partial<TaskFormData> => {
    const config = task.schedule_config
      || {
        schedule_type: "interval",
        weekdays: [],
        time_ranges: [],
        interval_seconds: null,
        exclude_holidays: false,
        exclude_custom_dates: [],
        month_day: null,
      };
    const timeRanges = config.time_ranges?.map((tr: TimeRange) => ({
      start: dayjs().hour(tr.start_hour).minute(tr.start_minute),
      end: dayjs().hour(tr.end_hour).minute(tr.end_minute),
    })) || [{ start: dayjs().hour(9).minute(0), end: dayjs().hour(17).minute(0) }];

    return {
      schedule_type: config.schedule_type || "interval",
      weekdays: config.weekdays || [],
      time_ranges: timeRanges,
      exclude_holidays: config.exclude_holidays || false,
      exclude_custom_dates: config.exclude_custom_dates || [],
      month_day: config.month_day || null,
      interval_hours: config.interval_seconds ? config.interval_seconds / 3600 : 24,
    };
  };

  const serializeScheduleConfig = (values: TaskFormData): ScheduleConfig => {
    return {
      schedule_type: values.schedule_type,
      weekdays: values.weekdays || [],
      time_ranges: values.time_ranges?.map(tr => ({
        start_hour: tr.start.hour(),
        start_minute: tr.start.minute(),
        end_hour: tr.end.hour(),
        end_minute: tr.end.minute(),
      })) || [],
      interval_seconds: values.schedule_type === "interval" ? values.interval_hours * 3600 : null,
      exclude_holidays: values.exclude_holidays || false,
      exclude_custom_dates: values.exclude_custom_dates || [],
      month_day: values.month_day || null,
    };
  };

  const formatScheduleDescription = (task: ScheduledTask): string => {
    const config = task.schedule_config;
    if (!config) { return "-"; }

    switch (config.schedule_type) {
      case "interval":
        return t("settings.scheduler.intervalDesc", { hours: (config.interval_seconds || 86400) / 3600 });
      case "daily":
        return t("settings.scheduler.dailyDesc");
      case "weekly":
        const dayNames = (config.weekdays || []).map(w => t(`settings.scheduler.${w}`)).join(", ");
        return t("settings.scheduler.weeklyDesc", { days: dayNames || "-" });
      case "monthly":
        return t("settings.scheduler.monthlyDesc", { day: config.month_day || "-" });
      case "advanced":
        return t("settings.scheduler.advancedDesc");
      default:
        return "-";
    }
  };

  const openCreateModal = () => {
    setEditingTask(null);
    setSelectedTemplate(null);
    form.resetFields();
    form.setFieldsValue({
      schedule_type: "interval",
      interval_hours: 24,
      weekdays: ["monday", "tuesday", "wednesday", "thursday", "friday"],
      time_ranges: [{ start: dayjs().hour(9).minute(0), end: dayjs().hour(17).minute(0) }],
      exclude_holidays: false,
      exclude_custom_dates: [],
      month_day: 1,
    });
    setTaskModalOpen(true);
  };

  const openEditModal = (task: ScheduledTask) => {
    setEditingTask(task);
    setSelectedTemplate(null);
    const parsedConfig = parseScheduleConfig(task);
    form.setFieldsValue({
      name: task.name,
      description: task.description,
      ...parsedConfig,
    });
    setTaskModalOpen(true);
  };

  const handleTemplateSelect = (templateType: string) => {
    const template = taskTemplates.find(t => t.template_type === templateType);
    if (template) {
      setSelectedTemplate(templateType);
      const config = template.schedule_config;
      const timeRanges = config.time_ranges?.map((tr: TimeRange) => ({
        start: dayjs().hour(tr.start_hour).minute(tr.start_minute),
        end: dayjs().hour(tr.end_hour).minute(tr.end_minute),
      })) || [{ start: dayjs().hour(9).minute(0), end: dayjs().hour(17).minute(0) }];

      form.setFieldsValue({
        name: template.name,
        description: template.description,
        template_type: template.template_type,
        workflow_id: template.workflow_id,
        schedule_type: config.schedule_type,
        weekdays: config.weekdays || [],
        time_ranges: timeRanges,
        interval_hours: config.interval_seconds ? config.interval_seconds / 3600 : 24,
        exclude_holidays: config.exclude_holidays,
        exclude_custom_dates: config.exclude_custom_dates || [],
        month_day: config.month_day,
      });
    }
  };

  const handleCreateOrUpdateTask = async () => {
    try {
      const values = await form.validateFields();
      setLoading(true);

      const scheduleConfig = serializeScheduleConfig(values);

      if (editingTask) {
        const updatedTask: ScheduledTask = {
          ...editingTask,
          name: values.name,
          description: values.description,
          interval_seconds: scheduleConfig.interval_seconds,
          schedule_config: scheduleConfig,
        };
        await invoke("update_scheduled_task", { taskId: editingTask.id, task: updatedTask });
        message.success(t("settings.scheduler.updateTask") + " - OK");
      } else {
        const taskType = values.workflow_id ? "workflow" : "custom";
        await invoke("create_scheduled_task", {
          name: values.name,
          description: values.description,
          task_type: taskType,
          schedule_config: scheduleConfig,
          workflow_id: values.workflow_id,
        });
        message.success(t("settings.scheduler.createTask") + " - OK");
      }

      setTaskModalOpen(false);
      await loadCustomTasks();
    } catch (e) {
      console.error("Failed to save task:", e);
      message.error(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handlePauseTask = async (taskId: string) => {
    try {
      await invoke("pause_scheduled_task", { taskId });
      await loadCustomTasks();
      message.success(t("settings.scheduler.pauseTask") + " - OK");
    } catch (e) {
      console.error("Failed to pause task:", e);
    }
  };

  const handleResumeTask = async (taskId: string) => {
    try {
      await invoke("resume_scheduled_task", { taskId });
      await loadCustomTasks();
      message.success(t("settings.scheduler.resumeTask") + " - OK");
    } catch (e) {
      console.error("Failed to resume task:", e);
    }
  };

  const handleDeleteTask = async (taskId: string) => {
    try {
      await invoke("delete_scheduled_task", { taskId });
      await loadCustomTasks();
      message.success(t("settings.scheduler.deleteTask") + " - OK");
    } catch (e) {
      console.error("Failed to delete task:", e);
    }
  };

  // 手动执行任务
  const handleExecuteNow = async (taskId: string) => {
    try {
      setExecuting((prev) => ({ ...prev, [taskId]: true }));
      await invoke("execute_scheduled_task", { taskId });
      message.success(t("settings.scheduler.executed") + " - OK");
      await loadCustomTasks();
    } catch (e) {
      message.error(String(e));
    } finally {
      setExecuting((prev) => ({ ...prev, [taskId]: false }));
    }
  };

  // 加载执行历史
  const handleLoadHistory = async (taskId: string) => {
    const isExpanded = expandedHistory[taskId];
    if (isExpanded) {
      setExpandedHistory((prev) => ({ ...prev, [taskId]: false }));
      return;
    }
    try {
      const records = await invoke<ExecutionRecord[]>("get_task_execution_history", { taskId });
      setHistoryMap((prev) => ({ ...prev, [taskId]: records }));
      setExpandedHistory((prev) => ({ ...prev, [taskId]: true }));
    } catch (e) {
      console.warn("Failed to load history:", e);
    }
  };

  // 快速创建模板任务
  const handleQuickCreate = async (templateType: string) => {
    try {
      setLoading(true);
      const cmdMap: Record<string, string> = {
        daily_summary: "create_daily_summary_task",
        backup: "create_backup_task",
        cleanup: "create_cleanup_task",
      };
      const cmd = cmdMap[templateType];
      if (!cmd) {
        message.error("Unknown template type");
        return;
      }
      await invoke(cmd);
      message.success(t("settings.scheduler.quickCreated") + " - OK");
      await loadCustomTasks();
    } catch (e) {
      message.error(String(e));
    } finally {
      setLoading(false);
    }
  };

  // 刷新所有任务
  const handleRefreshAll = async () => {
    try {
      await invoke("load_scheduled_tasks_from_db");
      await loadCustomTasks();
      message.success(t("settings.scheduler.refreshed"));
    } catch (e) {
      message.error(String(e));
    }
  };

  return (
    <div>
      <SettingsGroup title={t("settings.scheduler.autoBackup")}>
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.scheduler.enabled")}</span>
          <Switch
            checked={backupSettings?.enabled ?? false}
            onChange={handleAutoBackupChange}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.scheduler.backupInterval")}</span>
          <div className="flex items-center gap-2">
            <InputNumber
              min={1}
              max={720}
              value={backupSettings?.intervalHours ?? 24}
              onChange={handleBackupIntervalChange}
              style={{ width: 80 }}
            />
            <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>
              {t("settings.scheduler.hoursUnit")}
            </span>
          </div>
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.scheduler.maxCount")}</span>
          <InputNumber
            min={1}
            max={100}
            value={backupSettings?.maxCount ?? 10}
            onChange={async (v) => {
              if (!backupSettings || !v) { return; }
              await updateBackupSettings({ ...backupSettings, maxCount: v });
            }}
            style={{ width: 80 }}
          />
        </div>
        {backupSettings?.enabled && (
          <>
            <Divider style={{ margin: "4px 0" }} />
            <div style={rowStyle} className="flex items-center justify-between">
              <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>
                {t("settings.scheduler.status")}
              </span>
              <Tag color="green">{t("settings.scheduler.running")}</Tag>
            </div>
          </>
        )}
      </SettingsGroup>

      <SettingsGroup title={t("settings.scheduler.webdavSync")}>
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.scheduler.enabled")}</span>
          <Switch
            checked={settings.webdav_sync_enabled ?? false}
            onChange={handleWebdavSyncChange}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.scheduler.syncInterval")}</span>
          <Select
            value={settings.webdav_sync_interval_minutes ?? 60}
            options={webdavIntervalOptions}
            onChange={handleWebdavIntervalChange}
            style={{ width: 120 }}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.scheduler.maxRemoteBackups")}</span>
          <InputNumber
            min={1}
            max={100}
            value={settings.webdav_max_remote_backups ?? 10}
            onChange={(v) => v && saveSettings({ webdav_max_remote_backups: v })}
            style={{ width: 80 }}
          />
        </div>
        {settings.webdav_sync_enabled && (
          <>
            <Divider style={{ margin: "4px 0" }} />
            <div style={rowStyle} className="flex items-center justify-between">
              <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>
                {t("settings.scheduler.status")}
              </span>
              <Tag color="green">{t("settings.scheduler.running")}</Tag>
            </div>
          </>
        )}
      </SettingsGroup>

      <SettingsGroup title={t("settings.scheduler.closedLoop")}>
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.scheduler.enabled")}</span>
          <Switch
            checked={settings.closed_loop_enabled ?? true}
            onChange={handleClosedLoopChange}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t("settings.scheduler.nudgeInterval")}</span>
          <Select
            value={settings.closed_loop_interval_minutes ?? 5}
            options={closedLoopIntervalOptions}
            onChange={handleClosedLoopIntervalChange}
            style={{ width: 120 }}
          />
        </div>
        {(settings.closed_loop_enabled ?? true) && (
          <>
            <Divider style={{ margin: "4px 0" }} />
            <div style={rowStyle} className="flex items-center justify-between">
              <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>
                {t("settings.scheduler.status")}
              </span>
              <Tag color="blue">{t("settings.scheduler.running")}</Tag>
            </div>
          </>
        )}
      </SettingsGroup>

      <SettingsGroup
        title={t("settings.scheduler.customTasks")}
        extra={
          <div className="flex items-center gap-2">
            <Button size="small" icon={<RefreshCw size={14} />} onClick={handleRefreshAll}>
              刷新
            </Button>
            <Button size="small" icon={<Zap size={14} />} onClick={() => handleQuickCreate("daily_summary")}>
              日报
            </Button>
            <Button size="small" icon={<Calendar size={14} />} onClick={() => handleQuickCreate("backup")}>
              备份
            </Button>
            <Button size="small" icon={<Rocket size={14} />} onClick={() => handleQuickCreate("cleanup")}>
              清理
            </Button>
            <Button type="primary" size="small" icon={<Plus size={14} />} onClick={openCreateModal}>
              {t("settings.scheduler.addTask")}
            </Button>
          </div>
        }
      >
        {customTasks.length === 0
          ? (
            <div style={{ textAlign: "center", padding: "20px 0", color: "var(--color-text-secondary)" }}>
              {t("settings.scheduler.noTasks")}
            </div>
          )
          : (
            customTasks.map((task) => (
              <div key={task.id}>
                <div style={rowStyle} className="flex items-center justify-between">
                  <div className="flex items-center gap-2 min-w-0">
                    <span
                      style={{ fontWeight: 500, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
                    >
                      {task.name}
                    </span>
                    <Tag color={getStatusColor(task.status)}>{getStatusText(task.status)}</Tag>
                    {task.last_result && (
                      <Tag color={task.last_result.success ? "green" : "red"} style={{ fontSize: 10 }}>
                        {task.last_result.success ? "✓" : "✗"} {task.last_result.duration_ms}ms
                      </Tag>
                    )}
                  </div>
                  <div className="flex items-center gap-1 flex-shrink-0">
                    <Button
                      type="text"
                      size="small"
                      icon={executing[task.id] ? <RefreshCw size={14} className="animate-spin" /> : <Play size={14} />}
                      onClick={() => handleExecuteNow(task.id)}
                      loading={executing[task.id]}
                      title="立即执行"
                    />
                    {task.status === "active"
                      ? (
                        <Button
                          type="text"
                          size="small"
                          icon={<Pause size={14} />}
                          onClick={() => handlePauseTask(task.id)}
                          title={t("settings.scheduler.pauseTask")}
                        />
                      )
                      : (
                        <Button
                          type="text"
                          size="small"
                          icon={<Play size={14} />}
                          onClick={() => handleResumeTask(task.id)}
                          title={t("settings.scheduler.resumeTask")}
                        />
                      )}
                    <Button
                      type="text"
                      size="small"
                      icon={<History size={14} />}
                      onClick={() => handleLoadHistory(task.id)}
                      title="执行历史"
                    />
                    <Button
                      type="text"
                      size="small"
                      icon={<Edit2 size={14} />}
                      onClick={() => openEditModal(task)}
                      title={t("settings.scheduler.editTask")}
                    />
                    <Popconfirm
                      title={t("settings.scheduler.deleteTaskConfirm")}
                      onConfirm={() => handleDeleteTask(task.id)}
                      okText="Yes"
                      cancelText="No"
                    >
                      <Button
                        type="text"
                        size="small"
                        danger
                        icon={<Trash2 size={14} />}
                        title={t("settings.scheduler.deleteTask")}
                      />
                    </Popconfirm>
                  </div>
                </div>
                {task.description && (
                  <div style={{ fontSize: 12, color: "var(--color-text-secondary)", marginBottom: 4 }}>
                    {task.description}
                  </div>
                )}
                <div style={{ fontSize: 12, color: "var(--color-text-secondary)", marginBottom: 4 }}>
                  <Clock size={12} style={{ display: "inline", marginRight: 4 }} />
                  <span>{t("settings.scheduler.schedule")}: {formatScheduleDescription(task)}</span>
                  <span style={{ marginLeft: 16 }}>
                    {t("settings.scheduler.nextRunAt")}: {formatNextRun(task.next_run_at)}
                  </span>
                </div>

                {/* 执行历史 */}
                {expandedHistory[task.id] && (
                  <div style={{ marginTop: 8, marginBottom: 8 }}>
                    <Text type="secondary" style={{ fontSize: 11 }}>执行历史</Text>
                    {historyMap[task.id]?.length === 0
                      ? <div style={{ fontSize: 11, color: "#888", padding: "4px 0" }}>暂无记录</div>
                      : (
                        <div style={{ maxHeight: 200, overflowY: "auto", marginTop: 4 }}>
                          {(historyMap[task.id] || []).slice(0, 20).map((rec) => (
                            <div
                              key={rec.id}
                              style={{
                                display: "flex",
                                alignItems: "center",
                                gap: 8,
                                padding: "3px 6px",
                                fontSize: 11,
                                borderBottom: "1px solid var(--color-border)",
                                backgroundColor: rec.success ? undefined : "#fff2f0",
                              }}
                            >
                              <Tag color={rec.success ? "green" : "red"} style={{ fontSize: 10, margin: 0 }}>
                                {rec.success ? "成功" : "失败"}
                              </Tag>
                              <span
                                style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
                              >
                                {rec.output || rec.error || "-"}
                              </span>
                              <span style={{ color: "#888", whiteSpace: "nowrap" }}>{rec.duration_ms}ms</span>
                              <span style={{ color: "#bbb", whiteSpace: "nowrap", fontSize: 10 }}>
                                {rec.started_at ? new Date(rec.started_at).toLocaleString() : "-"}
                              </span>
                            </div>
                          ))}
                        </div>
                      )}
                  </div>
                )}
                <Divider style={{ margin: "4px 0" }} />
              </div>
            ))
          )}
      </SettingsGroup>

      <Modal
        title={editingTask ? t("settings.scheduler.editTask") : t("settings.scheduler.addTask")}
        open={taskModalOpen}
        onOk={handleCreateOrUpdateTask}
        onCancel={() => setTaskModalOpen(false)}
        confirmLoading={loading}
        okText={editingTask ? t("settings.scheduler.updateTask") : t("settings.scheduler.createTask")}
        cancelText="Cancel"
        width={700}
      >
        {!editingTask && (
          <>
            <div style={{ marginBottom: 16 }}>
              <label style={{ fontWeight: 500, display: "block", marginBottom: 8 }}>
                {t("settings.scheduler.selectTemplate") || "Select Template"}
              </label>

              <div style={{ marginBottom: 12 }}>
                <div style={{ fontSize: 12, color: "#888", marginBottom: 6 }}>
                  {t("settings.scheduler.reportTemplates")}
                </div>
                <div style={{ display: "grid", gridTemplateColumns: "repeat(2, 1fr)", gap: 8 }}>
                  {taskTemplates.filter(t => !t.workflow_id).map((template) => (
                    <div
                      key={template.template_type}
                      onClick={() => handleTemplateSelect(template.template_type)}
                      style={{
                        padding: "10px",
                        border: selectedTemplate === template.template_type
                          ? "2px solid var(--color-primary)"
                          : "1px solid var(--color-border)",
                        borderRadius: 6,
                        cursor: "pointer",
                        backgroundColor: selectedTemplate === template.template_type
                          ? "var(--color-bg-tertiary)"
                          : "var(--color-bg-secondary)",
                        transition: "all 0.2s",
                      }}
                    >
                      <div style={{ fontWeight: 500, marginBottom: 2, fontSize: 13, color: "var(--color-text)" }}>
                        {t(`settings.scheduler.${template.template_type}`) || template.name}
                      </div>
                      <div style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>
                        {t(`settings.scheduler.${template.template_type}Desc`) || template.description}
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              <div>
                <div style={{ fontSize: 12, color: "var(--color-text-secondary)", marginBottom: 6 }}>
                  {t("settings.scheduler.workflowTemplates")}
                </div>
                <div style={{ display: "grid", gridTemplateColumns: "repeat(2, 1fr)", gap: 8 }}>
                  {taskTemplates.filter(t => t.workflow_id).map((template) => (
                    <div
                      key={template.template_type}
                      onClick={() => handleTemplateSelect(template.template_type)}
                      style={{
                        padding: "10px",
                        border: selectedTemplate === template.template_type
                          ? "2px solid var(--color-success)"
                          : "1px solid var(--color-border)",
                        borderRadius: 6,
                        cursor: "pointer",
                        backgroundColor: selectedTemplate === template.template_type
                          ? "var(--color-bg-tertiary)"
                          : "var(--color-bg-secondary)",
                        transition: "all 0.2s",
                      }}
                    >
                      <div style={{ fontWeight: 500, marginBottom: 2, fontSize: 13, color: "var(--color-text)" }}>
                        {t(`settings.scheduler.${template.template_type}`) || template.name}
                      </div>
                      <div style={{ fontSize: 11, color: "var(--color-text-secondary)" }}>
                        {t(`settings.scheduler.${template.template_type}Desc`) || template.description}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
            <Divider />
          </>
        )}
        <Form form={form} layout="vertical" style={{ marginTop: 16 }}>
          <Form.Item
            name="name"
            label={t("settings.scheduler.taskName")}
            rules={[{ required: true, message: "Please input task name" }]}
          >
            <Input placeholder={t("settings.scheduler.taskName")} />
          </Form.Item>
          <Form.Item
            name="description"
            label={t("settings.scheduler.taskDescription")}
          >
            <Input.TextArea placeholder={t("settings.scheduler.taskDescription")} rows={2} />
          </Form.Item>
          <Divider>{t("settings.scheduler.scheduleSettings")}</Divider>
          <Form.Item
            name="schedule_type"
            label={t("settings.scheduler.scheduleType")}
            rules={[{ required: true }]}
          >
            <Select options={scheduleTypeOptions} placeholder={t("settings.scheduler.selectScheduleType")} />
          </Form.Item>

          <Form.Item noStyle shouldUpdate={(_, values) => values.schedule_type === "interval"}>
            {({ getFieldValue }) =>
              getFieldValue("schedule_type") === "interval" && (
                <Form.Item
                  name="interval_hours"
                  label={t("settings.scheduler.intervalHours")}
                  rules={[{ required: true, message: "Please select interval" }]}
                >
                  <Select
                    options={[
                      { value: 1, label: t("settings.scheduler.hour") },
                      { value: 6, label: t("settings.scheduler.hours", { count: 6 }) },
                      { value: 12, label: t("settings.scheduler.hours", { count: 12 }) },
                      { value: 24, label: t("settings.scheduler.daily") },
                      { value: 168, label: t("settings.scheduler.weekly") },
                      { value: 720, label: t("settings.scheduler.monthly") },
                    ]}
                  />
                </Form.Item>
              )}
          </Form.Item>

          <Form.Item noStyle shouldUpdate={(_, values) => ["weekly", "advanced"].includes(values.schedule_type)}>
            {({ getFieldValue }) =>
              ["weekly", "advanced"].includes(getFieldValue("schedule_type")) && (
                <Form.Item
                  name="weekdays"
                  label={t("settings.scheduler.weekdays")}
                  rules={[{ required: true, message: "Please select weekdays" }]}
                >
                  <Checkbox.Group options={weekdayOptions} />
                </Form.Item>
              )}
          </Form.Item>

          <Form.Item
            noStyle
            shouldUpdate={(_, values) => ["daily", "weekly", "monthly", "advanced"].includes(values.schedule_type)}
          >
            {({ getFieldValue }) =>
              ["daily", "weekly", "monthly", "advanced"].includes(getFieldValue("schedule_type")) && (
                <Form.Item
                  name="time_ranges"
                  label={t("settings.scheduler.timeRanges")}
                  rules={[{ required: true, message: "Please add time range" }]}
                >
                  <Form.List name="time_ranges">
                    {(fields, { add, remove }) => (
                      <>
                        {fields.map(({ key, name, ...restField }, index) => (
                          <div key={key} style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 8 }}>
                            <TimePicker
                              format="HH:mm"
                              placeholder={t("settings.scheduler.startTime")}
                              {...restField}
                              style={{ flex: 1 }}
                            />
                            <span>-</span>
                            <TimePicker
                              format="HH:mm"
                              placeholder={t("settings.scheduler.endTime")}
                              {...restField}
                              style={{ flex: 1 }}
                            />
                            {index > 0 && (
                              <Button
                                type="text"
                                danger
                                onClick={() => remove(index)}
                              >
                                -
                              </Button>
                            )}
                          </div>
                        ))}
                        <Button
                          type="dashed"
                          onClick={() => add({ start: dayjs().hour(9).minute(0), end: dayjs().hour(17).minute(0) })}
                          block
                        >
                          + {t("settings.scheduler.addTimeRange")}
                        </Button>
                      </>
                    )}
                  </Form.List>
                </Form.Item>
              )}
          </Form.Item>

          <Form.Item noStyle shouldUpdate={(_, values) => values.schedule_type === "monthly"}>
            {({ getFieldValue }) =>
              getFieldValue("schedule_type") === "monthly" && (
                <Form.Item
                  name="month_day"
                  label={t("settings.scheduler.monthDay")}
                  rules={[{ required: true, message: "Please select day of month" }]}
                >
                  <Select options={monthDayOptions} placeholder={t("settings.scheduler.selectDayOfMonth")} />
                </Form.Item>
              )}
          </Form.Item>

          <Form.Item
            name="exclude_holidays"
            valuePropName="checked"
          >
            <Checkbox>{t("settings.scheduler.excludeHolidays")}</Checkbox>
          </Form.Item>

          <Form.Item
            name="exclude_custom_dates"
            label={t("settings.scheduler.excludeCustomDates")}
          >
            <Select
              mode="tags"
              placeholder={t("settings.scheduler.enterExcludeDates")}
              tokenSeparators={[","]}
            />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
