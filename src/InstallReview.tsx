import { useEffect, useRef, useState, type RefObject } from 'react';
import { Check, FilePlus2, Gamepad2, ImagePlus, Info, LoaderCircle, Package, RotateCcw, X } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { api, isPreview } from './api';
import type { InstallOptions, LocalApk, LocalObb, TaskRequest } from './types';
import { duplicatePackages, installationStatus, observeInstalledApps, type InstalledSnapshot } from './installState';
import { canAttachObbs, mergeObbs, obbOptions } from './obbState';

type Item = { source: string; details?: LocalApk; error?: string; enabled: boolean; name: string; icon: string | null; compatibility: boolean; obbs: LocalObb[] };
export type InstallDropHandler = (paths: string[]) => void;
const basename = (path: string) => path.split(/[\\/]/).pop() || path;
const errorText = (error: unknown) => error instanceof Error ? error.message : String(error);
const sizeText = (size: number) => size >= 1024 ** 3 ? `${(size / 1024 ** 3).toFixed(2)} GiB` : `${(size / 1024 ** 2).toFixed(1)} MiB`;

export function InstallReview({ paths, target, device, appRevision, canInstall, onQueue, onClose, onBusy, dropHandler }: {
  paths: string[]; target: string | undefined; device: string; appRevision: string; canInstall: boolean;
  onQueue: (request: Omit<TaskRequest, 'device'>, device: string) => Promise<void>; onClose: () => void; onBusy: (busy: boolean) => void;
  dropHandler: RefObject<InstallDropHandler | null>;
}) {
  const [items, setItems] = useState<Item[]>(() => [...new Set(paths)].map(source => ({ source, enabled: false, name: '', icon: null, compatibility: false, obbs: [] })));
  const itemsRef = useRef(items); itemsRef.current = items;
  const alive = useRef(true);
  const obbPending = useRef(false);
  const [obbLoading, setObbLoading] = useState(false);
  const [selected, setSelected] = useState(paths[0]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [installed, setInstalled] = useState<InstalledSnapshot | null>(null);
  const [checkRevision, setCheckRevision] = useState(0);
  const checkKey = JSON.stringify([device, appRevision, checkRevision]);
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [imageLoading, setImageLoading] = useState(false);
  const [zoom, setZoom] = useState(1);
  const [offset, setOffset] = useState({ x: 50, y: 50 });
  const canvas = useRef<HTMLCanvasElement>(null);
  const imageRequest = useRef(0);
  const item = items.find(row => row.source === selected) ?? items[0];
  const source = item?.source;
  const duplicates = duplicatePackages(items);
  const statusFor = (row: Item) => installationStatus(row.details?.packageName, row.details?.versionCode ?? '', device, checkKey, installed);
  const status = item ? statusFor(item) : null;
  const obbAllowed = canAttachObbs(item?.details);

  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);

  const addObbs = async (pick: () => Promise<string[]>, capturedSource: string | undefined) => {
    if (busy || obbPending.current || !capturedSource) return;
    if (!canAttachObbs(itemsRef.current.find(row => row.source === capturedSource)?.details)) {
      setError('OBB files require a readable APK package name.'); return;
    }
    obbPending.current = true; setObbLoading(true); setError(null);
    try {
      const selectedPaths = await pick();
      if (!alive.current || !selectedPaths.length) return;
      if (selectedPaths.some(path => !/\.obb$/i.test(path))) throw new Error('Drop only .obb files for the selected APK.');
      const files = await api.inspectObbs([...new Set(selectedPaths)]);
      if (!alive.current) return;
      const row = itemsRef.current.find(candidate => candidate.source === capturedSource);
      if (!row || !canAttachObbs(row.details)) return;
      const obbs = mergeObbs(row.obbs, files);
      setItems(rows => rows.map(candidate => candidate.source === capturedSource ? { ...candidate, obbs } : candidate));
    } catch (cause) { if (alive.current) setError(errorText(cause)); }
    finally { obbPending.current = false; if (alive.current) setObbLoading(false); }
  };

  useEffect(() => {
    dropHandler.current = paths => { void addObbs(() => Promise.resolve(paths), source); };
    return () => { dropHandler.current = null; };
  });

  const chooseObbs = () => addObbs(async () => {
    if (isPreview) return ['C:\\Example\\main.100.com.example.orbit.obb', 'C:\\Example\\audio.obb'];
    const result = await open({ multiple: true, directory: false, title: 'Choose OBB files', filters: [{ name: 'OBB files', extensions: ['obb'] }] });
    return result ? Array.isArray(result) ? result : [result] : [];
  }, source);

  useEffect(() => {
    if (!device) { setInstalled(null); return; }
    return observeInstalledApps(checkKey, device, api.apps, setInstalled);
  }, [device, checkKey]);

  useEffect(() => {
    let disposed = false;
    void (async () => {
      for (const path of [...new Set(paths)]) {
        try {
          const details = await api.inspectApk(path);
          if (disposed) return;
          setItems(rows => rows.map(row => row.source === path ? { ...row, details, name: details.assets.displayName ?? '' } : row));
        } catch (cause) {
          if (disposed) return;
          setItems(rows => rows.map(row => row.source === path ? { ...row, error: errorText(cause) } : row));
        }
      }
    })();
    return () => { disposed = true; imageRequest.current++; };
  }, [paths]);

  useEffect(() => { imageRequest.current++; setImage(null); setImageLoading(false); }, [source]);
  useEffect(() => {
    const context = canvas.current?.getContext('2d');
    if (!context || !image) return;
    const side = Math.min(image.width, image.height) / zoom;
    context.clearRect(0, 0, 512, 512);
    context.drawImage(image, (image.width - side) * offset.x / 100, (image.height - side) * offset.y / 100, side, side, 0, 0, 512, 512);
  }, [image, zoom, offset]);

  const change = (patch: Partial<Item>) => setItems(rows => rows.map(row => row.source === source ? { ...row, ...patch } : row));
  const optionsFor = (row: Item): InstallOptions | undefined => row.enabled && row.details ? {
    sourceStamp: row.details.sourceStamp,
    displayName: row.name === (row.details.assets.displayName ?? '') ? null : row.name.trim(),
    iconPng: row.icon?.split(',')[1] ?? null,
    compatibility: row.compatibility,
  } : undefined;
  const invalid = (row: Item) => {
    if (!row.enabled) return false;
    const options = optionsFor(row);
    return !options || row.details?.split || (options.displayName !== null && (!options.displayName || [...options.displayName].length > 120))
      || (!options.compatibility && options.displayName === null && options.iconPng === null);
  };
  const editing = item && (item.name !== (item.details?.assets.displayName ?? '') || item.icon !== null);
  const icon = item?.enabled && item.icon ? item.icon : item?.details?.assets.iconDataUrl;
  const title = item?.enabled ? item.name : item?.details?.assets.displayName;

  const chooseImage = async (file: File | undefined) => {
    if (!file) return;
    const request = ++imageRequest.current;
    setError(null); setImage(null); setImageLoading(true);
    try {
      if (file.size > 8 * 1024 ** 2 || !['image/png', 'image/jpeg', 'image/webp'].includes(file.type)) throw new Error('Choose a PNG, JPEG or WebP image smaller than 8 MiB.');
      const data = await new Promise<string>((resolve, reject) => { const reader = new FileReader(); reader.onload = () => resolve(String(reader.result)); reader.onerror = () => reject(new Error('Could not read the image.')); reader.readAsDataURL(file); });
      const loaded = new Image(); loaded.src = data; await loaded.decode();
      if (loaded.width * loaded.height > 32 * 1024 ** 2) throw new Error('Choose an image with fewer than 32 million pixels.');
      if (request !== imageRequest.current) return;
      setZoom(1); setOffset({ x: 50, y: 50 }); setImage(loaded);
    } catch (cause) { if (request === imageRequest.current) setError(errorText(cause)); }
    finally { if (request === imageRequest.current) setImageLoading(false); }
  };

  const submit = async () => {
    if (!canInstall || !device || busy || obbPending.current) return;
    const capturedDevice = device;
    setBusy(true); onBusy(true); setError(null);
    try {
      for (const row of items) {
        await onQueue({ kind: 'install', source: row.source, packageName: row.details?.packageName, installOptions: optionsFor(row), obb: obbOptions(row.details, row.obbs) }, capturedDevice);
        // Remove queued rows immediately so a partial failure cannot queue them twice.
        setItems(rows => rows.filter(candidate => candidate.source !== row.source));
      }
      onClose();
    } catch (cause) { setError(errorText(cause)); }
    finally { setBusy(false); onBusy(false); }
  };

  return <>
    <p className="modal-description">Install on <strong>{device ? target || device : 'No headset connected'}</strong>. Review each APK before adding it to the queue.</p>
    {isPreview && <p className="install-notice">Preview · fictional APKs. Installation is disabled.</p>}
    {error && <p className="dialog-error" role="alert">{error}</p>}
    <div className="install-review">
      <nav className="apk-picker" aria-label="Selected APKs">{items.map(row => <div className={row.source === source ? 'selected' : ''} key={row.source}>
        <button className="apk-select" disabled={busy} onClick={() => { setSelected(row.source); setError(null); }}><Package size={17} /><span>{basename(row.source)}<small>{row.details ? sizeText(row.details.size) : row.error ? 'Preview unavailable' : 'Reading APK…'}</small>{row.details && <small className={statusFor(row).attention ? 'apk-attention' : ''}>{statusFor(row).label}</small>}{row.details && duplicates.has(row.details.packageName) && <small className="apk-attention">Duplicate package selected</small>}{row.obbs.length > 0 && <small>{row.obbs.length} OBB files attached</small>}</span>{row.enabled && <span className="modified-dot" aria-label="Modified installation" />}</button>
        <button className="icon-button" title="Remove APK" aria-label={`Remove ${basename(row.source)}`} disabled={busy} onClick={() => setItems(rows => rows.filter(candidate => candidate.source !== row.source))}><X size={14} /></button>
      </div>)}</nav>
      {item ? <section className="apk-editor" aria-label="APK review">
        <div className="apk-preview"><div className="apk-art">{icon ? <img src={icon} alt="APK icon preview" /> : <Gamepad2 size={34} />}</div><div><span className="eyebrow">{item.enabled && editing ? 'MODIFIED PREVIEW' : 'APK PREVIEW'}</span><h3>{title || 'Name unavailable'}</h3><code>{item.details?.packageName ?? basename(item.source)}</code><p>{item.details && `Version ${item.details.versionName || item.details.versionCode || 'Unknown'} · ${sizeText(item.details.size)}`}</p></div></div>
        {!item.details && !item.error && <p className="inline-note"><LoaderCircle className="spin" size={15} />Reading application resources…</p>}
        {item.error && <p className="install-notice">{item.error} You can still try installing the original APK.</p>}
        {status && <div className={`apk-install-status ${status.attention ? 'attention' : ''}`} aria-label="Headset installation status">
          <div className="apk-status-heading"><strong role="status">{status.label}</strong>{device && <button className="button secondary" disabled={busy || (installed?.key === checkKey && installed.status === 'loading')} onClick={() => setCheckRevision(value => value + 1)}>Check again</button>}</div>
          {status.installed && <>
            <p>Version code: <strong>{status.installed.versionCode || 'Unknown'}</strong> on headset → <strong>{item.details?.versionCode || 'Unknown'}</strong> in APK{status.installed.system ? ' · System application' : ''}</p>
            {status.comparison === 'same' && <p>This will attempt to replace the installed app. Matching version codes do not mean the APK files are identical.</p>}
            {status.comparison === 'older' && <p>Android normally refuses lower version codes. This installer does not force downgrades.</p>}
            <p>{item.enabled ? 'This package is already installed. Re-signing may prevent updating it with this copy. Existing apps are never uninstalled automatically.' : 'Updating requires a compatible signature and must pass Android installation checks.'}</p>
          </>}
          {device && installed?.key === checkKey && installed.status === 'error' && <p>{installed.error} You can check again or still try installing.</p>}
          {device && installed?.key === checkKey && installed.status === 'ready' && <p className="apk-note">Status reflects the last check. Other installations or queued tasks may change it.</p>}
        </div>}
        {item.details && duplicates.has(item.details.packageName) && <p className="install-notice">Multiple selected APKs use this package name. Each will be queued in list order and may replace the previous installation. Remove any copies you do not intend to install.</p>}
        {item.details && !icon && <p className="inline-note">Icon preview is unavailable. The APK may use an adaptive or vector icon.</p>}
        {item.details?.assets.notes.map(note => <p className="apk-note" key={note}>{note}</p>)}
        <section className={`obb-attachments ${obbAllowed ? '' : 'unavailable'}`} aria-label="OBB files" aria-disabled={!obbAllowed}>
          <div className="obb-heading"><div><strong>OBB files <span>Optional</span></strong><p>Choose or drop files for this APK. Original filenames are preserved.</p></div><button className="button secondary" disabled={busy || obbLoading || !obbAllowed} onClick={() => void chooseObbs()}><FilePlus2 size={16} />Add OBB files</button></div>
          {!obbAllowed ? <p className="apk-note">{item.details?.split ? 'OBB files are unavailable for split APKs.' : 'A readable APK package name is required. OBB selection and drops are disabled.'}</p> : <>
            <p className="obb-destination">Destination <code>/sdcard/Android/obb/{item.details!.packageName}/</code></p>
            {item.obbs.length ? <ul className="obb-list">{item.obbs.map(file => <li key={file.source}><div><strong>{file.name}</strong><small>{sizeText(file.size)}</small></div><button className="icon-button" title="Remove OBB" aria-label={`Remove ${file.name}`} disabled={busy || obbLoading} onClick={() => change({ obbs: item.obbs.filter(candidate => candidate.source !== file.source) })}><X size={14} /></button></li>)}</ul> : <p className="obb-empty">Drop one or more .obb files anywhere in this review to attach them to the selected APK.</p>}
            {item.obbs.length > 0 && <p className="apk-note">{item.obbs.length} OBB file{item.obbs.length === 1 ? '' : 's'} · {sizeText(item.obbs.reduce((total, file) => total + file.size, 0))}. Installs with this APK as one task. Identical existing files are reused; different files are never overwritten.</p>}
          </>}
          {obbLoading && <p className="inline-note" role="status"><LoaderCircle className="spin" size={15} />Reading selected OBB files…</p>}
        </section>
        {item.details?.split && <p className="install-notice">This package requires a split APK installation flow, which is not supported here.</p>}
        {item.details && item.details.size > 2 * 1024 ** 3 && item.details.veritySigning && <p className="install-notice">This large APK uses a signature algorithm that may overflow on some headsets. If ordinary installation fails with an integer overflow, try Compatibility install.</p>}
        <label className="apk-switch"><input type="checkbox" checked={item.enabled} disabled={busy || !item.details || item.details.split} onChange={e => { change({ enabled: e.target.checked }); setImage(null); setImageLoading(false); imageRequest.current++; }} /><span><strong>Modify and re-sign APK</strong><small>Creates a prepared copy with a local signing key. The original APK is preserved.</small></span></label>
        {item.enabled && <div className="apk-modifications">
          <p className="install-notice"><Info size={16} /><span>Changing the name or icon rebuilds APK resources. A new signature usually cannot update an original installation. Existing apps are never uninstalled automatically; signature-dependent game features may be affected.</span></p>
          <label className="apk-name">App display name<input value={item.name} maxLength={120} disabled={busy} onChange={e => change({ name: e.target.value })} placeholder="Enter a readable name" /></label>
          <p className="apk-note">Applies one display name to the app and its launcher entry across languages.</p>
          <div className="apk-image-actions"><label className={`button secondary ${busy ? 'disabled' : ''}`}><ImagePlus size={16} />Choose icon<input type="file" accept="image/png,image/jpeg,image/webp" disabled={busy} onChange={e => { void chooseImage(e.target.files?.[0]); e.target.value = ''; }} /></label><button className="button secondary" disabled={busy} onClick={() => { change({ name: item.details?.assets.displayName ?? '', icon: null }); setImage(null); setImageLoading(false); imageRequest.current++; }}><RotateCcw size={14} />Reset appearance</button></div>
          {imageLoading && <p className="inline-note"><LoaderCircle className="spin" size={15} />Reading icon…</p>}
          {image && <div className="icon-crop"><canvas width={512} height={512} ref={canvas} aria-label="Cropped icon preview" /><div><label>Zoom<input type="range" min="1" max="3" step="0.05" value={zoom} onChange={e => setZoom(Number(e.target.value))} /></label><label>Horizontal position<input type="range" min="0" max="100" value={offset.x} onChange={e => setOffset({ ...offset, x: Number(e.target.value) })} /></label><label>Vertical position<input type="range" min="0" max="100" value={offset.y} onChange={e => setOffset({ ...offset, y: Number(e.target.value) })} /></label><button className="button secondary" onClick={() => { const png = canvas.current?.toDataURL('image/png'); if (png) change({ icon: png }); setImage(null); }}><Check size={15} />Use this icon</button></div></div>}
          <div className="apk-compatibility"><label className="apk-switch"><input type="checkbox" checked={editing || item.compatibility} disabled={busy || editing} onChange={e => change({ compatibility: e.target.checked })} /><span><strong>Compatibility install</strong><small>{editing ? 'Included when changing the name or icon.' : 'Re-signs a copy without changing its name or icon.'}</small></span></label><p className="apk-note">Some APKs larger than 2 GiB may fail signature verification with an integer overflow. If this occurs, try this option. It may not resolve other installation errors.</p></div>
          {invalid(item) && <p className="apk-note">{!item.name.trim() && editing ? 'Enter a display name.' : 'Change the name or icon, or enable Compatibility install.'}</p>}
          <p className="apk-note">Keep a backup of your local signing keys for future modified updates. Keys are separate from the artwork cache; see Help.</p>
        </div>}
        {!item.enabled && <p className="apk-note">The original APK and its signature will be used. Compatible updates normally retain app data.</p>}
      </section> : <p className="modal-description">No APKs selected.</p>}
    </div>
    <div className="modal-actions"><span className="apk-footer-note">{items.length} APK{items.length === 1 ? '' : 's'} · {items.reduce((count, row) => count + row.obbs.length, 0)} OBB files · One task per APK</span><button className="button secondary" disabled={busy} onClick={onClose}>Cancel</button><button className="button primary" disabled={!canInstall || busy || obbLoading || imageLoading || !!image || items.length === 0 || items.some(row => (!row.details && !row.error) || invalid(row) || row.details?.split)} onClick={() => void submit()}><Package size={16} />{busy ? 'Queuing…' : `Install ${items.length > 1 ? `${items.length} APKs` : 'APK'}`}</button></div>
  </>;
}
