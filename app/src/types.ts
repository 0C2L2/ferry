// Shared shapes matching the Rust backend's serde output.
// Note: Tauri commands take camelCase args (usbRoot, driveLetter, ...),
// but returned structs keep their Rust snake_case field names.

export interface DriveInfo {
  drive_letter: string;
  model: string;
  /** Whole-disk capacity, not this partition's — erasing reclaims the disk. */
  total_bytes: number;
  free_bytes: number;
  is_removable: boolean;
  disk_number: number | null;
  /** >1 means the stick is already partitioned; all of it gets erased. */
  partition_count: number;
  /** One entry per visible partition, for the expandable picker. */
  partitions: PartitionInfo[];
}

export interface PartitionInfo {
  /** Drive letter without trailing backslash, e.g. "E:". */
  letter: string;
  /** Volume label, e.g. "FERRY_DATA". Empty when the volume has none. */
  label: string;
  /** Filesystem name, e.g. "exFAT", "FAT32". */
  filesystem: string;
  /** This partition's capacity (not the disk's). */
  total_bytes: number;
  /** This partition's free space. */
  free_bytes: number;
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

export interface CustomIso {
  path: string;
  filename: string;
  size_bytes: number;
}

export interface CustomIsoInfo {
  filename: string;
  size_bytes: number;
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
  /** OneDrive Files On-Demand placeholders: they are listed in `files`, but
   *  their contents live in the cloud, so copying them yields empty files. */
  cloud_only_count: number;
  cloud_only_bytes: number;
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

export interface WifiProfile {
  ssid: string;
  security: string;
  has_password: boolean;
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
