import assert from 'node:assert/strict';
import { test } from 'node:test';
import { applicationChanges, createApplicationCache, installSource, type AppMutation } from '../../src/applicationState.ts';
import type { AppDetails, AppPackage } from '../../src/types.ts';

const app = (name: string, patch: Partial<AppPackage> = {}): AppPackage => ({
  packageName: `com.example.${name}`, versionCode: '100', system: false, installer: null, apkPath: `/data/app/example/${name}/base.apk`, ...patch,
});
const details = (app: AppPackage, label = 'Example app'): AppDetails => ({
  packageName: app.packageName, versionName: '1.0', versionCode: app.versionCode, apkPaths: [], apkFiles: [], apkSize: 100,
  firstInstallTime: null, lastUpdateTime: null, installer: app.installer, minSdk: null, targetSdk: null, primaryAbi: null,
  secondaryAbi: null, uid: null, androidUser: 0, enabled: true, stateFlags: [], permissions: [],
  assets: { displayName: label, iconDataUrl: '/example.svg', vrFeatures: [], signingSchemes: [], certificateSha256: [], notes: [] },
});
const mutation = (patch: Partial<AppMutation> = {}): AppMutation => ({ id: 'EXAMPLE-1', device: 'DEMO-USB-001', kind: 'install', status: 'success', packageName: 'com.example.game', ...patch });

test('source uses exact installer identity; null and app package names do not prove a source', () => {
  assert.equal(installSource(app('game', { installer: 'com.oculus.ocms' })), 'meta');
  assert.equal(installSource(app('game', { installer: 'com.android.shell' })), 'sideload');
  assert.equal(installSource(app('game', { installer: 'com.example.installer' })), 'other');
  assert.equal(installSource(app('game', { installer: 'com.oculus.ocms.fake' })), 'other');
  assert.equal(installSource(app('game', { packageName: 'com.oculus.example' })), 'unknown');
  assert.equal(installSource(app('system', { system: true })), 'unknown');
});

test('deleting one app and toggling system visibility reuses unrelated metadata', async () => {
  const cache = createApplicationCache();
  const first = app('first'), second = app('second'), system = app('system', { system: true });
  const reads: string[] = [];
  const enrich = async (includeSystem: boolean) => {
    for (const entry of cache.snapshot().apps.filter(app => includeSystem || !app.system)) {
      if (cache.needsRead(entry.packageName)) await cache.read(entry.packageName, async () => { reads.push(entry.packageName); return details(entry); });
    }
  };
  cache.reconcile([first, second, system]);
  await enrich(false);
  const preserved = cache.snapshot().metadata[second.packageName];
  cache.invalidate(new Set([first.packageName]));
  cache.reconcile([second, system]);
  await enrich(false);
  assert.equal(cache.snapshot().metadata[first.packageName], undefined);
  assert.equal(cache.snapshot().metadata[second.packageName], preserved);
  await enrich(true);
  await enrich(false);
  await enrich(true);
  assert.deepEqual(reads, [first.packageName, second.packageName, system.packageName]);
});

test('same-version install invalidates only its package and explicit refresh retains displayed artwork', async () => {
  const cache = createApplicationCache();
  const game = app('game'), other = app('other');
  cache.reconcile([game, other]);
  await cache.read(game.packageName, async () => details(game, 'Original'));
  await cache.read(other.packageName, async () => details(other));
  cache.invalidate(applicationChanges([mutation()], ['DEMO-USB-001'], new Set()));
  cache.installed(game.packageName);
  cache.reconcile([game, other]);
  assert.equal(cache.needsRead(game.packageName), true);
  assert.equal(cache.needsRead(other.packageName), false);
  assert.equal(cache.snapshot().metadata[game.packageName].assets.displayName, 'Original');
  assert.equal(cache.snapshot().sources[game.packageName], 'sideload');
  await cache.read(game.packageName, async () => details(game, 'Modified'));
  assert.equal(cache.snapshot().metadata[game.packageName].assets.displayName, 'Modified');
  cache.invalidate();
  assert.equal(cache.snapshot().metadata[game.packageName].assets.displayName, 'Modified');
  assert.equal(cache.needsRead(other.packageName), true);
});

test('version, APK path and installer changes invalidate metadata and clear obsolete source hints', async () => {
  const cache = createApplicationCache();
  let game = app('game');
  cache.installed(game.packageName); cache.reconcile([game]);
  for (const patch of [{ versionCode: '101' }, { apkPath: '/data/app/example/new/base.apk' }, { installer: 'com.oculus.ocms' }]) {
    await cache.read(game.packageName, async () => details(game));
    game = { ...game, ...patch }; cache.reconcile([game]);
    assert.equal(cache.needsRead(game.packageName), true);
  }
  assert.equal(cache.snapshot().sources[game.packageName], 'meta');
});

test('invalidated and removed pending reads cannot overwrite or restore application entries', async () => {
  const cache = createApplicationCache(), game = app('game');
  cache.reconcile([game]);
  let finish!: (details: AppDetails) => void;
  const old = cache.read(game.packageName, () => new Promise(resolve => { finish = resolve; }));
  await Promise.resolve();
  cache.invalidate(new Set([game.packageName]));
  await cache.read(game.packageName, async () => details(game, 'New'));
  finish(details(game, 'Old'));
  assert.equal(await old, null);
  assert.equal(cache.snapshot().metadata[game.packageName].assets.displayName, 'New');
  const removed = cache.read(game.packageName, () => new Promise(resolve => { finish = resolve; }), true);
  await Promise.resolve();
  cache.reconcile([]); finish(details(game));
  assert.equal(await removed, null);
  assert.deepEqual(cache.snapshot().metadata, {});
});

test('details share an active request, force fresh reads when reopened and recover after failure', async () => {
  const cache = createApplicationCache(), game = app('game'); cache.reconcile([game]);
  let count = 0;
  const read = async () => { count++; return details(game); };
  await Promise.all([cache.read(game.packageName, read), cache.read(game.packageName, read, true)]);
  await cache.read(game.packageName, read);
  assert.equal(count, 1);
  await cache.read(game.packageName, read, true);
  assert.equal(count, 2);
  cache.invalidate();
  await assert.rejects(cache.read(game.packageName, () => { throw new Error('Example offline'); }));
  assert.equal(cache.snapshot().errors[game.packageName], 'Example offline');
  assert.ok(cache.snapshot().metadata[game.packageName]);
  await cache.read(game.packageName, read, true);
  assert.equal(cache.snapshot().errors[game.packageName], undefined);
});

test('mutation invalidation includes alternate transports, ignores unrelated work and falls back for missing hints', () => {
  const seen = new Set<string>();
  const tasks = [mutation(), mutation({ id: 'EXAMPLE-2', device: 'DEMO-WIFI-001', packageName: 'com.example.other' }), mutation({ id: 'EXAMPLE-3', device: 'DEMO-OTHER-DEVICE' })];
  assert.deepEqual([...applicationChanges(tasks, ['DEMO-USB-001', 'DEMO-WIFI-001'], seen)!], ['com.example.game', 'com.example.other']);
  assert.equal(applicationChanges(tasks, ['DEMO-USB-001', 'DEMO-WIFI-001'], seen)?.size, 0);
  assert.equal(applicationChanges([mutation({ packageName: null })], ['DEMO-USB-001'], new Set()), null);
  assert.equal(applicationChanges([mutation({ kind: 'export' }), mutation({ status: 'failed' })], ['DEMO-USB-001'], new Set())?.size, 0);
});
