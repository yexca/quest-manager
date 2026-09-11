import { useState } from 'react';
import { Gamepad2, Info, LoaderCircle, ShieldCheck } from 'lucide-react';
import type { AppDetails, AppPackage } from './types';
import { installSource, sourceLabels, type InstallSource } from './applicationState';

export function formatBytes(value: number | null | undefined) {
  if (value == null) return '—';
  if (value === 0) return '0 B';
  const index = Math.min(Math.floor(Math.log(value) / Math.log(1024)), 4);
  return `${(value / 1024 ** index).toFixed(index === 0 ? 0 : 1)} ${['B', 'KB', 'MB', 'GB', 'TB'][index]}`;
}

export function AppIcon({ data, index = 0 }: { data?: AppDetails | null; index?: number }) {
  const [failedUrl, setFailedUrl] = useState<string | null>(null);
  const url = data?.assets.iconDataUrl;
  return <span className={`app-tile tile-${index % 5}`}>
    {url && url !== failedUrl ? <img src={url} alt="" onError={() => setFailedUrl(url)} /> : <Gamepad2 size={22} />}
  </span>;
}

const state = (enabled: boolean | null | undefined) => enabled == null ? 'Unavailable' : enabled ? 'Enabled' : 'Disabled';

export function ApplicationDetails({ app, source, data, error, onRetry }: { app: AppPackage; source?: InstallSource; data: AppDetails | null; error: string | null; onRetry: () => void }) {
  const [tab, setTab] = useState('overview');
  const [permissionQuery, setPermissionQuery] = useState('');
  const rows = data ? [
    ['Version', data.versionName], ['Version code', data.versionCode], ['APK size', formatBytes(data.apkSize)],
    ['Application type', app.system ? 'System' : 'Third-party'], ['State', state(data.enabled)],
    ['Install source', sourceLabels[source ?? installSource(app)]],
    ['First installed', data.firstInstallTime], ['Last updated', data.lastUpdateTime], ['Installer package', data.installer],
    ['Minimum SDK', data.minSdk], ['Target SDK', data.targetSdk], ['Primary ABI', data.primaryAbi], ['Secondary ABI', data.secondaryAbi],
    ['App ID', data.uid], ['Android user', String(data.androidUser)], ['State flags', data.stateFlags.length ? data.stateFlags.join(', ') : 'None reported'],
  ] : [];
  const permissions = data?.permissions.filter(item => item.name.toLowerCase().includes(permissionQuery.toLowerCase())) ?? [];
  return <>
    <div className="app-detail-heading"><AppIcon data={data} /><div><strong>{data?.assets.displayName ?? app.packageName}</strong><small className="mono">{app.packageName}</small></div><span className={`type-badge ${app.system ? 'system' : ''}`}>{app.system ? 'System' : 'Installed'}</span></div>
    {error ? <div className="metadata-error"><p role="alert">{error}</p><button className="button secondary small" onClick={onRetry}>Retry</button></div> : !data ? <div className="list-empty"><LoaderCircle size={25} className="spin" /><p>Reading application details…</p><span>Names and artwork load once, then stay cached on this computer.</span></div> : <>
      <div className="detail-tabs" role="tablist" aria-label="Application information" onKeyDown={event => {
        const ids = ['overview', 'permissions', 'files', 'signing'];
        const index = ids.indexOf(tab);
        const next = event.key === 'ArrowRight' ? (index + 1) % ids.length : event.key === 'ArrowLeft' ? (index + ids.length - 1) % ids.length : event.key === 'Home' ? 0 : event.key === 'End' ? ids.length - 1 : -1;
        if (next >= 0) { event.preventDefault(); setTab(ids[next]); event.currentTarget.querySelectorAll('button')[next].focus(); }
      }}>
        {[['overview', 'Overview'], ['permissions', `Permissions (${data.permissions.length})`], ['files', `APK files (${data.apkFiles.length})`], ['signing', 'Signing & VR']].map(([id, label]) => <button key={id} role="tab" id={`metadata-tab-${id}`} aria-controls="metadata-panel" aria-selected={tab === id} tabIndex={tab === id ? 0 : -1} onClick={() => setTab(id)}>{label}</button>)}
      </div>
      <div id="metadata-panel" role="tabpanel" aria-labelledby={`metadata-tab-${tab}`} className="metadata-panel">
        {tab === 'overview' && <><dl className="metadata-grid">{rows.map(([label, value]) => <div key={label}><dt>{label}</dt><dd>{value ?? 'Unavailable'}</dd></div>)}</dl><p className="inline-note"><Info size={14} />APK size excludes app data, cache, OBB files and saved games. Times are reported in the headset’s local time.</p></>}
        {tab === 'permissions' && <><input className="text-input" aria-label="Filter permissions" placeholder="Filter permissions…" value={permissionQuery} onChange={e => setPermissionQuery(e.target.value)} /><p className="inline-note">Grants reflect Android user {data.androidUser}. “Unknown” means the headset did not report a grant state.</p><div className="permission-list">{permissions.map(item => <div key={item.name}><div><code>{item.name}</code><small>{item.kind}</small></div><span className={`permission-grant ${item.granted === false ? 'denied' : ''}`}>{item.granted == null ? 'Unknown' : item.granted ? 'Granted' : 'Not granted'}</span></div>)}</div>{!permissions.length && <p className="metadata-empty">No matching permissions reported.</p>}</>}
        {tab === 'files' && <><p className="inline-note">Export includes the base APK and every installed split. These are application files; private app data is separate.</p><div className="apk-file-list">{data.apkFiles.map(file => <div key={file.path}><div><strong>{file.path.split('/').pop()}</strong><span>{formatBytes(file.size)}</span></div><code>{file.path}</code></div>)}</div></>}
        {tab === 'signing' && <><h3 className="metadata-heading"><ShieldCheck size={16} />Signing certificates</h3><p className="inline-note">SHA-256 fingerprints from the base APK signing block. This identifies certificates; it does not verify APK integrity, publisher identity or store ownership.</p><p className="metadata-label">Reported schemes: {data.assets.signingSchemes.join(', ') || 'Unavailable'}</p>{data.assets.certificateSha256.map(item => <code className="apk-path certificate" key={item}>{item}</code>)}{!data.assets.certificateSha256.length && <p className="metadata-empty">No supported signing certificate was available.</p>}<h3 className="metadata-heading">VR declarations</h3><p className="inline-note">Manifest declarations describe intended capabilities, not verified headset compatibility.</p>{data.assets.vrFeatures.map(item => <code className="apk-path" key={item}>{item}</code>)}{!data.assets.vrFeatures.length && <p className="metadata-empty">No VR declarations were found in the base manifest.</p>}</>}
      </div>
      {data.assets.notes.length > 0 && <details className="metadata-notes"><summary>Metadata availability</summary>{data.assets.notes.map(note => <p key={note}>{note}</p>)}</details>}
    </>}
  </>;
}
