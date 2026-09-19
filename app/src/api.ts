// Thin typed wrappers around Tauri commands.
import { invoke } from "@tauri-apps/api/core";
import type {
  AppEntry,
  AppSuggestion,
  BackupLocation,
  BrowserBackup,
  DriveInfo,
  DriverEntry,
  FileToBackup,
  Manifest,
  MigrationProfile,
  OsSource,
  RestoreSummary,
  SandboxVerifyResult,
  ScanResult,
  ShareLink,
  UsbLayout,
} from "./types";

// Ferry Assist: an optional sidecar (see ../assist-server) for AI migration
// suggestions and Cloud Backup share links. Unset by default — every other
// Ferry feature works fully offline without it.
const ASSIST_SERVER_URL = import.meta.env.VITE_ASSIST_SERVER_URL;

export function isAssistConfigured(): boolean {
  return Boolean(ASSIST_SERVER_URL);
}

async function assistFetch<T>(path: string, body: unknown): Promise<T> {
  if (!ASSIST_SERVER_URL) throw new Error("Ferry Assist server is not configured");
  const res = await fetch(`${ASSIST_SERVER_URL}${path}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  const data = await res.json();
  if (!res.ok) throw new Error(data?.error ?? `Ferry Assist returned ${res.status}`);
  return data as T;
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
  prepareUsb: (driveLetter: string) => invoke<UsbLayout>("prepare_usb", { driveLetter }),
  writeBootloader: (bootLetter: string, dataLetter: string, isoFilename: string) =>
    invoke<void>("write_bootloader", { bootLetter, dataLetter, isoFilename }),
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
  uploadBackupB2: (usbRoot: string) => invoke<string>("upload_backup_b2", { usbRoot }),
  downloadCloudBackup: (backupId: string) => invoke<string>("download_backup_b2", { backupId }),
  deleteCloudBackup: (backupId: string) => invoke<void>("delete_cloud_backup", { backupId }),
};

export const assistApi = {
  suggestForApps: (apps: Pick<AppEntry, "name" | "publisher" | "tier">[], targetFamily: string) =>
    assistFetch<{ suggestions: AppSuggestion[] }>("/api/assist/suggest", { apps, targetFamily }),
  verifyCommand: (command: string) =>
    assistFetch<SandboxVerifyResult>("/api/assist/verify", { command }),
  createShareLink: (backupId: string) =>
    assistFetch<ShareLink>("/api/cloud/share-link", { backupId }),
};
