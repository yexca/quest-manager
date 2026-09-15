import { useEffect, useMemo, useRef, useState } from 'react';
import { api } from './api';
import { applicationChanges, createApplicationCache, type AppMutation } from './applicationState';
import type { AppPackage } from './types';
import { taskChangedDevice } from './taskState';

export function useApplications(device: string, transports: string[], active: boolean, includeSystem: boolean, refresh: number, revision: number, tasks: AppMutation[], fail: (error: unknown) => void) {
  const cache = useMemo(() => createApplicationCache(), [device]);
  const currentCache = useRef(cache);
  currentCache.current = cache;
  const [view, setView] = useState(() => ({ device, ...cache.snapshot() }));
  const [loadingApps, setLoadingApps] = useState(false);
  const [inventory, setInventory] = useState({ device: '', valid: false });
  const [loadingMetadata, setLoadingMetadata] = useState(false);
  const [metadataPaused, setMetadataPaused] = useState(false);
  const latest = useRef({ tasks, transports });
  latest.current = { tasks, transports };
  const previousRefresh = useRef(refresh);
  const seen = useRef(new Set<string>());
  const publish = () => { if (currentCache.current === cache) setView({ device, ...cache.snapshot() }); };

  useEffect(() => { setMetadataPaused(false); }, [device]);
  useEffect(() => {
    let alive = true;
    if (!device) { setLoadingApps(false); return; }
    const explicit = previousRefresh.current !== refresh;
    previousRefresh.current = refresh;
    for (const task of latest.current.tasks) {
      if (taskChangedDevice(task) && !seen.current.has(task.id) && latest.current.transports.includes(task.device) && task.kind === 'install' && task.packageName) cache.installed(task.packageName);
    }
    const affected = applicationChanges(latest.current.tasks, latest.current.transports, seen.current);
    cache.invalidate(explicit ? null : affected);
    if (explicit) setMetadataPaused(false);
    setLoadingApps(true);
    setInventory({ device, valid: false });
    void api.apps(device, true).then(apps => {
      if (!alive) return;
      cache.reconcile(apps); publish();
      setInventory({ device, valid: true });
    }).catch(cause => { if (alive) fail(cause); }).finally(() => { if (alive) setLoadingApps(false); });
    return () => { alive = false; };
  }, [cache, device, refresh, revision, fail]);

  useEffect(() => {
    let alive = true;
    if (!device || view.device !== device || !active || metadataPaused) { setLoadingMetadata(false); return; }
    // A read may have finished while enrichment was paused or another page open.
    publish();
    const candidates = view.apps.filter(app => (includeSystem || !app.system) && cache.needsRead(app.packageName));
    setLoadingMetadata(candidates.length > 0);
    void (async () => {
      for (const app of candidates) {
        if (!alive) return;
        try { await cache.read(app.packageName, () => api.details(device, app.packageName)); } catch { /* Per-app error stays in the cache. */ }
        if (alive) publish();
      }
      if (alive) setLoadingMetadata(false);
    })();
    return () => { alive = false; };
  }, [cache, device, active, includeSystem, metadataPaused, view.apps, view.device]);

  const readDetails = async (app: AppPackage) => {
    try {
      const data = await cache.read(app.packageName, () => api.details(device, app.packageName), true);
      return currentCache.current === cache ? data : null;
    }
    finally { publish(); }
  };
  const resumeMetadata = () => { cache.invalidate(new Set(Object.keys(cache.snapshot().errors))); setMetadataPaused(false); };
  return {
    ...(view.device === device ? view : { device, ...cache.snapshot() }), loadingApps, loadingMetadata, inventoryReady: inventory.device === device && inventory.valid,
    metadataPaused, setMetadataPaused, resumeMetadata, readDetails,
  };
}
