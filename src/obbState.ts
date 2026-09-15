import type { LocalApk, LocalObb, ObbInstall } from './types.ts';

export function canAttachObbs(details?: Pick<LocalApk, 'packageName' | 'split'>): boolean {
  return !!details && !details.split && /^[A-Za-z0-9_]+(?:\.[A-Za-z0-9_]+)+$/.test(details.packageName);
}

export function mergeObbs(current: LocalObb[], incoming: LocalObb[]): LocalObb[] {
  const result = [...current];
  for (const file of incoming) {
    if (!/\.obb$/i.test(file.name)) throw new Error('Choose .obb files for the selected APK.');
    const existing = result.find(row => row.name.toLowerCase() === file.name.toLowerCase());
    if (existing && existing.source.toLowerCase() !== file.source.toLowerCase()) throw new Error(`More than one OBB file is named ${file.name}. Choose only one.`);
    if (existing) result[result.indexOf(existing)] = file;
    else result.push(file);
  }
  if (result.length > 128) throw new Error('Choose no more than 128 OBB files per APK.');
  return result;
}

export function obbOptions(details: LocalApk | undefined, files: LocalObb[]): ObbInstall | undefined {
  if (!canAttachObbs(details) || !files.length) return undefined;
  return { apkSourceStamp: details!.sourceStamp, files: files.map(({ source, sourceStamp }) => ({ source, sourceStamp })) };
}
