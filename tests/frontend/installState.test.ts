import assert from 'node:assert/strict';
import { test } from 'node:test';
import { compareVersions, duplicatePackages, installationStatus, observeInstalledApps, type InstalledSnapshot } from '../../src/installState.ts';
import type { AppPackage } from '../../src/types.ts';

const app: AppPackage = { packageName: 'com.example.game', versionCode: '100', system: false, installer: null, apkPath: null };
const ready: InstalledSnapshot = { key: 'EXAMPLE-CHECK-1', status: 'ready', apps: [app] };
const status = (snapshot: InstalledSnapshot | null, packageName: string | undefined = app.packageName, device = 'DEMO-USB-001') => installationStatus(packageName, '101', device, ready.key, snapshot);
const tick = () => new Promise<void>(resolve => setImmediate(resolve));

test('compares numeric versions losslessly and never guesses for unavailable versions', () => {
  assert.equal(compareVersions('10', '9'), 'newer');
  assert.equal(compareVersions('9', '10'), 'older');
  assert.equal(compareVersions('0100', '100'), 'same');
  assert.equal(compareVersions('9007199254740993', '9007199254740992'), 'newer');
  assert.equal(compareVersions('4294967296', '2147483647'), 'newer');
  for (const value of ['', 'Unknown', '1.2', '-1', '1e3', ' 100']) {
    assert.equal(compareVersions(value, '100'), 'unknown');
    assert.equal(compareVersions('100', value), 'unknown');
  }
});

test('only a successful current complete query can report absence; packages match exactly', () => {
  assert.equal(status(ready).comparison, 'newer');
  assert.equal(status(ready, 'com.example.gam').label, 'Not installed on this headset');
  assert.equal(status({ ...ready, apps: [{ ...app, system: true }] }).installed?.system, true);
  assert.equal(status({ ...ready, apps: [{ ...app, versionCode: 'Unknown' }] }).comparison, 'unknown');
  assert.equal(status(null).label, 'Checking headset…');
  assert.equal(status({ ...ready, key: 'EXAMPLE-OLD-CHECK' }).label, 'Checking headset…');
  assert.equal(status({ key: ready.key, status: 'error', error: 'Example failure' }).label, 'Installation status unavailable');
  assert.equal(status(ready, '', 'DEMO-USB-001').label, 'Installation status unavailable');
  assert.equal(status(ready, app.packageName, '').label, 'Connect a headset to check');
});

test('discards old device results, includes system apps and exposes failures for retry', async () => {
  const snapshots: InstalledSnapshot[] = [];
  let finish!: (apps: AppPackage[]) => void;
  const dispose = observeInstalledApps('EXAMPLE-OLD', 'DEMO-USB-001', (device, includeSystem) => {
    assert.equal(device, 'DEMO-USB-001');
    assert.equal(includeSystem, true);
    return new Promise(resolve => { finish = resolve; });
  }, value => snapshots.push(value));
  dispose();
  observeInstalledApps('EXAMPLE-NEW', 'DEMO-USB-002', async () => [app], value => snapshots.push(value));
  await tick();
  finish([]);
  await tick();
  assert.deepEqual(snapshots.at(-1), { key: 'EXAMPLE-NEW', status: 'ready', apps: [app] });
  assert.equal(snapshots.filter(value => value.key === 'EXAMPLE-OLD').length, 1);
  observeInstalledApps('EXAMPLE-FAIL', 'DEMO-USB-002', async () => { throw new Error('Example offline connection'); }, value => snapshots.push(value));
  await tick();
  assert.deepEqual(snapshots.at(-1), { key: 'EXAMPLE-FAIL', status: 'error', error: 'Example offline connection' });
  observeInstalledApps('EXAMPLE-RETRY', 'DEMO-USB-002', async () => [], value => snapshots.push(value));
  await tick();
  assert.deepEqual(snapshots.at(-1), { key: 'EXAMPLE-RETRY', status: 'ready', apps: [] });
});

test('duplicates depend on package identity and disappear when a selected copy is removed', () => {
  const items = [{ details: app }, {}, { details: { packageName: 'com.example.other' } }, { details: app }];
  assert.deepEqual([...duplicatePackages(items)], [app.packageName]);
  assert.equal(duplicatePackages(items.slice(0, -1)).size, 0);
});
