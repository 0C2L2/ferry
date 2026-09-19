// Thin typed wrappers around Tauri commands.
import { invoke } from "@tauri-apps/api/core";
import type {
  AppEntry,
  BackupLocation,
  BrowserBackup,
  DriveInfo,
  DriverEntry,
  FileToBackup,
  Manifest,
  MigrationProfile,
  OsSource,
  RestoreSummary,
  ScanResult,
} from "./types";

export const api = {
  listDrives: () => invoke<DriveInfo[]>("list_removable_drives"),
  listOsSources: () => invoke<OsSource[]>("list_os_sources"),
  scanFiles: (extraExcludes: string[], roots: string[]) =>
    invoke<ScanResult>("scan_user_files", { extraExcludes, roots }),
  listUsbFiles: (usbRoot: string) =>
    invoke<FileToBackup[]>("list_usb_backup_files", { usbRoot }),
  migrationProfile: (targetId: string) =>
    invoke<MigrationProfile>("migration_profile", { targetId }),
  copyFiles: (files: FileToBackup[], usbRoot: string) =>
    invoke<void>("copy_files_to_usb", { files, usbRoot }),
  verifyBackup: (files: FileToBackup[], usbRoot: string, skipped: [string, string][]) =>
    invoke<Manifest>("verify_backup", { files, usbRoot, skipped }),
  readManifest: (backupDir: string) => invoke<Manifest>("read_manifest", { backupDir }),
  encryptBackup: (usbRoot: string, password: string) =>
    invoke<void>("encrypt_backup", { usbRoot, password }),
  downloadOs: (sourceId: string, destDir: string) =>
    invoke<string>("download_os_image", { sourceId, destDir }),
  prepareUsb: (driveLetter: string, usbRoot: string) =>
    invoke<void>("prepare_usb", { driveLetter, usbRoot }),
  writeBootloader: (bootPartitionLetter: string) =>
    invoke<void>("write_bootloader", { bootPartitionLetter }),
  backupChromium: (usbRoot: string) =>
    invoke<BrowserBackup>("backup_chromium_data", { usbRoot }),
  backupFirefox: (usbRoot: string) =>
    invoke<BrowserBackup>("backup_firefox_data", { usbRoot }),
  exportWifi: (usbRoot: string) => invoke<number>("export_wifi_profiles", { usbRoot }),
  scanApps: () => invoke<AppEntry[]>("scan_installed_apps"),
  scanDrivers: () => invoke<DriverEntry[]>("scan_drivers"),
  resolveTiers: (apps: AppEntry[]) => invoke<AppEntry[]>("resolve_app_tiers", { apps }),
  installApp: (wingetId: string) => invoke<string>("install_app", { wingetId }),
  saveInventory: (usbRoot: string, appsJson: string, driversJson: string) =>
    invoke<void>("save_inventory", { usbRoot, appsJson, driversJson }),
  readInventory: (backupDir: string) =>
    invoke<{ apps: AppEntry[]; drivers: DriverEntry[] }>("read_inventory", { backupDir }),
  findBackup: () => invoke<BackupLocation | null>("find_backup_on_usb"),
  decryptBackup: (usbRoot: string, password: string) =>
    invoke<string>("decrypt_backup", { usbRoot, password, stagingDir: "" }),
  restoreFiles: (backupDir: string) =>
    invoke<RestoreSummary>("restore_files", { backupDir }),
  importWifi: (backupRoot: string) =>
    invoke<[number, number]>("import_wifi_profiles", { backupRoot }),
  uploadBackupB2: (
    usbRoot: string,
    keyId: string,
    appKey: string,
    bucketName: string,
  ) =>
    invoke<string>("upload_backup_b2", { usbRoot, keyId, appKey, bucketName }),
};
