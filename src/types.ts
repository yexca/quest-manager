export interface Transport { serial: string; kind: 'usb' | 'wifi'; state: string }
export type ConnectionPreference = 'auto' | 'usb' | 'wifi';
export type WirelessRequest = { method: 'pair'; address: string; code: string } | { method: 'usb'; device: string };
export interface WirelessResult { serial: string | null; message: string }
export interface WirelessQrSnapshot {
  id: string;
  status: 'waiting' | 'pairing' | 'connecting' | 'connected' | 'failed' | 'cancelled' | 'expired';
  message: string;
  qrDataUrl: string | null;
  expiresAt: number;
  serial: string | null;
}
export interface Device { id: string; model: string; transports: Transport[]; displayName?: string; connectionPreference?: ConnectionPreference }
export interface DeviceProfile { id: string; displayName: string; model: string; connectionPreference: ConnectionPreference }
export interface DevicePreferences { profiles: DeviceProfile[]; autoSwitch: boolean }
export interface DeviceInfo {
  model: string; androidVersion: string; batteryLevel: number | null; charging: boolean;
  storageTotal: number; storageUsed: number; storageAvailable: number;
}
export interface DevicePowerSettings { stayAwake: boolean | null; raw: string }
export interface AppPackage { packageName: string; versionCode: string; system: boolean; installer: string | null; apkPath: string | null }
export interface AppAssets {
  displayName: string | null; iconDataUrl: string | null; vrFeatures: string[];
  signingSchemes: string[]; certificateSha256: string[]; notes: string[];
}
export interface AppDetails {
  packageName: string; versionName: string; versionCode: string; apkPaths: string[];
  apkFiles: { path: string; size: number | null; modified: number | null }[]; apkSize: number | null;
  firstInstallTime: string | null; lastUpdateTime: string | null; installer: string | null;
  minSdk: string | null; targetSdk: string | null; primaryAbi: string | null; secondaryAbi: string | null;
  uid: string | null; androidUser: number; enabled: boolean | null; stateFlags: string[];
  permissions: { name: string; granted: boolean | null; kind: string }[]; assets: AppAssets;
}
export interface FileEntry { name: string; path: string; kind: 'directory' | 'file' | 'symlink' | 'other'; size: number; modifiedAt: number }
export type TaskKind = 'install' | 'uninstall' | 'upload' | 'download' | 'export' | 'mkdir' | 'rename' | 'delete';
export interface LocalApk {
  packageName: string; versionName: string;
  /** Full manifest version code as decimal text, or empty when unavailable. */
  versionCode: string; size: number;
  sourceStamp: string; assets: AppAssets; split: boolean; veritySigning: boolean | null;
}
export interface InstallOptions {
  sourceStamp: string; displayName: string | null; iconPng: string | null; compatibility: boolean;
}
export interface LocalObb { source: string; sourceStamp: string; name: string; size: number }
export interface ObbInstall { apkSourceStamp: string; files: Pick<LocalObb, 'source' | 'sourceStamp'>[] }
export interface LightningSelection { launcherAsset: number | null; navigatorAsset: number | null }
export interface LightningRelease { tag: string; assetId: number; size: number; publishedAt: string; sha256: string | null }
export interface LightningCatalog { launchers: LightningRelease[]; navigators: LightningRelease[] }
export interface LightningRecommendation { launcherTag: string; navigatorTag: string | null }
export interface TaskRequest { device: string; kind: TaskKind; source?: string; destination?: string; packageName?: string; installOptions?: InstallOptions; obb?: ObbInstall; lightning?: LightningSelection }
export interface Task {
  /** Optional package refresh hint; never an installation target. */
  packageName?: string | null;
  apkInstalled?: boolean;
  includesObb?: boolean;
  id: string; revision: number; device: string; kind: TaskKind; label: string;
  status: 'queued' | 'running' | 'success' | 'failed' | 'cancelled';
  detail: string; progress: number | null; createdAt: number;
}
