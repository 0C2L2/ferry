// Thin typed wrappers around Tauri commands.
import { invoke } from "@tauri-apps/api/core";
import type {
  AppEntry,
  BackupLocation,
  BrowserBackup,
  CustomIsoInfo,
  DriveInfo,
  DriverEntry,
  FileToBackup,
  Manifest,
  MigrationProfile,
  OsSource,
  RestoreSummary,
  ScanResult,
  UsbLayout,
  WifiProfile,
} from "./types";

// Ferry's server (../assist-server) mints the scoped keys for Cloud Backup.
// Unset by default — every other Ferry feature works fully offline without it.
const ASSIST_SERVER_URL = import.meta.env.VITE_ASSIST_SERVER_URL;

export function isAssistConfigured(): boolean {
  return Boolean(ASSIST_SERVER_URL);
}

/** Configured AND answering. Offering Cloud Backup when the server is down
 *  would only show the user a button that fails with a network error. */
export async function isCloudReachable(): Promise<boolean> {
  if (!ASSIST_SERVER_URL) return false;
  try {
    const res = await fetch(`${ASSIST_SERVER_URL}/ready`, { signal: AbortSignal.timeout(3000) });
    return res.ok;
  } catch {
    return false;
  }
}

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
  inspectCustomIso: (path: string) =>
    invoke<CustomIsoInfo>("inspect_custom_iso", { path }),
  copyCustomIso: (sourcePath: string, destDir: string) =>
    invoke<string>("copy_custom_iso", { sourcePath, destDir }),
  prepareUsb: (driveLetter: string, bootMb: number) =>
    invoke<UsbLayout>("prepare_usb", { driveLetter, bootMb }),
  writeBootloader: (bootLetter: string, dataLetter: string, isoFilename: string) =>
    invoke<void>("write_bootloader", { bootLetter, dataLetter, isoFilename }),
  backupChromium: (usbRoot: string) =>
    invoke<BrowserBackup>("backup_chromium_data", { usbRoot }),
  backupFirefox: (usbRoot: string) =>
    invoke<BrowserBackup>("backup_firefox_data", { usbRoot }),
  copyRestoreTool: (usbRoot: string) => invoke<void>("copy_restore_tool", { usbRoot }),
  exportWifi: (usbRoot: string) => invoke<number>("export_wifi_profiles", { usbRoot }),
  listWifiProfiles: (backupRoot: string) =>
    invoke<WifiProfile[]>("list_wifi_profiles", { backupRoot }),
  wifiPassword: (backupRoot: string, ssid: string) =>
    invoke<string>("wifi_password", { backupRoot, ssid }),
  scanApps: () => invoke<AppEntry[]>("scan_installed_apps"),
  scanNetworkAdapters: () => invoke<DriverEntry[]>("scan_network_adapters"),
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
  uploadBackupB2: (usbRoot: string) => invoke<string>("upload_backup_b2", { usbRoot }),
  downloadCloudBackup: (backupId: string) => invoke<string>("download_backup_b2", { backupId }),
  deleteCloudBackup: (backupId: string) => invoke<void>("delete_cloud_backup", { backupId }),
};

