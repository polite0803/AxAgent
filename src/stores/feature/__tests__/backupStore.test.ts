// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@/lib/invoke", () => ({
  invoke: invokeMock,
  isTauri: () => false,
}));

import { useBackupStore } from "@/stores/feature/backupStore";

const BACKUP_ID = "backup-1";

function makeBackup(overrides?: Record<string, unknown>) {
  return {
    id: BACKUP_ID,
    version: "1.0",
    createdAt: "2025-01-01T00:00:00Z",
    encrypted: false,
    checksum: "abc123",
    objectCountsJson: "{}",
    sourceAppVersion: "2.0",
    filePath: "/tmp/backup.db",
    fileSize: 1024,
    ...overrides,
  };
}

function makeBackupSettings(overrides?: Record<string, unknown>) {
  return {
    enabled: true,
    intervalHours: 24,
    maxCount: 10,
    backupDir: null,
    ...overrides,
  };
}

describe("backupStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useBackupStore.setState({
      backups: [],
      loading: false,
      error: null,
      selectedIds: [],
      backupSettings: null,
    });
  });

  describe("loadBackups", () => {
    it("loads backups from backend", async () => {
      const backups = [makeBackup(), makeBackup({ id: "backup-2" })];
      invokeMock.mockResolvedValueOnce(backups);

      await useBackupStore.getState().loadBackups();

      expect(invokeMock).toHaveBeenCalledWith("list_backups");
      expect(useBackupStore.getState().backups).toEqual(backups);
      expect(useBackupStore.getState().loading).toBe(false);
    });

    it("sets error on failure", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Network error"));

      await useBackupStore.getState().loadBackups();

      expect(useBackupStore.getState().error).toBe("Error: Network error");
      expect(useBackupStore.getState().loading).toBe(false);
    });
  });

  describe("createBackup", () => {
    it("creates a backup and reloads list", async () => {
      const backup = makeBackup();
      invokeMock.mockResolvedValueOnce(backup);
      invokeMock.mockResolvedValueOnce([backup]);

      const result = await useBackupStore.getState().createBackup();

      expect(invokeMock).toHaveBeenCalledWith("create_backup", { format: "json" });
      expect(result).toEqual(backup);
      expect(useBackupStore.getState().backups).toContainEqual(backup);
    });

    it("creates a backup with custom format", async () => {
      const backup = makeBackup({ format: "zip" });
      invokeMock.mockResolvedValueOnce(backup);
      invokeMock.mockResolvedValueOnce([backup]);

      const result = await useBackupStore.getState().createBackup("zip");

      expect(invokeMock).toHaveBeenCalledWith("create_backup", { format: "zip" });
      expect(result).toEqual(backup);
    });

    it("returns null on failure", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Disk full"));

      const result = await useBackupStore.getState().createBackup();

      expect(result).toBeNull();
      expect(useBackupStore.getState().error).toBe("Error: Disk full");
    });
  });

  describe("restoreBackup", () => {
    it("restores a backup", async () => {
      const report = { restored_items: 5, errors: [] };
      invokeMock.mockResolvedValueOnce(report);

      const result = await useBackupStore.getState().restoreBackup(BACKUP_ID);

      expect(invokeMock).toHaveBeenCalledWith("restore_backup", {
        backupId: BACKUP_ID,
        strategy: null,
      });
      expect(result).toEqual(report);
      expect(useBackupStore.getState().loading).toBe(false);
    });

    it("restores a backup with strategy", async () => {
      invokeMock.mockResolvedValueOnce({});

      await useBackupStore.getState().restoreBackup(BACKUP_ID, "merge");

      expect(invokeMock).toHaveBeenCalledWith("restore_backup", {
        backupId: BACKUP_ID,
        strategy: "merge",
      });
    });
  });

  describe("deleteBackup", () => {
    it("deletes a backup and removes from store", async () => {
      const backup = makeBackup();
      useBackupStore.setState({ backups: [backup], selectedIds: [BACKUP_ID] });

      invokeMock.mockResolvedValueOnce(undefined);

      await useBackupStore.getState().deleteBackup(BACKUP_ID);

      expect(invokeMock).toHaveBeenCalledWith("delete_backup", { backupId: BACKUP_ID });
      expect(useBackupStore.getState().backups).toHaveLength(0);
      expect(useBackupStore.getState().selectedIds).toHaveLength(0);
    });
  });

  describe("batchDeleteBackups", () => {
    it("batch deletes backups and clears selection", async () => {
      const backup1 = makeBackup({ id: "backup-1" });
      const backup2 = makeBackup({ id: "backup-2" });
      const backup3 = makeBackup({ id: "backup-3" });
      useBackupStore.setState({
        backups: [backup1, backup2, backup3],
        selectedIds: ["backup-1", "backup-2"],
      });

      invokeMock.mockResolvedValueOnce(undefined);

      await useBackupStore.getState().batchDeleteBackups(["backup-1", "backup-2"]);

      expect(invokeMock).toHaveBeenCalledWith("batch_delete_backups", {
        backupIds: ["backup-1", "backup-2"],
      });
      expect(useBackupStore.getState().backups).toHaveLength(1);
      expect(useBackupStore.getState().backups[0].id).toBe("backup-3");
      expect(useBackupStore.getState().selectedIds).toHaveLength(0);
    });
  });

  describe("setSelectedIds", () => {
    it("sets selected IDs", () => {
      useBackupStore.getState().setSelectedIds(["backup-1", "backup-2"]);
      expect(useBackupStore.getState().selectedIds).toEqual(["backup-1", "backup-2"]);

      useBackupStore.getState().setSelectedIds([]);
      expect(useBackupStore.getState().selectedIds).toEqual([]);
    });
  });

  describe("loadBackupSettings", () => {
    it("loads backup settings", async () => {
      const settings = makeBackupSettings();
      invokeMock.mockResolvedValueOnce(settings);

      await useBackupStore.getState().loadBackupSettings();

      expect(invokeMock).toHaveBeenCalledWith("get_backup_settings");
      expect(useBackupStore.getState().backupSettings).toEqual(settings);
    });
  });

  describe("updateBackupSettings", () => {
    it("updates backup settings", async () => {
      const settings = makeBackupSettings({ interval_hours: 12 });
      invokeMock.mockResolvedValueOnce(undefined);

      await useBackupStore.getState().updateBackupSettings(settings);

      expect(invokeMock).toHaveBeenCalledWith("update_backup_settings", {
        backupSettings: settings,
      });
      expect(useBackupStore.getState().backupSettings).toEqual(settings);
    });
  });
});
