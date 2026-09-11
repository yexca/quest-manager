import type { AppDetails, AppPackage, Task } from './types.ts';

export type InstallSource = 'meta' | 'sideload' | 'other' | 'unknown';
export const sourceLabels: Record<InstallSource, string> = {
  meta: 'Meta Store (inferred)', sideload: 'Sideloaded (inferred)', other: 'Other installer', unknown: 'Unknown source',
};
export function installSource(app: AppPackage): InstallSource {
  // Exact installer identities, never the installed application's package prefix.
  if (app.installer === 'com.oculus.ocms') return 'meta';
  if (app.installer === 'com.android.shell') return 'sideload';
  return app.installer ? 'other' : 'unknown';
}

export type AppMutation = Pick<Task, 'id' | 'device' | 'kind' | 'status' | 'packageName'>;
export function applicationChanges(tasks: AppMutation[], transports: string[], seen: Set<string>): Set<string> | null {
  const packages = new Set<string>();
  let all = false;
  for (const task of tasks) {
    if (task.status !== 'success' || !['install', 'uninstall'].includes(task.kind) || !transports.includes(task.device) || seen.has(task.id)) continue;
    seen.add(task.id);
    if (task.packageName) packages.add(task.packageName); else all = true;
  }
  return all ? null : packages;
}

type Entry = { app: AppPackage; details?: AppDetails; error?: string; stale: boolean; sideloaded?: boolean; pending?: Promise<AppDetails | null> };
const identity = (app: AppPackage) => JSON.stringify([app.versionCode, app.apkPath, app.installer, app.system]);

// One selected transport's session cache. Filters never mutate this inventory.
// Replacing entry objects invalidates in-flight reads without erasing artwork.
export function createApplicationCache() {
  let apps: AppPackage[] = [];
  let entries = new Map<string, Entry>();
  const installedHere = new Set<string>();
  return {
    installed(name: string) { installedHere.add(name); },
    reconcile(next: AppPackage[]) {
      const retained = new Map<string, Entry>();
      for (const app of next) {
        const previous = entries.get(app.packageName);
        const entry = previous && identity(previous.app) === identity(app)
          ? previous : { app, details: previous?.details, stale: true };
        if (installedHere.has(app.packageName)) entry.sideloaded = true;
        retained.set(app.packageName, entry);
      }
      entries = retained;
      apps = next;
      installedHere.clear();
    },
    invalidate(packages: Set<string> | null = null) {
      for (const [name, entry] of entries) {
        if (!packages || packages.has(name)) entries.set(name, { app: entry.app, details: entry.details, sideloaded: entry.sideloaded, stale: true });
      }
    },
    needsRead(name: string) { const entry = entries.get(name); return !!entry?.stale && !entry.error; },
    snapshot() {
      const metadata: Record<string, AppDetails> = {}, errors: Record<string, string> = {};
      const sources: Record<string, InstallSource> = {};
      for (const [name, entry] of entries) {
        if (entry.details) metadata[name] = entry.details;
        if (entry.error) errors[name] = entry.error;
        sources[name] = entry.sideloaded ? 'sideload' : installSource(entry.app);
      }
      return { apps, metadata, errors, sources };
    },
    async read(name: string, query: () => Promise<AppDetails>, force = false): Promise<AppDetails | null> {
      const entry = entries.get(name);
      if (!entry) return null;
      if (entry.pending) return entry.pending;
      if (!force && !entry.stale && entry.details) return entry.details;
      const pending = (async () => {
        try {
          const details = await Promise.resolve().then(query);
          if (entries.get(name) !== entry) return null;
          entry.details = details; entry.stale = false; entry.error = undefined;
          return details;
        } catch (cause) {
          if (entries.get(name) !== entry) return null;
          entry.error = cause instanceof Error ? cause.message : String(cause);
          throw cause;
        } finally { entry.pending = undefined; }
      })();
      entry.pending = pending;
      return pending;
    },
  };
}
