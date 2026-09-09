export interface Transport { serial: string; kind: 'usb' | 'wifi'; state: string }
export interface Device { id: string; model: string; transports: Transport[] }
export interface DeviceInfo {
  model: string; androidVersion: string; batteryLevel: number | null; charging: boolean;
  storageTotal: number; storageUsed: number; storageAvailable: number;
}
export interface AppPackage { packageName: string; versionCode: string; system: boolean }
export interface AppDetails { packageName: string; versionName: string; versionCode: string; apkPaths: string[] }
export interface FileEntry { name: string; path: string; kind: 'directory' | 'file' | 'symlink' | 'other'; size: number; modifiedAt: number }
export type TaskKind = 'install' | 'uninstall' | 'upload' | 'download' | 'export' | 'mkdir' | 'rename' | 'delete';
export interface TaskRequest { device: string; kind: TaskKind; source?: string; destination?: string; packageName?: string }
export interface Task {
  id: string; device: string; kind: TaskKind; label: string;
  status: 'queued' | 'running' | 'success' | 'failed' | 'cancelled';
  detail: string; progress: number | null; createdAt: number;
}
