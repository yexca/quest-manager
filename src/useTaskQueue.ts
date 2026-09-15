import { useCallback, useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { api, isDesktop, isPreview } from './api';
import { connectTaskStream, createRefreshBatcher, createTaskStore, refreshForTransports } from './taskState';
import type { Task, TaskRequest } from './types';
import type { AppMutation } from './applicationState';

export function useTaskQueue(transports: string[], fail: (error: unknown) => void) {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [appMutations, setAppMutations] = useState<AppMutation[]>([]);
  const [versions, setVersions] = useState({ info: 0, apps: 0, files: 0 });
  const [clearing, setClearing] = useState(false);
  const currentTransports = useRef(transports);
  currentTransports.current = transports;
  const [batcher] = useState(() => createRefreshBatcher(pending => {
    const areas = refreshForTransports(pending, currentTransports.current);
    if (areas.info || areas.apps || areas.files) setVersions(current => ({
      info: current.info + Number(areas.info), apps: current.apps + Number(areas.apps), files: current.files + Number(areas.files),
    }));
  }));
  const [store] = useState(() => createTaskStore(setTasks, task => {
    if (task.kind === 'install' || task.kind === 'uninstall') setAppMutations(current => [...current, {
      id: task.id, device: task.device, kind: task.kind, status: task.status, packageName: task.packageName, apkInstalled: task.apkInstalled,
    }]);
    batcher.add(task);
  }));

  useEffect(() => {
    if (!isDesktop && !isPreview) return;
    const disconnect = connectTaskStream(
      receive => isDesktop ? listen<Task>('task-updated', event => receive(event.payload)) : Promise.resolve(() => {}),
      api.tasks, store.merge, fail,
    );
    return () => { disconnect(); batcher.dispose(); };
  }, [store, batcher, fail]);

  const start = useCallback(async (request: TaskRequest) => {
    const task = await api.start(request);
    store.merge([task]);
    return task;
  }, [store]);
  const clearCompleted = async () => {
    if (clearing) return;
    setClearing(true);
    try { store.remove(await api.clearCompletedTasks()); }
    catch (error) { fail(error); }
    finally { setClearing(false); }
  };
  return { tasks, appMutations, versions, start, clearCompleted, clearing };
}
