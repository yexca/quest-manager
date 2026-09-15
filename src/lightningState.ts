import type { AppPackage, LightningCatalog, LightningRelease } from './types.ts';

export const lightningPackage = 'com.threethan.launcher';
export const navigatorPackage = 'com.threethan.launcher.service.navigator';
export const lightningVariants = [lightningPackage, `${lightningPackage}.metastore`, `${lightningPackage}.playstore`];
export const suggestionKey = 'quest-manager.show-lightning-suggestion';
export function showLightningSuggestion(ready: boolean, inventoryReady: boolean, enabled: boolean, apps: AppPackage[]) {
  return ready && inventoryReady && enabled && !apps.some(app => lightningVariants.includes(app.packageName));
}
export function matchingRelease(version: string, releases: LightningRelease[]) {
  const normalize = (value: string) => {
    const numeric = value.replace(/^(?:addons|v)/, '');
    return /^\d+(?:\.\d+)*$/.test(numeric) ? numeric.replace(/(?:\.0)+$/, '') : null;
  };
  const installed = normalize(version);
  return installed === null ? undefined : releases.find(release => normalize(release.tag) === installed);
}
export function matchingInstalledRelease(packageName: string | null | undefined, version: string, catalog: LightningCatalog | null) {
  const releases = packageName === lightningPackage ? catalog?.launchers : packageName === navigatorPackage ? catalog?.navigators : undefined;
  return matchingRelease(version, releases ?? []);
}
export function recommendedService(tag: string | null | undefined, catalog: LightningCatalog | null) {
  return tag ? catalog?.navigators.find(release => release.tag === tag) : undefined;
}
export type ServiceChoice = { key: string; assetId: number | null; otherVersions: boolean };
export function serviceChoiceFor(key: string, choice: ServiceChoice | null, recommended: LightningRelease | undefined): ServiceChoice {
  return choice?.key === key ? choice : { key, assetId: recommended?.assetId ?? null, otherVersions: false };
}
// Fictional release metadata for layout inspection; preview never contacts GitHub.
export const previewLightningCatalog: LightningCatalog = {
  launchers: [
    { tag: '1.2.0', assetId: 1, size: 3145728, publishedAt: '2025-02-10T00:00:00Z', sha256: 'a'.repeat(64) },
    { tag: '1.1.0', assetId: 2, size: 2945728, publishedAt: '2025-01-01T00:00:00Z', sha256: null },
  ],
  navigators: [
    { tag: 'addons1.2.0', assetId: 3, size: 24576, publishedAt: '2025-02-10T00:00:00Z', sha256: 'b'.repeat(64) },
    { tag: 'addons1.1.0', assetId: 4, size: 26112, publishedAt: '2025-01-01T00:00:00Z', sha256: 'c'.repeat(64) },
  ],
};
