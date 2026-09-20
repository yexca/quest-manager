import { invoke, isTauri } from '@tauri-apps/api/core';
import type { AppDetails, AppPackage, Device, DeviceInfo, DevicePowerSettings, DevicePreferences, DeviceProfile, FileEntry, LocalApk, LocalObb, Task, TaskRequest } from './types';
import type { LightningCatalog, LightningRecommendation } from './types';
import type { DisconnectWirelessRequest, ReconnectWirelessRequest, ReconnectWirelessResult, WirelessRequest, WirelessResult, WirelessQrSnapshot } from './types';
import { lightningPackage, navigatorPackage, previewLightningCatalog } from './lightningState';

export const isDesktop = isTauri();
export const isPreview = !isDesktop && new URLSearchParams(location.search).get('preview') === '1';
const lightningPreview = isPreview ? new URLSearchParams(location.search).get('lightning') : null;
const previewLightningVersion = lightningPreview === 'installed' ? '1.1.0' : lightningPreview === 'current' ? '1.2.0' : null;
const gib = 1024 ** 3;
// Preview data is fictional and is never collected from a connected device.
const previewDevices: Device[] = [
  { id: 'DEMO-DEVICE-001', model: 'Quest 3', transports: [{ serial: 'DEMO-USB-001', kind: 'usb', state: 'device' }, { serial: 'DEMO-WIFI-001', kind: 'wifi', state: 'device' }, { serial: '192.0.2.10:37001', kind: 'wifi', state: 'device' }] },
  { id: 'DEMO-DEVICE-002', model: 'Quest 3S', transports: [{ serial: 'DEMO-USB-002', kind: 'usb', state: 'device' }] },
  { id: 'DEMO-DEVICE-003', model: 'Quest 2', transports: [{ serial: 'DEMO-USB-003', kind: 'usb', state: 'device' }] },
  { id: 'DEMO-DEVICE-004', model: 'Demo headset', transports: [{ serial: 'DEMO-USB-004', kind: 'usb', state: 'device' }] },
];
let previewDevicePreferences: DevicePreferences = {
  profiles: previewDevices.map(device => ({ id: device.id, displayName: device.model, model: device.model, connectionPreference: 'auto' })),
  autoSwitch: true,
};
const previewApps: AppPackage[] = [
  'com.example.orbit', 'com.example.rhythm', 'com.example.minigolf',
  'com.example.paint', 'com.example.puzzle', 'com.example.explorer',
].map((packageName, index) => ({ packageName, versionCode: '100', system: false,
  installer: index < 2 ? 'com.oculus.ocms' : index < 4 ? 'com.android.shell' : index === 4 ? 'com.example.installer' : null,
  apkPath: `/data/app/example/${packageName}/base.apk`,
}));
if (previewLightningVersion) {
  for (const packageName of [lightningPackage, navigatorPackage]) {
    previewApps.push({ packageName, versionCode: '100', system: false, installer: null, apkPath: '/data/app/example/base.apk' });
  }
}
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
  const optionalApp = previewLightningVersion && (packageName === lightningPackage || packageName === navigatorPackage);
  const paths = ['/data/app/example/base.apk', '/data/app/example/split_config.arm64_v8a.apk'];
  return {
    packageName, versionName: optionalApp ? previewLightningVersion : index === 1 ? '2.4.1' : '1.2.0', versionCode: '100', apkPaths: paths,
    apkFiles: paths.map((path, i) => ({ path, size: (i ? 24 : 156) * 1024 ** 2, modified: 1735732800 })), apkSize: 180 * 1024 ** 2,
    firstInstallTime: '2025-01-01 12:00:00', lastUpdateTime: '2025-02-10 09:30:00', installer: previewApps[index]?.installer ?? null,
    minSdk: '26', targetSdk: '34', primaryAbi: 'arm64-v8a', secondaryAbi: null, uid: '10123', androidUser: 0,
    enabled: index !== 4, stateFlags: index === 4 ? ['stopped'] : [],
    permissions: [
      { name: 'android.permission.INTERNET', granted: true, kind: 'Install' },
      { name: 'android.permission.RECORD_AUDIO', granted: false, kind: 'Runtime' },
      { name: 'com.example.permission.PLAY', granted: null, kind: 'Requested' },
    ],
    assets: { displayName: optionalApp ? `${packageName === lightningPackage ? 'Lightning Launcher' : 'Navigator service'} (preview)` : unavailable ? null : previewNames[index] ?? 'System Shell', iconDataUrl: index < 0 || unavailable ? null : '/preview-app.svg',
      vrFeatures: unavailable ? [] : ['android.hardware.vr.headtracking', 'org.khronos.openxr'], signingSchemes: unavailable ? [] : ['v2', 'v3'],
      certificateSha256: unavailable ? [] : [Array(32).fill('AB').join(':')], notes: unavailable ? ['App artwork is unavailable. A standard icon is shown.'] : [],
    },
  };
}

// Session cache owns deduplication so an invalidated read cannot be reused.
function details(device: string, packageName: string): Promise<AppDetails> {
  if (isPreview) return Promise.resolve(previewDetails(packageName));
  return call<AppDetails>('app_details', { device, package: packageName });
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktop) throw new Error('Open Quest Manager using run-dev.ps1 or the desktop app to connect to your headset.');
  return invoke<T>(command, args);
}

export const api = {
  wirelessConnection: (request: WirelessRequest): Promise<WirelessResult> => call('wireless_connection', { request }),
  disconnectWireless: (request: DisconnectWirelessRequest): Promise<void> => call('disconnect_wireless', { request }),
  reconnectWireless: (request: ReconnectWirelessRequest): Promise<ReconnectWirelessResult> => call('reconnect_wireless', { request }),
  startWirelessQr: (): Promise<WirelessQrSnapshot> => call('start_wireless_qr'),
  wirelessQrStatus: (id: string): Promise<WirelessQrSnapshot> => call('wireless_qr_status', { id }),
  cancelWirelessQr: (id: string): Promise<void> => call('cancel_wireless_qr', { id }),
  lightningReleases: (refresh = false): Promise<LightningCatalog> => isPreview ? Promise.resolve(previewLightningCatalog) : call('lightning_releases', { refresh }),
  lightningRecommendation: (tag: string): Promise<LightningRecommendation> => isPreview ? Promise.resolve({ launcherTag: tag, navigatorTag: `addons${tag}` }) : call('lightning_recommendation', { tag }),
  navigatorEnabled: (device: string): Promise<boolean> => isPreview ? Promise.resolve(false) : call('navigator_enabled', { device }),
  openLightningRepository: (): Promise<void> => isPreview ? Promise.resolve(void window.open('https://github.com/threethan/LightningLauncher', '_blank', 'noopener,noreferrer')) : call('open_lightning_repository'),
  openProjectRepository: (): Promise<void> => call('open_project_repository'),
  inspectApk: (source: string): Promise<LocalApk> => {
    if (!isPreview) return call('inspect_apk', { source });
    if (source.includes('Unreadable')) return Promise.reject(new Error('The APK package name could not be read.'));
    const unknown = source.includes('Unknown');
    const system = source.includes('System Shell');
    const versionCode = system ? '' : source.includes('update') ? '110' : source.includes('older') ? '90' : '100';
    const packageName = system ? 'com.example.systemshell' : unknown ? 'com.example.unknown' : 'com.example.orbit';
    return Promise.resolve({ packageName, versionName: system ? '' : versionCode === '110' ? '1.3.0' : versionCode === '90' ? '1.1.0' : '1.2.0', versionCode, size: (unknown || system ? 0.15 : 2.94) * gib, sourceStamp: 'DEMO-APK-STAMP', split: false, veritySigning: !unknown && !system,
      assets: { ...previewDetails(unknown ? 'com.example.explorer' : packageName).assets, notes: ['Preview uses APK default launcher resources. Quest language and launcher artwork may differ.'] } });
  },
  devices: (): Promise<Device[]> => isPreview ? Promise.resolve(new URLSearchParams(location.search).get('connection') === 'none' ? [] : new URLSearchParams(location.search).get('connection') === 'unauthorized' ? [{ id: 'DEMO-DEVICE-001', model: 'Quest 3', transports: [{ serial: 'DEMO-USB-001', kind: 'usb', state: 'unauthorized' }] }] : previewDevices) : call('list_devices'),
  devicePreferences: (): Promise<DevicePreferences> => isPreview ? Promise.resolve(structuredClone(previewDevicePreferences)) : call('device_preferences'),
  saveDeviceProfile: (profile: DeviceProfile): Promise<DevicePreferences> => {
    if (!isPreview) return call('save_device_profile', { profile });
    const duplicate = previewDevicePreferences.profiles.some(item => item.id !== profile.id && item.displayName.trim().toLowerCase() === profile.displayName.trim().toLowerCase());
    if (duplicate) return Promise.reject(new Error('That device name is already in use. Choose a different name.'));
    const profiles = previewDevicePreferences.profiles.some(item => item.id === profile.id)
      ? previewDevicePreferences.profiles.map(item => item.id === profile.id ? profile : item)
      : [...previewDevicePreferences.profiles, profile];
    previewDevicePreferences = { ...previewDevicePreferences, profiles };
    return Promise.resolve(structuredClone(previewDevicePreferences));
  },
  removeDeviceProfile: (id: string): Promise<DevicePreferences> => {
    if (!isPreview) return call('remove_device_profile', { id });
    const profiles = previewDevicePreferences.profiles.filter(profile => profile.id !== id);
    if (profiles.length === previewDevicePreferences.profiles.length) return Promise.reject(new Error('The saved device was not found.'));
    previewDevicePreferences = { ...previewDevicePreferences, profiles };
    return Promise.resolve(structuredClone(previewDevicePreferences));
  },
  setDeviceAutoSwitch: (enabled: boolean): Promise<DevicePreferences> => {
    if (!isPreview) return call('set_device_auto_switch', { enabled });
    previewDevicePreferences = { ...previewDevicePreferences, autoSwitch: enabled };
    return Promise.resolve(structuredClone(previewDevicePreferences));
  },
  inspectObbs: (sources: string[]): Promise<LocalObb[]> => isPreview
    ? Promise.resolve(sources.map((source, index) => ({ source, sourceStamp: 'DEMO-OBB-STAMP', name: source.split(/[\\/]/).pop() || source, size: (index + 1) * 128 * 1024 ** 2 })))
    : call('inspect_obb_files', { sources }),
  info: (device: string): Promise<DeviceInfo> => isPreview ? Promise.resolve({ model: previewDevices.find(item => item.transports.some(transport => transport.serial === device))?.model ?? 'Demo headset', androidVersion: '14', batteryLevel: 80, charging: true, storageTotal: 128 * gib, storageUsed: 64 * gib, storageAvailable: 64 * gib }) : call('device_info', { device }),
  powerSettings: (device: string): Promise<DevicePowerSettings> => {
    if (isPreview) {
      const mode = new URLSearchParams(location.search).get('stayawake');
      return Promise.resolve({ stayAwake: mode === 'unknown' ? null : mode !== 'off', raw: mode === 'unknown' ? 'unknown' : mode === 'off' ? '0' : '15' });
    }
    return call('device_power_settings', { device });
  },
  setStayAwake: (device: string, enabled: boolean): Promise<DevicePowerSettings> => {
    if (isPreview) return Promise.resolve({ stayAwake: enabled, raw: enabled ? '15' : '0' });
    return call('set_device_stay_awake', { device, enabled });
  },
  apps: (device: string, includeSystem: boolean): Promise<AppPackage[]> => isPreview ? Promise.resolve(includeSystem ? [...previewApps, { packageName: 'com.example.systemshell', versionCode: '100', system: true, installer: null, apkPath: '/system/app/ExampleShell/base.apk' }] : previewApps) : call('list_apps', { device, includeSystem }),
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
