import assert from 'node:assert/strict';
import { test } from 'node:test';
import { connectTaskStream, createRefreshBatcher, createTaskStore, refreshForTransports } from '../../src/taskState.ts';
import type { Task } from '../../src/types.ts';

const task = (overrides: Partial<Task> = {}): Task => ({
  id: 'EXAMPLE-TASK-1', revision: 0, device: 'DEMO-USB-001', kind: 'upload', label: 'Upload example.txt',
  status: 'queued', detail: 'Waiting', progress: null, createdAt: 1, ...overrides,
});
const tick = () => new Promise<void>(resolve => setImmediate(resolve));

test('a completed APK with failed OBB data refreshes apps and files once without becoming a success', () => {
  let current: Task[] = [];
  const refreshes: Task[] = [];
  let flush!: () => void;
  let areas = { info: false, apps: false, files: false };
  const batcher = createRefreshBatcher(pending => { areas = refreshForTransports(pending, ['DEMO-USB-001']); }, run => { flush = run; return () => {}; });
  const store = createTaskStore(tasks => { current = tasks; }, task => { refreshes.push(task); batcher.add(task); });
  store.merge([task({ kind: 'install', revision: 1, status: 'running', apkInstalled: true, includesObb: true })]);
  assert.equal(refreshes.length, 0);
  const failed = task({ kind: 'install', revision: 2, status: 'failed', apkInstalled: true, includesObb: true });
  store.merge([failed]); store.merge([failed]);
  flush();
  assert.equal(refreshes.length, 1);
  assert.equal(current[0].status, 'failed');
  assert.deepEqual(areas, { info: true, apps: true, files: true });
  store.merge([task({ id: 'EXAMPLE-NO-INSTALL', kind: 'install', status: 'failed', includesObb: true, apkInstalled: false })]);
  assert.equal(refreshes.length, 1);
});
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(done => { resolve = done; });
  return { promise, resolve };
}

test('newer events survive stale start responses and snapshots; success refreshes once', () => {
  let current: Task[] = [];
  let refreshes = 0;
  const store = createTaskStore(tasks => { current = tasks; }, () => { refreshes++; });
  store.merge([task({ revision: 2, status: 'running', progress: 50 })]);
  store.merge([task()]);
  assert.equal(current[0].progress, 50);
  store.merge([task({ revision: 3, status: 'success' })]);
  store.merge([task({ revision: 1, status: 'running' })], false);
  store.merge([task({ revision: 3, status: 'success' })]);
  assert.equal(current[0].status, 'success');
  assert.equal(refreshes, 1);
});

test('clearing terminal tasks cannot restore them through delayed events or snapshots', () => {
  let current: Task[] = [];
  const store = createTaskStore(tasks => { current = tasks; }, () => {});
  store.merge([task(), task({ id: 'EXAMPLE-TASK-2', revision: 3, status: 'success' })]);
  store.remove(['EXAMPLE-TASK-2']);
  store.merge([task({ id: 'EXAMPLE-TASK-2', revision: 3, status: 'success' })], false);
  assert.deepEqual(current.map(task => task.id), ['EXAMPLE-TASK-1']);
});

test('subscription is established before snapshot; an overlapping completion wins', async () => {
  const registered = deferred<() => void>();
  const snapshot = deferred<Task[]>();
  let receive!: (task: Task) => void;
  let reads = 0;
  let current: Task[] = [];
  const store = createTaskStore(tasks => { current = tasks; }, () => {});
  const disconnect = connectTaskStream(callback => { receive = callback; return registered.promise; },
    () => { reads++; return snapshot.promise; }, store.merge, error => assert.fail(String(error)));
  assert.equal(reads, 0);
  registered.resolve(() => {});
  await tick();
  assert.equal(reads, 1);
  receive(task({ revision: 2, status: 'success' }));
  snapshot.resolve([task({ revision: 1, status: 'running' })]);
  await tick();
  assert.equal(current[0].status, 'success');
  disconnect();
});

test('unmount during registration disposes the listener without fetching or updating', async () => {
  const registered = deferred<() => void>();
  let receive!: (task: Task) => void;
  let stopped = 0;
  const disconnect = connectTaskStream(callback => { receive = callback; return registered.promise; },
    async () => { assert.fail('must not read after disposal'); }, () => assert.fail('stale update'), error => assert.fail(String(error)));
  disconnect();
  registered.resolve(() => { stopped++; });
  await tick();
  receive(task());
  assert.equal(stopped, 1);
});

test('unmount during snapshot ignores its late response', async () => {
  const snapshot = deferred<Task[]>();
  const disconnect = connectTaskStream(async () => () => {}, () => snapshot.promise,
    () => assert.fail('stale update'), error => assert.fail(String(error)));
  await tick();
  disconnect();
  snapshot.resolve([task()]);
  await tick();
});

test('completed task history does not refresh device data on startup', () => {
  const store = createTaskStore(() => {}, () => assert.fail('history must not trigger a refresh'));
  store.merge([task({ revision: 2, status: 'success' })], false);
});

test('batch completion refreshes only affected areas of the selected physical device', () => {
  let callback!: () => void;
  let schedules = 0;
  let selected = ['DEMO-USB-001', 'DEMO-WIFI-001'];
  const updates: ReturnType<typeof refreshForTransports>[] = [];
  const batcher = createRefreshBatcher(pending => updates.push(refreshForTransports(pending, selected)), run => {
    schedules++; callback = run; return () => {};
  });
  batcher.add(task({ status: 'success' }));
  batcher.add(task({ status: 'success', kind: 'delete' }));
  batcher.add(task({ status: 'success', kind: 'install', device: 'DEMO-USB-002' }));
  assert.equal(schedules, 1);
  callback();
  assert.deepEqual(updates[0], { info: true, apps: false, files: true });
  batcher.add(task({ status: 'success', kind: 'uninstall', device: 'DEMO-WIFI-001' }));
  callback();
  assert.deepEqual(updates[1], { info: true, apps: true, files: false });
  batcher.add(task({ status: 'success' }));
  selected = ['DEMO-USB-002'];
  callback();
  assert.deepEqual(updates[2], { info: false, apps: false, files: false });
});

test('downloads, exports and non-success states never trigger device refreshes', () => {
  const batcher = createRefreshBatcher(() => assert.fail('unexpected flush'), () => { assert.fail('unexpected timer'); });
  batcher.add(task({ status: 'success', kind: 'download' }));
  batcher.add(task({ status: 'success', kind: 'export' }));
  batcher.add(task({ status: 'failed' }));
  batcher.add(task());
});

test('disposing pending refreshes cancels the timer', () => {
  let cancelled = 0;
  const batcher = createRefreshBatcher(() => assert.fail('unexpected flush'), () => () => { cancelled++; });
  batcher.add(task({ status: 'success' }));
  batcher.dispose();
  assert.equal(cancelled, 1);
});
