import type { AppPackage } from './types.ts';

export type InstalledSnapshot =
  | { key: string; status: 'loading' }
  | { key: string; status: 'ready'; apps: AppPackage[] }
  | { key: string; status: 'error'; error: string };

// One complete list per review refresh, independent of the Applications filter.
export function observeInstalledApps(
  key: string, device: string, read: (device: string, includeSystem: boolean) => Promise<AppPackage[]>,
  publish: (snapshot: InstalledSnapshot) => void,
) {
  let disposed = false;
  publish({ key, status: 'loading' });
  void (async () => {
    try {
      const apps = await read(device, true);
      if (!disposed) publish({ key, status: 'ready', apps });
    } catch (cause) {
      if (!disposed) publish({ key, status: 'error', error: cause instanceof Error ? cause.message : String(cause) });
    }
  })();
  return () => { disposed = true; };
}

export function compareVersions(apk: string, installed: string): 'newer' | 'same' | 'older' | 'unknown' {
  if (!/^\d+$/.test(apk) || !/^\d+$/.test(installed)) return 'unknown';
  const local = BigInt(apk), remote = BigInt(installed);
  return local > remote ? 'newer' : local < remote ? 'older' : 'same';
}

export function installationStatus(packageName: string | undefined, versionCode: string, device: string, key: string, snapshot: InstalledSnapshot | null) {
  if (!device) return { label: 'Connect a headset to check', attention: false };
  if (!packageName) return { label: 'Installation status unavailable', attention: false };
  if (!snapshot || snapshot.key !== key || snapshot.status === 'loading') return { label: 'Checking headset…', attention: false };
  if (snapshot.status === 'error') return { label: 'Installation status unavailable', attention: true };
  const installed = snapshot.apps.find(app => app.packageName === packageName);
  if (!installed) return { label: 'Not installed on this headset', attention: false };
  const comparison = compareVersions(versionCode, installed.versionCode);
  const labels = { newer: 'Newer APK selected', same: 'Same version code', older: 'Older APK selected', unknown: 'Version comparison unavailable' };
  return { label: `Already installed · ${labels[comparison]}`, attention: comparison === 'older', installed, comparison };
}

export function duplicatePackages(items: { details?: { packageName: string } }[]): Set<string> {
  const seen = new Set<string>(), duplicates = new Set<string>();
  for (const item of items) {
    const name = item.details?.packageName;
    if (!name) continue;
    if (seen.has(name)) duplicates.add(name);
    seen.add(name);
  }
  return duplicates;
}
