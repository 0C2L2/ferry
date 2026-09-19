// Shared shapes matching the Rust backend's serde output.
// Note: Tauri commands take camelCase args (usbRoot, driveLetter, ...),
// but returned structs keep their Rust snake_case field names.

export interface DriveInfo {
  drive_letter: string;
  model: string;
  total_bytes: number;
  free_bytes: number;
  is_removable: boolean;
}

export interface UsbLayout {
  boot_letter: string;
  data_letter: string;
}

export type SourceType = "microsoft_mct" | "direct_url";

export interface OsSource {
  id: string;
  label: string;
  source_type: SourceType;
  url: string | null;
  checksum_url: string | null;
  approx_bytes: number | null;
}

export interface FileToBackup {
  source: string;
  relative: string;
  size_bytes: number;
}

export interface ScanResult {
  files: FileToBackup[];
  total_bytes: number;
  skipped_count: number;
  skipped_reasons: [string, string][];
}

export interface ManifestEntry {
  original_path: string;
  backup_path: string;
  size_bytes: number;
  sha256: string;
}

export interface SkippedEntry {
  path: string;
  reason: string;
}

export interface Manifest {
  ferry_version: string;
  created_at: string;
  source_user: string;
  source_os: string;
  files: ManifestEntry[];
  skipped: SkippedEntry[];
}

export interface AppEntry {
  name: string;
  version: string | null;
  publisher: string | null;
  url_info: string | null;
  tier: number;
  winget_id: string | null;
}

export interface DriverEntry {
  name: string;
  inf_name: string | null;
  provider: string | null;
  version: string | null;
  third_party: boolean;
}

export interface BrowserBackup {
  backed_up: string[];
  skipped: string[];
}

export interface BackupLocation {
  drive_letter: string;
  enc_path: string;
  salt_path: string;
}

export interface RestoreSummary {
  restored: number;
  skipped: number;
  failed: string[];
  wifi_restored: number;
  wifi_failed: number;
}

export interface ProgressPayload {
  stage: string;
  current: number;
  total: number;
  current_item: string;
}

export interface ProfileFolder {
  name: string;
  path: string;
  size_bytes: number;
  file_count: number;
  recommended: boolean;
}

export interface MigrationProfile {
  source_os: string;
  target_family: string;
  folders: ProfileFolder[];
  warnings: string[];
}

export interface AppSuggestion {
  name: string;
  suggestion: string;
  command: string | null;
}

export interface SandboxVerifyResult {
  success: boolean;
  output: string;
}

export interface ShareLink {
  subdomain: string;
  url: string;
}

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = bytes;
  let u = -1;
  do {
    v /= 1024;
    u++;
  } while (v >= 1024 && u < units.length - 1);
  return `${v.toFixed(1)} ${units[u]}`;
}

export function formatGB(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  return `${(bytes / 1e9).toFixed(1)} GB free`;
}
