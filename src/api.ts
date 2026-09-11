import { invoke, isTauri } from '@tauri-apps/api/core';
import type { AppDetails, AppPackage, Device, DeviceInfo, FileEntry, LocalApk, Task, TaskRequest } from './types';

export const isDesktop = isTauri();
export const isPreview = !isDesktop && new URLSearchParams(location.search).get('preview') === '1';
const gib = 1024 ** 3;
// Preview data is fictional and is never collected from a connected device.
const previewApps: AppPackage[] = [
  'com.example.orbit', 'com.example.rhythm', 'com.example.minigolf',
  'com.example.paint', 'com.example.puzzle', 'com.example.explorer',
].map(packageName => ({ packageName, versionCode: '100', system: false }));
const modifiedAt = new Date('2025-01-01T12:00:00Z').getTime();
const entry = (parent: string, name: string, kind: FileEntry['kind'], size = 0): FileEntry => ({ name, path: `${parent}/${name}`, kind, size, modifiedAt });
const previewNames = ['Orbit Adventures', 'Rhythm Studio', 'Pocket Minigolf', 'Open Canvas', 'Quiet Puzzles', 'World Explorer'];
const previewTasks: Task[] = [
  { id: 'EXAMPLE-TASK-1', revision: 2, device: 'DEMO-USB-001', kind: 'upload', label: 'Upload example-movie.mp4', status: 'running', detail: 'Sample transfer progress. No device operation is running.', progress: 42, createdAt: modifiedAt + 3 },
  { id: 'EXAMPLE-TASK-2', revision: 0, device: 'DEMO-WIFI-001', kind: 'install', label: 'Install Example Adventure.apk', status: 'queued', detail: 'Sample queued installation.', progress: null, createdAt: modifiedAt + 2 },
  { id: 'EXAMPLE-TASK-3', revision: 3, device: 'DEMO-DISCONNECTED-002', kind: 'download', label: 'Download example-notes.txt', status: 'failed', detail: 'Sample connection failure. Reconnect the original target before starting a new task.', progress: null, createdAt: modifiedAt + 1 },
  { id: 'EXAMPLE-TASK-4', revision: 3, device: 'DEMO-USB-001', kind: 'export', label: 'Export com.example.orbit', status: 'success', detail: 'Sample completed export.', progress: 100, createdAt: modifiedAt },
];
function previewDetails(packageName: string): AppDetails {
  const index = previewApps.findIndex(app => app.packageName === packageName);
  const unavailable = index === 5;
  const paths = ['/data/app/example/base.apk', '/data/app/example/split_config.arm64_v8a.apk'];
  return {
    packageName, versionName: index === 1 ? '2.4.1' : '1.2.0', versionCode: '100', apkPaths: paths,
    apkFiles: paths.map((path, i) => ({ path, size: (i ? 24 : 156) * 1024 ** 2, modified: 1735732800 })), apkSize: 180 * 1024 ** 2,
    firstInstallTime: '2025-01-01 12:00:00', lastUpdateTime: '2025-02-10 09:30:00', installer: unavailable ? null : 'com.example.store',
    minSdk: '26', targetSdk: '34', primaryAbi: 'arm64-v8a', secondaryAbi: null, uid: '10123', androidUser: 0,
    enabled: index !== 4, stateFlags: index === 4 ? ['stopped'] : [],
    permissions: [
      { name: 'android.permission.INTERNET', granted: true, kind: 'Install' },
      { name: 'android.permission.RECORD_AUDIO', granted: false, kind: 'Runtime' },
      { name: 'com.example.permission.PLAY', granted: null, kind: 'Requested' },
    ],
    assets: { displayName: unavailable ? null : previewNames[index] ?? 'System Shell', iconDataUrl: index < 0 || unavailable ? null : '/preview-app.svg',
      vrFeatures: unavailable ? [] : ['android.hardware.vr.headtracking', 'org.khronos.openxr'], signingSchemes: unavailable ? [] : ['v2', 'v3'],
      certificateSha256: unavailable ? [] : [Array(32).fill('AB').join(':')], notes: unavailable ? ['App artwork is unavailable. A standard icon is shown.'] : [],
    },
  };
}

// A row and its open dialog can share a request without scheduling duplicate reads.
const pendingDetails = new Map<string, Promise<AppDetails>>();
function details(device: string, packageName: string): Promise<AppDetails> {
  if (isPreview) return Promise.resolve(previewDetails(packageName));
  const key = JSON.stringify([device, packageName]);
  const pending = pendingDetails.get(key);
  if (pending) return pending;
  const request = call<AppDetails>('app_details', { device, package: packageName }).finally(() => pendingDetails.delete(key));
  pendingDetails.set(key, request);
  return request;
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktop) throw new Error('Open Quest Manager using run-dev.ps1 or the desktop app to connect to your headset.');
  return invoke<T>(command, args);
}

export const api = {
  openProjectRepository: (): Promise<void> => call('open_project_repository'),
  inspectApk: (source: string): Promise<LocalApk> => {
    if (!isPreview) return call('inspect_apk', { source });
    const unknown = source.includes('Unknown');
    return Promise.resolve({ packageName: unknown ? 'com.example.unknown' : 'com.example.orbit', versionName: '1.2.0', versionCode: '100', size: (unknown ? 0.15 : 2.94) * gib, sourceStamp: 'DEMO-APK-STAMP', split: false, veritySigning: !unknown,
      assets: { ...previewDetails(unknown ? 'com.example.explorer' : 'com.example.orbit').assets, notes: ['Preview uses APK default launcher resources. Quest language and launcher artwork may differ.'] } });
  },
  devices: (): Promise<Device[]> => isPreview ? Promise.resolve([{ id: 'DEMO-DEVICE-001', model: 'Demo headset', transports: [{ serial: 'DEMO-USB-001', kind: 'usb', state: 'device' }, { serial: 'DEMO-WIFI-001', kind: 'wifi', state: 'device' }] }]) : call('list_devices'),
  info: (device: string): Promise<DeviceInfo> => isPreview ? Promise.resolve({ model: 'Demo headset', androidVersion: '14', batteryLevel: 80, charging: true, storageTotal: 128 * gib, storageUsed: 64 * gib, storageAvailable: 64 * gib }) : call('device_info', { device }),
  apps: (device: string, includeSystem: boolean): Promise<AppPackage[]> => isPreview ? Promise.resolve(includeSystem ? [...previewApps, { packageName: 'com.example.systemshell', versionCode: '100', system: true }] : previewApps) : call('list_apps', { device, includeSystem }),
  details,
  clearMetadataCache: (): Promise<void> => isPreview ? Promise.resolve() : call('clear_metadata_cache'),
  files: (device: string, path: string): Promise<FileEntry[]> => {
    if (!isPreview) return call('list_files', { device, path });
    if (path === '/sdcard') return Promise.resolve(['Android', 'DCIM', 'Download', 'Movies', 'Music', 'Oculus', 'Pictures'].map(name => entry(path, name, 'directory')));
    if (path === '/sdcard/Android') return Promise.resolve(['data', 'obb'].map(name => entry(path, name, 'directory')));
    if (path === '/sdcard/Download') return Promise.resolve([entry(path, 'Sample experience.apk', 'file', 156 * 1024 ** 2), entry(path, 'Example notes.txt', 'file', 1200)]);
    if (path === '/sdcard/Movies') return Promise.resolve([entry(path, 'Sample video.mp4', 'file', 2.4 * gib)]);
    return Promise.resolve([]);
  },
  tasks: (): Promise<Task[]> => isPreview ? Promise.resolve(previewTasks) : call('list_tasks'),
  clearCompletedTasks: (): Promise<string[]> => call('clear_completed_tasks'),
  exitWithActiveTasks: (): Promise<void> => call('exit_with_active_tasks'),
  start: (request: TaskRequest): Promise<Task> => call('start_task', { request }),
  cancel: (id: string): Promise<void> => call('cancel_task', { id }),
};
