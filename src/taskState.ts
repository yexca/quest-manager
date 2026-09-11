import type { Task } from './types.ts';

export const isActiveTask = (task: Task) => task.status === 'queued' || task.status === 'running';

// Shared by events, initial snapshots and start_task responses. Cleared IDs stay
// as small session-only tombstones so delayed IPC responses cannot restore them.
export function createTaskStore(changed: (tasks: Task[]) => void, succeeded: (task: Task) => void) {
  const records = new Map<string, Task>();
  const cleared = new Set<string>();
  return {
    merge(incoming: Task[], notifySuccess = true) {
      let updated = false;
      const successes: Task[] = [];
      for (const task of incoming) {
        const previous = records.get(task.id);
        if (cleared.has(task.id) || (previous && previous.revision >= task.revision)) continue;
        records.set(task.id, task);
        updated = true;
        if (notifySuccess && task.status === 'success' && previous?.status !== 'success') successes.push(task);
      }
      if (updated) changed([...records.values()]);
      successes.forEach(succeeded);
    },
    remove(ids: string[]) {
      for (const id of ids) { cleared.add(id); records.delete(id); }
      changed([...records.values()]);
    },
  };
}

// Register first, then fetch: every change falls into the snapshot, the event
// stream, or both. Revision merging makes the overlap harmless.
export function connectTaskStream(
  subscribe: (receive: (task: Task) => void) => Promise<() => void>,
  snapshot: () => Promise<Task[]>,
  merge: (tasks: Task[], notifySuccess?: boolean) => void,
  fail: (error: unknown) => void,
) {
  let disposed = false;
  let stop: (() => void) | undefined;
  void (async () => {
    try {
      stop = await subscribe(task => { if (!disposed) merge([task]); });
      if (disposed) { stop(); return; }
      const tasks = await snapshot();
      if (!disposed) merge(tasks, false);
    } catch (error) { if (!disposed) fail(error); }
  })();
  return () => { disposed = true; stop?.(); };
}

export interface RefreshAreas { info: boolean; apps: boolean; files: boolean }
export function taskRefreshAreas(task: Task): RefreshAreas {
  const apps = task.kind === 'install' || task.kind === 'uninstall';
  const files = ['upload', 'mkdir', 'rename', 'delete'].includes(task.kind);
  return { info: apps || files, apps, files };
}

export function createRefreshBatcher(
  flush: (pending: Map<string, RefreshAreas>) => void,
  schedule: (callback: () => void) => () => void = callback => {
    const timer = setTimeout(callback, 350);
    return () => clearTimeout(timer);
  },
) {
  let pending = new Map<string, RefreshAreas>();
  let cancel: (() => void) | undefined;
  return {
    add(task: Task) {
      if (task.status !== 'success') return;
      const areas = taskRefreshAreas(task);
      if (!areas.info && !areas.apps && !areas.files) return;
      const previous = pending.get(task.device);
      pending.set(task.device, {
        info: areas.info || !!previous?.info,
        apps: areas.apps || !!previous?.apps,
        files: areas.files || !!previous?.files,
      });
      if (!cancel) cancel = schedule(() => {
        const batch = pending;
        pending = new Map(); cancel = undefined;
        flush(batch);
      });
    },
    dispose() { cancel?.(); cancel = undefined; pending.clear(); },
  };
}

export function refreshForTransports(pending: Map<string, RefreshAreas>, transports: string[]): RefreshAreas {
  return transports.reduce<RefreshAreas>((result, serial) => {
    const areas = pending.get(serial);
    return { info: result.info || !!areas?.info, apps: result.apps || !!areas?.apps, files: result.files || !!areas?.files };
  }, { info: false, apps: false, files: false });
}
