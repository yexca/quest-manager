export interface Transport { serial: string; kind: 'usb' | 'wifi'; state: string }
export interface Device { id: string; model: string; transports: Transport[] }
export interface DeviceInfo {
  model: string; androidVersion: string; batteryLevel: number | null; charging: boolean;
  storageTotal: number; storageUsed: number; storageAvailable: number;
}
export interface AppPackage { packageName: string; versionCode: string; system: boolean }
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
  packageName: string; versionName: string; versionCode: string; size: number;
  sourceStamp: string; assets: AppAssets; split: boolean; veritySigning: boolean | null;
}
export interface InstallOptions {
  sourceStamp: string; displayName: string | null; iconPng: string | null; compatibility: boolean;
}
export interface TaskRequest { device: string; kind: TaskKind; source?: string; destination?: string; packageName?: string; installOptions?: InstallOptions }
export interface Task {
  id: string; device: string; kind: TaskKind; label: string;
  status: 'queued' | 'running' | 'success' | 'failed' | 'cancelled';
  detail: string; progress: number | null; createdAt: number;
}
