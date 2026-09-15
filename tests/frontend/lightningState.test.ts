import test from 'node:test';
import assert from 'node:assert/strict';
import { lightningPackage, lightningVariants, matchingInstalledRelease, matchingRelease, navigatorPackage, previewLightningCatalog, recommendedService, serviceChoiceFor, showLightningSuggestion } from '../../src/lightningState.ts';
import { applicationChanges } from '../../src/applicationState.ts';
import type { AppPackage } from '../../src/types.ts';

const app = (packageName: string): AppPackage => ({ packageName, versionCode: '1', system: false, installer: null, apkPath: null });
test('suggestions require a completed inventory and respect every edition and explicit dismissal', () => {
  assert.equal(showLightningSuggestion(true, true, true, []), true);
  for (const [ready, loaded, enabled] of [[false, true, true], [true, false, true], [true, true, false]]) {
    assert.equal(showLightningSuggestion(ready, loaded, enabled, []), false);
  }
  for (const name of lightningVariants) assert.equal(showLightningSuggestion(true, true, true, [app(name)]), false);
  assert.equal(showLightningSuggestion(true, true, true, [app(`${lightningPackage}.unrelated`), app(navigatorPackage)]), true);
});
test('recommendations match explicit tags, never choose the newest unverified service', () => {
  assert.equal(recommendedService('addons1.1.0', previewLightningCatalog)?.assetId, 4);
  assert.equal(recommendedService('addons9.9.0', previewLightningCatalog), undefined);
  assert.equal(recommendedService(null, previewLightningCatalog), undefined);
  assert.equal(matchingRelease('v1.2', previewLightningCatalog.launchers)?.assetId, 1);
  assert.equal(matchingRelease('unknown', previewLightningCatalog.launchers), undefined);
});
test('installed markers match numeric versions and exact APK editions, including addon tags', () => {
  assert.equal(matchingInstalledRelease(lightningPackage, 'v1.2', previewLightningCatalog)?.assetId, 1);
  assert.equal(matchingInstalledRelease(navigatorPackage, '1.1.0', previewLightningCatalog)?.assetId, 4);
  assert.equal(matchingInstalledRelease(navigatorPackage, '1.2', previewLightningCatalog)?.assetId, 3);
  for (const name of [null, `${lightningPackage}.metastore`, `${lightningPackage}.playstore`, `${navigatorPackage}.unrelated`]) {
    assert.equal(matchingInstalledRelease(name, '1.2.0', previewLightningCatalog), undefined);
  }
  for (const version of ['', 'unknown', '1.2-beta', '1.20.0', '9.9.0']) {
    assert.equal(matchingInstalledRelease(lightningPackage, version, previewLightningCatalog), undefined);
  }
  assert.equal(matchingInstalledRelease(lightningPackage, '1.2.0', null), undefined);
});
test('changing the launcher or refreshing cannot carry over a manual service choice', () => {
  const previous = { key: '1.1.0:0', assetId: 3, otherVersions: true };
  const older = recommendedService('addons1.1.0', previewLightningCatalog);
  const newer = recommendedService('addons1.2.0', previewLightningCatalog);
  assert.deepEqual(serviceChoiceFor('1.1.0:0', previous, older), previous);
  assert.deepEqual(serviceChoiceFor('1.2.0:0', previous, undefined), { key: '1.2.0:0', assetId: null, otherVersions: false });
  assert.deepEqual(serviceChoiceFor('1.2.0:0', previous, newer), { key: '1.2.0:0', assetId: 3, otherVersions: false });
  assert.deepEqual(serviceChoiceFor('1.1.0:1', previous, older), { key: '1.1.0:1', assetId: 4, otherVersions: false });
});
test('partial optional setup refreshes both packages only on its captured transport', () => {
  const task = { id: 'EXAMPLE-LIGHTNING-TASK', device: 'DEMO-USB-001', kind: 'install' as const, status: 'failed' as const, apkInstalled: true, packageName: null };
  assert.equal(applicationChanges([task], ['DEMO-USB-001'], new Set()), null);
  assert.deepEqual(applicationChanges([task], ['DEMO-USB-002'], new Set()), new Set());
});
