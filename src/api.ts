import { invoke, isTauri } from '@tauri-apps/api/core';
import type { AppDetails, AppPackage, Device, DeviceInfo, FileEntry, Task, TaskRequest } from './types';

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

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktop) throw new Error('Open Quest Manager using run-dev.ps1 or the desktop app to connect to your headset.');
  return invoke<T>(command, args);
}

export const api = {
  devices: (): Promise<Device[]> => isPreview ? Promise.resolve([{ id: 'DEMO-DEVICE-001', model: 'Demo headset', transports: [{ serial: 'DEMO-USB-001', kind: 'usb', state: 'device' }, { serial: 'DEMO-WIFI-001', kind: 'wifi', state: 'device' }] }]) : call('list_devices'),
  info: (device: string): Promise<DeviceInfo> => isPreview ? Promise.resolve({ model: 'Demo headset', androidVersion: '14', batteryLevel: 80, charging: true, storageTotal: 128 * gib, storageUsed: 64 * gib, storageAvailable: 64 * gib }) : call('device_info', { device }),
  apps: (device: string, includeSystem: boolean): Promise<AppPackage[]> => isPreview ? Promise.resolve(includeSystem ? [...previewApps, { packageName: 'com.example.systemshell', versionCode: '100', system: true }] : previewApps) : call('list_apps', { device, includeSystem }),
  details: (device: string, packageName: string): Promise<AppDetails> => isPreview ? Promise.resolve({ packageName, versionName: '1.0.0', versionCode: '100', apkPaths: ['/data/app/example/base.apk'] }) : call('app_details', { device, package: packageName }),
  files: (device: string, path: string): Promise<FileEntry[]> => {
    if (!isPreview) return call('list_files', { device, path });
    if (path === '/sdcard') return Promise.resolve(['Android', 'DCIM', 'Download', 'Movies', 'Music', 'Oculus', 'Pictures'].map(name => entry(path, name, 'directory')));
    if (path === '/sdcard/Android') return Promise.resolve(['data', 'obb'].map(name => entry(path, name, 'directory')));
    if (path === '/sdcard/Download') return Promise.resolve([entry(path, 'Sample experience.apk', 'file', 156 * 1024 ** 2), entry(path, 'Example notes.txt', 'file', 1200)]);
    if (path === '/sdcard/Movies') return Promise.resolve([entry(path, 'Sample video.mp4', 'file', 2.4 * gib)]);
    return Promise.resolve([]);
  },
  tasks: (): Promise<Task[]> => isPreview ? Promise.resolve([]) : call('list_tasks'),
  start: (request: TaskRequest): Promise<Task> => call('start_task', { request }),
  cancel: (id: string): Promise<void> => call('cancel_task', { id }),
};
