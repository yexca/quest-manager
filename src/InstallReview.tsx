import { useEffect, useRef, useState } from 'react';
import { Check, Gamepad2, ImagePlus, Info, LoaderCircle, Package, RotateCcw, X } from 'lucide-react';
import { api, isPreview } from './api';
import type { InstallOptions, LocalApk, TaskRequest } from './types';

type Item = { source: string; details?: LocalApk; error?: string; enabled: boolean; name: string; icon: string | null; compatibility: boolean };
const basename = (path: string) => path.split(/[\\/]/).pop() || path;
const errorText = (error: unknown) => error instanceof Error ? error.message : String(error);
const sizeText = (size: number) => size >= 1024 ** 3 ? `${(size / 1024 ** 3).toFixed(2)} GiB` : `${(size / 1024 ** 2).toFixed(1)} MiB`;

export function InstallReview({ paths, target, canInstall, onQueue, onClose, onBusy }: {
  paths: string[]; target: string | undefined; canInstall: boolean;
  onQueue: (request: Omit<TaskRequest, 'device'>) => Promise<void>; onClose: () => void; onBusy: (busy: boolean) => void;
}) {
  const [items, setItems] = useState<Item[]>(() => [...new Set(paths)].map(source => ({ source, enabled: false, name: '', icon: null, compatibility: false })));
  const [selected, setSelected] = useState(paths[0]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [imageLoading, setImageLoading] = useState(false);
  const [zoom, setZoom] = useState(1);
  const [offset, setOffset] = useState({ x: 50, y: 50 });
  const canvas = useRef<HTMLCanvasElement>(null);
  const imageRequest = useRef(0);
  const item = items.find(row => row.source === selected) ?? items[0];
  const source = item?.source;

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
    setBusy(true); onBusy(true); setError(null);
    try {
      for (const row of items) {
        await onQueue({ kind: 'install', source: row.source, installOptions: optionsFor(row) });
        // Remove queued rows immediately so a partial failure cannot queue them twice.
        setItems(rows => rows.filter(candidate => candidate.source !== row.source));
      }
      onClose();
    } catch (cause) { setError(errorText(cause)); }
    finally { setBusy(false); onBusy(false); }
  };

  return <>
    <p className="modal-description">Install on <strong>{target || 'No headset connected'}</strong>. Review each APK before adding it to the queue.</p>
    {isPreview && <p className="install-notice">Preview · fictional APKs. Installation is disabled.</p>}
    {error && <p className="dialog-error" role="alert">{error}</p>}
    <div className="install-review">
      <nav className="apk-picker" aria-label="Selected APKs">{items.map(row => <div className={row.source === source ? 'selected' : ''} key={row.source}>
        <button className="apk-select" disabled={busy} onClick={() => { setSelected(row.source); setError(null); }}><Package size={17} /><span>{basename(row.source)}<small>{row.details ? sizeText(row.details.size) : row.error ? 'Preview unavailable' : 'Reading APK…'}</small></span>{row.enabled && <span className="modified-dot" aria-label="Modified installation" />}</button>
        <button className="icon-button" title="Remove APK" aria-label={`Remove ${basename(row.source)}`} disabled={busy} onClick={() => setItems(rows => rows.filter(candidate => candidate.source !== row.source))}><X size={14} /></button>
      </div>)}</nav>
      {item ? <section className="apk-editor" aria-label="APK review">
        <div className="apk-preview"><div className="apk-art">{icon ? <img src={icon} alt="APK icon preview" /> : <Gamepad2 size={34} />}</div><div><span className="eyebrow">{item.enabled && editing ? 'MODIFIED PREVIEW' : 'APK PREVIEW'}</span><h3>{title || 'Name unavailable'}</h3><code>{item.details?.packageName ?? basename(item.source)}</code><p>{item.details && `Version ${item.details.versionName || item.details.versionCode} · ${sizeText(item.details.size)}`}</p></div></div>
        {!item.details && !item.error && <p className="inline-note"><LoaderCircle className="spin" size={15} />Reading application resources…</p>}
        {item.error && <p className="install-notice">{item.error} You can still try installing the original APK.</p>}
        {item.details && !icon && <p className="inline-note">Icon preview is unavailable. The APK may use an adaptive or vector icon.</p>}
        {item.details?.assets.notes.map(note => <p className="apk-note" key={note}>{note}</p>)}
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
    <div className="modal-actions"><span className="apk-footer-note">{items.length} APK{items.length === 1 ? '' : 's'} · Each file is a separate task</span><button className="button secondary" disabled={busy} onClick={onClose}>Cancel</button><button className="button primary" disabled={!canInstall || busy || imageLoading || !!image || items.length === 0 || items.some(row => (!row.details && !row.error) || invalid(row) || row.details?.split)} onClick={() => void submit()}><Package size={16} />{busy ? 'Queuing…' : `Install ${items.length > 1 ? `${items.length} APKs` : 'APK'}`}</button></div>
  </>;
}
