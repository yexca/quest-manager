import { createContext, useCallback, useContext, useEffect, useId, useMemo, useRef, useState, type FormEvent, type ReactNode } from 'react';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { open } from '@tauri-apps/plugin-dialog';
import { Activity, AppWindow, ArrowDownToLine, ArrowLeft, ArrowRight, ArrowUpFromLine, BatteryCharging, Box, Check, CheckCircle2, ChevronRight, CircleHelp, Clock3, Download, File, FileArchive, FileImage, FileText, Film, Folder, FolderOpen, FolderPlus, HardDrive, Info, LayoutDashboard, ListTodo, LoaderCircle, Package, Pencil, Plus, RefreshCw, Search, ShieldCheck, Trash2, Unplug, Usb, Wifi, X, XCircle } from 'lucide-react';
import { api, isDesktop, isPreview } from './api';
import { AppIcon, ApplicationDetails, UninstallSummary, formatBytes, type UninstallSelection } from './AppMetadata';
import { InstallReview } from './InstallReview';
import { About, appVersion } from './About';
import { useDeviceDiscovery } from './useDeviceDiscovery';
import { useTaskQueue } from './useTaskQueue';
import { useApplications } from './useApplications';
import { installSource, sourceLabels, type InstallSource } from './applicationState';
import { isActiveTask as active } from './taskState';
import appLogo from '../src-tauri/icons/icon.png';
import type { AppDetails, AppPackage, Device, DeviceInfo, FileEntry, Task, TaskRequest } from './types';

type Page = 'overview' | 'apps' | 'files' | 'about';
type ConfirmAction = { title: string; description: string; action: string; danger?: boolean; uninstall?: UninstallSelection; run: () => Promise<void> };
type NameAction = { title: string; initial: string; run: (name: string) => Promise<void> };
const pageNames: Record<Page, string> = { overview: 'Overview', apps: 'Applications', files: 'Files', about: 'About' };
const ErrorContext = createContext<string | null>(null);
const errorText = (error: unknown) => error instanceof Error ? error.message : String(error);
function bytes(value: number, digits = 1) {
  if (value === 0) return '0 B';
  const index = Math.min(Math.floor(Math.log(value) / Math.log(1024)), 4);
  return `${(value / 1024 ** index).toFixed(index === 0 ? 0 : digits)} ${['B', 'KB', 'MB', 'GB', 'TB'][index]}`;
}

function IconButton({ label, children, ...props }: { label: string; children: ReactNode } & React.ButtonHTMLAttributes<HTMLButtonElement>) {
  return <button type="button" className="icon-button" aria-label={label} title={label} {...props}>{children}</button>;
}

function Modal({ title, children, onClose, wide = false }: { title: string; children: ReactNode; onClose: () => void; wide?: boolean }) {
  const ref = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  const error = useContext(ErrorContext);
  useEffect(() => { ref.current?.showModal(); }, []);
  return <dialog ref={ref} className={`modal ${wide ? 'wide' : ''}`} aria-labelledby={titleId} onCancel={event => { event.preventDefault(); onClose(); }} onClick={e => { if (e.target === ref.current) onClose(); }}>
    <div className="modal-title"><h2 id={titleId}>{title}</h2><IconButton label="Close dialog" onClick={onClose}><X size={19} /></IconButton></div>{error && <p className="dialog-error" role="alert">{error}</p>}{children}
  </dialog>;
}

function NameDialog({ action, onClose, onError }: { action: NameAction; onClose: () => void; onError: (error: unknown) => void }) {
  const [name, setName] = useState(action.initial);
  const [saving, setSaving] = useState(false);
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setSaving(true);
    try { await action.run(name); onClose(); } catch (error) { onError(error); } finally { setSaving(false); }
  };
  return <Modal title={action.title} onClose={onClose}><form onSubmit={submit}><label className="field-label" htmlFor="item-name">Name</label><input autoFocus id="item-name" className="text-input" value={name} onChange={e => setName(e.target.value)} required maxLength={240} /><div className="modal-actions"><button type="button" className="button secondary" onClick={onClose}>Cancel</button><button className="button primary" disabled={!name.trim() || saving}>{saving ? 'Saving…' : 'Save'}</button></div></form></Modal>;
}

function Headset() {
  return <svg className="headset" viewBox="0 0 440 280" fill="none" aria-hidden="true">
    <defs><linearGradient id="visor" x1="135" y1="83" x2="256" y2="241" gradientUnits="userSpaceOnUse"><stop stopColor="#fff" /><stop offset="1" stopColor="#cdd2cc" /></linearGradient><linearGradient id="strap" x1="115" y1="45" x2="302" y2="113" gradientUnits="userSpaceOnUse"><stop stopColor="#7e8b7e" /><stop offset="1" stopColor="#eef1e9" /></linearGradient><filter id="shadow"><feGaussianBlur stdDeviation="10" /></filter></defs>
    <ellipse cx="231" cy="239" rx="123" ry="16" fill="#152f20" opacity=".19" filter="url(#shadow)" />
    <path d="M145 139C112 55 218 6 290 66L319 127" stroke="url(#strap)" strokeWidth="27" />
    <path d="M147 137C125 72 218 29 273 75L301 130" stroke="#afbaae" strokeWidth="4" />
    <path d="M210 105 201 51 230 44 257 104" fill="#e9ede5" /><path d="m218 96-12-43 17-5 20 46" fill="#fafbf5" />
    <path d="M109 121c38-33 153-54 218-12 20 13 25 44 11 64l-44 44c-11 11-31 17-49 15l-112-20c-23-5-40-23-40-47 0-18 4-32 16-44Z" fill="#687369" />
    <path d="M94 128c18-28 146-43 207-25 22 6 35 23 34 46l-4 35c-2 26-24 45-50 46l-133-9c-35-3-59-18-62-44-2-18-1-35 8-49Z" fill="url(#visor)" />
    <path d="M105 132c37-24 130-28 181-18" stroke="white" strokeWidth="3" strokeLinecap="round" opacity=".9" />
    <ellipse cx="105" cy="144" rx="7" ry="8" fill="#414a41" /><ellipse cx="310" cy="132" rx="6" ry="7" fill="#414a41" />
    <ellipse cx="102" cy="194" rx="6" ry="7" fill="#414a41" /><ellipse cx="304" cy="203" rx="6" ry="7" fill="#414a41" />
    <path d="M185 163c7-12 20-12 27 0 8 12 21 12 27 0" stroke="#a7b0a7" strokeWidth="3.5" strokeLinecap="round" />
    <rect x="145" y="218" width="34" height="5" rx="2.5" fill="#7f8b7e" />
  </svg>;
}

function FileIcon({ file, size = 20 }: { file: FileEntry; size?: number }) {
  if (file.kind === 'directory') return <Folder size={size} />;
  if (/\.apk$/i.test(file.name)) return <Package size={size} />;
  if (/\.(mp4|mkv|webm|mov)$/i.test(file.name)) return <Film size={size} />;
  if (/\.(jpg|png|jpeg|webp)$/i.test(file.name)) return <FileImage size={size} />;
  if (/\.(zip|obb|7z)$/i.test(file.name)) return <FileArchive size={size} />;
  if (/\.(txt|json|log)$/i.test(file.name)) return <FileText size={size} />;
  return <File size={size} />;
}

function TaskRow({ task, devices, cancel }: { task: Task; devices: Device[]; cancel: (id: string) => void }) {
  const cancellable = task.status === 'queued' || (task.status === 'running' && ['upload', 'download', 'export'].includes(task.kind));
  const device = devices.find(device => device.transports.some(transport => transport.serial === task.device));
  const transport = device?.transports.find(transport => transport.serial === task.device);
  return <div className={`task-row task-${task.status}`}>
    <div className="task-icon">{task.status === 'success' ? <CheckCircle2 size={20} /> : task.status === 'failed' ? <XCircle size={20} /> : task.status === 'running' ? <LoaderCircle size={20} className="spin" /> : <Clock3 size={20} />}</div>
    <div className="task-copy"><strong>{task.label}</strong><small className="task-target">Target: {device ? `${device.model} · ${transport?.kind === 'usb' ? 'USB' : 'Wi-Fi'} · ` : ''}{task.device}</small><p>{task.detail}</p>{task.status === 'running' && <div className={`progress-track ${task.progress === null ? 'indeterminate' : ''}`}><span style={{ width: `${task.progress ?? 38}%` }} /></div>}</div>
    <div className="task-status">{task.status === 'running' && task.progress !== null ? `${Math.round(task.progress)}%` : task.status}</div>
    {cancellable && <IconButton label={`Cancel ${task.label}`} disabled={!isDesktop} onClick={() => cancel(task.id)}><X size={16} /></IconButton>}
  </div>;
}

export default function App() {
  const [page, setPage] = useState<Page>('overview');
  const [deviceId, setDeviceId] = useState('');
  const [preferredTransport, setPreferredTransport] = useState('');
  const [info, setInfo] = useState<DeviceInfo | null>(null);
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [path, setPath] = useState('/sdcard');
  const [pathInput, setPathInput] = useState('/sdcard');
  const [query, setQuery] = useState('');
  const [fileQuery, setFileQuery] = useState('');
  const [includeSystem, setIncludeSystem] = useState(false);
  const [sourceFilter, setSourceFilter] = useState<InstallSource | 'all'>('all');
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [showTasks, setShowTasks] = useState(false);
  const [exitBlocked, setExitBlocked] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const [installPaths, setInstallPaths] = useState<string[] | null>(null);
  const [installing, setInstalling] = useState(false);
  const [confirmAction, setConfirmAction] = useState<ConfirmAction | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [nameAction, setNameAction] = useState<NameAction | null>(null);
  const [details, setDetails] = useState<AppDetails | null>(null);
  const [inspecting, setInspecting] = useState<AppPackage | null>(null);
  const [detailsError, setDetailsError] = useState<string | null>(null);
  const [clearingMetadata, setClearingMetadata] = useState(false);
  const detailsRequest = useRef(0);
  const [error, setError] = useState<string | null>(null);
  const [refreshToken, setRefreshToken] = useState(0);
  const [loadingFiles, setLoadingFiles] = useState(false);
  const [dragging, setDragging] = useState(false);
  const fail = useCallback((cause: unknown) => setError(errorText(cause)), []);
  const { devices, loadingDevices } = useDeviceDiscovery(refreshToken, fail);
  const selectedDevice = devices.find(d => d.id === deviceId) ?? devices[0];
  const transport = selectedDevice?.transports.find(t => t.serial === preferredTransport && t.state === 'device') ?? selectedDevice?.transports.find(t => t.state === 'device');
  const serial = transport?.serial ?? '';
  const ready = Boolean(serial);
  const canWrite = ready && isDesktop;
  const uninstallReady = !confirmAction?.uninstall || (isDesktop && devices.some(device => device.transports.some(item => item.serial === confirmAction.uninstall?.device && item.state === 'device')));
  const { tasks, appMutations, versions, start: startTask, clearCompleted, clearing } = useTaskQueue(selectedDevice?.transports.map(transport => transport.serial) ?? [], fail);
  const { apps, metadata: appMetadata, errors: metadataErrors, sources, loadingApps, loadingMetadata, metadataPaused, setMetadataPaused, resumeMetadata, readDetails } = useApplications(
    serial, selectedDevice?.transports.map(item => item.serial) ?? [], page === 'apps', includeSystem, refreshToken, versions.apps, appMutations, fail,
  );
  const running = tasks.filter(active).length;
  const orderedTasks = useMemo(() => [...tasks].sort((a, b) => b.createdAt - a.createdAt), [tasks]);
  const userApps = apps.filter(app => !app.system);
  const visibleApps = apps.filter(app => (includeSystem || !app.system) && (sourceFilter === 'all' || (sources[app.packageName] ?? installSource(app)) === sourceFilter)
    && `${app.packageName} ${appMetadata[app.packageName]?.assets.displayName ?? ''}`.toLowerCase().includes(query.toLowerCase()));
  const visibleFiles = files.filter(file => file.name.toLowerCase().includes(fileQuery.toLowerCase()));
  const chosenFiles = files.filter(file => selected.has(file.path));
  const inspectedMetadata = inspecting ? appMetadata[inspecting.packageName] : undefined;
  const refresh = () => setRefreshToken(token => token + 1);

  useEffect(() => {
    if (!isDesktop) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen('app-close-blocked', () => { if (!disposed) setExitBlocked(true); })
      .then(stop => { if (disposed) stop(); else unlisten = stop; }).catch(fail);
    return () => { disposed = true; unlisten?.(); };
  }, [fail]);

  useEffect(() => { setInfo(null); }, [serial]);

  useEffect(() => {
    let alive = true;
    if (!serial) return;
    const load = () => api.info(serial).then(value => { if (alive) setInfo(value); }).catch(cause => { if (alive) fail(cause); });
    void load();
    const timer = setInterval(() => { void load(); }, 30000);
    return () => { alive = false; clearInterval(timer); };
  }, [serial, refreshToken, versions.info, fail]);

  useEffect(() => {
    setInspecting(null); setDetails(null); detailsRequest.current += 1;
  }, [serial]);

  useEffect(() => {
    if (!inspecting || loadingApps) return;
    if (!apps.some(app => app.packageName === inspecting.packageName)) {
      setDetailsError('This application is no longer in the current app list.');
      return;
    }
  }, [apps, inspecting, loadingApps, serial]);

  useEffect(() => {
    if (inspectedMetadata) { setDetails(inspectedMetadata); setDetailsError(null); }
  }, [inspectedMetadata]);

  useEffect(() => { setFiles([]); setSelected(new Set()); }, [serial, path]);

  useEffect(() => {
    let alive = true;
    if (!serial || page !== 'files') { setLoadingFiles(false); return; }
    setLoadingFiles(true);
    void api.files(serial, path).then(value => { if (alive) { setFiles(value); setSelected(current => new Set([...current].filter(path => value.some(file => file.path === path)))); } }).catch(cause => { if (alive) fail(cause); }).finally(() => { if (alive) setLoadingFiles(false); });
    return () => { alive = false; };
  }, [serial, path, page, refreshToken, versions.files, fail]);

  const queue = async (request: Omit<TaskRequest, 'device'>, target = serial) => {
    if (!target) throw new Error('Connect and authorize a headset first.');
    await startTask({ ...request, device: target });
    setShowTasks(true);
  };

  const reviewUninstall = (app: AppPackage) => {
    if (app.system || !serial || (!canWrite && !isPreview)) return;
    // Capture display data and the exact operation target together.
    const target = serial;
    setConfirmAction({
      title: 'Uninstall application?', description: 'This application and its local app data will be removed from the headset.',
      action: 'Uninstall', danger: true,
      uninstall: { app, data: appMetadata[app.packageName], device: target, targetLabel: `${selectedDevice?.model || 'Headset'} · ${transport?.kind === 'usb' ? 'USB' : 'Wi-Fi'}` },
      run: () => queue({ kind: 'uninstall', packageName: app.packageName }, target),
    });
  };

  const goToFolder = (nextPath: string) => {
    setPath(nextPath); setPathInput(nextPath); setFileQuery(''); setError(null); setPage('files');
  };

  const selectApks = async () => {
    if (isPreview) { setInstallPaths(['Orbit Adventures.apk', 'Orbit Adventures update.apk', 'Orbit Adventures older.apk', 'Unknown app.apk', 'System Shell.apk'].map(name => `C:\\Example\\${name}`)); return; }
    try {
      const paths = await open({ multiple: true, title: 'Choose APK files', filters: [{ name: 'Android application', extensions: ['apk'] }] });
      if (paths) setInstallPaths(Array.isArray(paths) ? paths : [paths]);
    } catch (cause) { fail(cause); }
  };

  const upload = async (directory: boolean) => {
    try {
      const result = await open({ multiple: !directory, directory, title: directory ? 'Choose a folder to upload' : 'Choose files to upload' });
      if (!result) return;
      const paths = Array.isArray(result) ? result : [result];
      for (const source of paths) await queue({ kind: 'upload', source, destination: path });
    } catch (cause) { fail(cause); }
  };

  const download = async (entries: FileEntry[]) => {
    try {
      const destination = await open({ directory: true, title: 'Choose a download folder' });
      if (typeof destination !== 'string') return;
      for (const file of entries) await queue({ kind: 'download', source: file.path, destination });
    } catch (cause) { fail(cause); }
  };

  const exportApp = async (app: AppPackage) => {
    try {
      const destination = await open({ directory: true, title: 'Choose an APK export folder' });
      if (typeof destination === 'string') await queue({ kind: 'export', packageName: app.packageName, destination });
    } catch (cause) { fail(cause); }
  };

  const inspectApp = async (app: AppPackage) => {
    const request = ++detailsRequest.current;
    setInspecting(app); setDetails(appMetadata[app.packageName] ?? null); setDetailsError(null);
    try {
      const result = await readDetails(app);
      if (result && request === detailsRequest.current) setDetails(result);
    } catch (cause) { if (request === detailsRequest.current) setDetailsError(errorText(cause)); }
  };

  const clearMetadata = async () => {
    setMetadataPaused(true); setClearingMetadata(true);
    try { await api.clearMetadataCache(); } catch (cause) { fail(cause); } finally { setClearingMetadata(false); }
  };

  const requestDelete = (entries: FileEntry[]) => {
    const target = serial;
    setConfirmAction({ title: entries.length === 1 ? `Delete “${entries[0].name}”?` : `Delete ${entries.length} items?`, description: 'These items will be permanently deleted from the headset. Folders include everything inside them.', action: 'Delete', danger: true, run: async () => { for (const file of entries) await queue({ kind: 'delete', source: file.path }, target); } });
  };

  useEffect(() => {
    if (!isDesktop) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void getCurrentWebview().onDragDropEvent(event => {
      if (disposed) return;
      if (event.payload.type === 'over' || event.payload.type === 'enter') setDragging(true);
      if (event.payload.type === 'leave') setDragging(false);
      if (event.payload.type === 'drop') {
        setDragging(false);
        if (installing) return;
        const paths = event.payload.paths;
        if (paths.length && paths.every(item => /\.apk$/i.test(item))) setInstallPaths(paths);
        else if (!serial) { fail('Connect and authorize a headset before uploading files.'); return; }
        else if (page === 'files') {
          const target = serial;
          setConfirmAction({ title: `Upload ${paths.length} item(s)?`, description: `The selected items will be copied to ${path}.`, action: 'Upload', run: async () => { for (const source of paths) await queue({ kind: 'upload', source, destination: path }, target); } });
        } else fail('Drop APK files to install them, or open Files to upload other items.');
      }
    }).then(stop => { if (disposed) stop(); else unlisten = stop; }).catch(fail);
    return () => { disposed = true; unlisten?.(); };
  }, [serial, page, path, fail, installing]);

  const cancelTask = (id: string) => { void api.cancel(id).catch(fail); };
  const choosePage = (nextPage: Page) => { setPage(nextPage); setError(null); };
  const storageRatio = info && info.storageTotal > 0 ? info.storageUsed / info.storageTotal : 0;
  const statusLabel = ready ? 'Connected' : selectedDevice?.transports.some(t => t.state === 'unauthorized') ? 'Authorization needed' : 'No device connected';

  return <ErrorContext.Provider value={error}><div className="app-shell">
    <aside className="sidebar">
      <a className="brand" href="#" onClick={e => { e.preventDefault(); choosePage('overview'); }}><span className="brand-icon"><img src={appLogo} alt="" width="39" height="39" /></span><span>quest<span className="brand-secondary">manager</span></span></a>
      <div className="sidebar-label">YOUR WORKSPACE</div>
      <nav aria-label="Main navigation">
        {([{ id: 'overview', icon: LayoutDashboard }, { id: 'apps', icon: AppWindow }, { id: 'files', icon: FolderOpen }] as const).map(item => <button key={item.id} className={`nav-item ${page === item.id ? 'selected' : ''}`} aria-current={page === item.id ? 'page' : undefined} onClick={() => choosePage(item.id)}><item.icon size={19} /><span>{pageNames[item.id]}</span>{item.id === 'apps' && ready && <span className="nav-count">{userApps.length}</span>}</button>)}
      </nav>
      <button className="sidebar-install" onClick={() => void selectApks()} disabled={(!isDesktop && !isPreview) || installing}><Plus size={18} />Install an APK<ArrowRight size={16} /></button>
      <div className="sidebar-bottom">
        <div className="sidebar-storage"><div><HardDrive size={17} /><span>Headset storage</span></div><strong>{info ? bytes(info.storageAvailable) : '—'} <span>free</span></strong><div className="sidebar-meter"><span style={{ width: `${storageRatio * 100}%` }} /></div><p>{info ? `${bytes(info.storageUsed)} of ${bytes(info.storageTotal)} used` : 'Connect a device to view storage'}</p></div>
        <button className="sidebar-utility" onClick={() => setShowTasks(true)}><ListTodo size={18} />Task queue{running > 0 && <span className="queue-count">{running}</span>}</button>
        <button className="sidebar-utility" onClick={() => setShowHelp(true)}><CircleHelp size={18} />Help</button>
        <button className={`sidebar-utility ${page === 'about' ? 'selected' : ''}`} aria-current={page === 'about' ? 'page' : undefined} onClick={() => choosePage('about')}><Info size={18} />About<small>v{appVersion}</small></button>
        <div className="sidebar-footer"><span className={`status-dot ${ready ? 'online' : ''}`} />{ready ? 'Your headset is ready' : 'Waiting for a headset'}</div>
      </div>
    </aside>

    <div className="main-shell">
      <header className="topbar"><div className="breadcrumb-top">Workspace<ChevronRight size={14} /><span>{pageNames[page]}</span></div><div className="topbar-actions">{isPreview && <span className="preview-badge">Preview · sample data</span>}<span className={`connection-pill ${ready ? '' : 'offline'}`}><span className="status-dot online" />{statusLabel}</span><IconButton label="Refresh device data" onClick={refresh} disabled={loadingDevices}><RefreshCw size={17} className={loadingDevices ? 'spin' : ''} /></IconButton></div></header>
      <main>
        {error && <div className="error-banner" role="alert"><Info size={18} /><p>{error}</p><IconButton label="Dismiss error" onClick={() => setError(null)}><X size={16} /></IconButton></div>}
        <div className="page-heading"><div><p className="eyebrow">{page === 'about' ? 'THE PROJECT BEHIND THE APP.' : page === 'overview' ? 'A LITTLE ORDER. MORE ROOM TO PLAY.' : 'YOUR HEADSET, ORGANIZED.'}</p><h1>{page === 'about' ? 'About Quest Manager' : page === 'overview' ? 'Device overview' : page === 'apps' ? 'Your applications' : 'File explorer'}</h1><p className="page-description">{page === 'about' ? 'The people, tools and license behind your workspace.' : page === 'overview' ? 'Everything on your Quest, within reach.' : page === 'apps' ? 'Install, inspect and manage the apps on your headset.' : 'Move files between your computer and your Quest.'}</p></div>{(page === 'overview' || page === 'apps') && <button className="button primary" disabled={(!isDesktop && !isPreview) || installing} onClick={() => void selectApks()}><Plus size={17} />Install APK</button>}</div>

        {page === 'about' ? <About /> : !ready ? <div className="empty-device card"><div className="empty-device-icon"><Unplug size={36} /></div><h2>{loadingDevices ? 'Looking for your headset…' : statusLabel}</h2><p>{selectedDevice?.transports.some(t => t.state === 'unauthorized') ? 'Put on your headset and accept the USB debugging prompt, then refresh.' : 'Connect your Quest with a USB cable, enable developer mode, and allow USB debugging in the headset.'}</p><button className="button primary" onClick={refresh} disabled={loadingDevices}><RefreshCw size={16} />Check connection</button></div> : <>
          {page === 'overview' && <>
            <section className="device-hero"><div className="hero-copy"><div className="hero-kicker"><span className="status-dot online" />CONNECTED DEVICE</div><h2>{info?.model ?? selectedDevice?.model}</h2><p>Your next session starts here.</p><div className="device-badges"><span>{transport?.kind === 'wifi' ? <Wifi size={14} /> : <Usb size={14} />}{transport?.kind === 'wifi' ? 'Wi-Fi connection' : 'USB connection'}</span><span>Android {info?.androidVersion ?? '—'}</span></div><div className="hero-device-picker"><label htmlFor="device-picker">Device</label><select id="device-picker" value={selectedDevice?.id ?? ''} onChange={e => { setDeviceId(e.target.value); setPreferredTransport(''); setPath('/sdcard'); setPathInput('/sdcard'); }}>{devices.map(device => <option key={device.id} value={device.id}>{device.model} · {device.id}</option>)}</select>{selectedDevice && selectedDevice.transports.filter(t => t.state === 'device').length > 1 && <select aria-label="Connection method" value={serial} onChange={e => setPreferredTransport(e.target.value)}>{selectedDevice.transports.filter(t => t.state === 'device').map(t => <option key={t.serial} value={t.serial}>{t.kind === 'usb' ? 'USB' : 'Wi-Fi'}</option>)}</select>}</div></div><div className="hero-art"><div className="orbit orbit-one" /><div className="orbit orbit-two" /><Headset /><div className="hero-art-caption"><ShieldCheck size={13} />Ready for your next adventure</div></div></section>
            <div className="stats-grid"><div className="stat card"><div className="stat-heading"><span>Installed apps</span><span className="stat-icon lilac"><AppWindow size={18} /></span></div><div className="stat-value">{loadingApps ? '—' : userApps.length}<span>apps</span></div><button className="text-button" onClick={() => choosePage('apps')}>Manage applications<ArrowRight size={14} /></button></div><div className="stat card"><div className="stat-heading"><span>Available storage</span><span className="stat-icon peach"><HardDrive size={18} /></span></div><div className="stat-value">{info ? bytes(info.storageAvailable).split(' ')[0] : '—'}<span>{info ? bytes(info.storageAvailable).split(' ')[1] : 'GB'}</span></div><div className="storage-foot"><div className={`storage-meter ${storageRatio > .9 ? 'low-space' : ''}`}><span style={{ width: `${storageRatio * 100}%` }} /></div><span>{Math.round(storageRatio * 100)}% used</span></div></div><div className="stat card"><div className="stat-heading"><span>Battery level</span><span className="stat-icon mint"><BatteryCharging size={19} /></span></div><div className="stat-value">{info?.batteryLevel ?? '—'}<span>%</span></div><p className="stat-note"><span className="status-dot online" />{info?.charging ? 'Connected to power' : 'Running on battery'}</p></div></div>
            <div className="section-heading"><h2>Make yourself at home</h2><span>The essentials, one click away</span></div>
            <div className="quick-grid"><button className="quick-card card" disabled={(!isDesktop && !isPreview) || installing} onClick={() => void selectApks()}><span className="quick-icon"><Package size={24} /></span><span><strong>Something new to play</strong><small>Choose an APK from your computer</small></span><ArrowRight size={18} /></button><button className="quick-card card" onClick={() => goToFolder('/sdcard')}><span className="quick-icon"><FolderOpen size={24} /></span><span><strong>A place for every file</strong><small>Browse, transfer and organize</small></span><ArrowRight size={18} /></button></div>
            <section className="activity card"><div className="section-heading"><h2><Activity size={18} />Recent activity</h2><button className="text-button" onClick={() => setShowTasks(true)}>View all<ArrowRight size={14} /></button></div>{orderedTasks.length ? orderedTasks.slice(0, 3).map(task => <TaskRow key={task.id} task={task} devices={devices} cancel={cancelTask} />) : <div className="activity-empty"><span><Check size={18} /></span><div><strong>All clear. Ready when you are.</strong><p>Your installs and transfers will appear here.</p></div><span className="quiet-label">A fresh start</span></div>}</section>
          </>}

          {page === 'apps' && <section className="card list-card">
            <div className="list-toolbar app-filters"><div className="search-field"><Search size={17} /><input aria-label="Search applications" placeholder="Search by app name or package…" value={query} onChange={e => setQuery(e.target.value)} /></div><label className="source-filter">Install source<select aria-label="Filter by install source" value={sourceFilter} onChange={e => setSourceFilter(e.target.value as InstallSource | 'all')}><option value="all">All sources</option>{Object.entries(sourceLabels).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label><label className="checkbox-label"><input type="checkbox" checked={includeSystem} onChange={e => setIncludeSystem(e.target.checked)} />Show system apps</label><span className="result-count">{visibleApps.length} applications</span></div>
            <p className="source-explanation">Sources are inferred from the reported installer or installs completed here this session. Missing records stay unknown; they do not prove an app came from outside the store.</p>
            <div className="table-scroll"><table className="app-table"><thead><tr><th>APPLICATION</th><th>VERSION</th><th>APK SIZE</th><th>TYPE / STATE</th><th>INSTALL SOURCE</th><th className="align-right">ACTIONS</th></tr></thead><tbody>{visibleApps.map((app, index) => {
              const data = appMetadata[app.packageName];
              return <tr key={app.packageName}><td><div className="app-cell"><AppIcon data={data} index={index} /><div><button className="package-button" onClick={() => void inspectApp(app)}>{data?.assets.displayName ?? app.packageName}</button><small className="app-package-id">{app.packageName}</small>{metadataErrors[app.packageName] && <small className="metadata-row-error">Details unavailable · click to retry</small>}</div></div></td>
                <td className="muted version-cell"><span>{data?.versionName ?? '—'}</span><small className="mono">{app.versionCode}</small></td><td className="muted tabular apk-size-cell">{formatBytes(data?.apkSize)}</td>
                <td><span className={app.system ? 'type-badge system' : 'type-badge'}>{app.system ? 'System' : 'Installed'}</span><small className={data?.enabled === false ? 'app-state disabled' : 'app-state'}>{!data ? '—' : data.enabled == null ? 'State unavailable' : data.enabled ? 'Enabled' : 'Disabled'}</small></td>
                <td className="app-source" title={`Reported installer: ${app.installer ?? 'Unavailable'}. Source is not proof of ownership or publisher identity.`}>{sourceLabels[sources[app.packageName] ?? installSource(app)]}</td>
                <td><div className="row-actions"><IconButton label={'Details for ' + app.packageName} onClick={() => void inspectApp(app)}><Info size={16} /></IconButton><IconButton label={'Export ' + app.packageName} disabled={!canWrite} onClick={() => void exportApp(app)}><Download size={16} /></IconButton><IconButton label={'Uninstall ' + app.packageName} disabled={(!canWrite && !isPreview) || app.system} onClick={() => reviewUninstall(app)}><Trash2 size={16} /></IconButton></div></td></tr>;
            })}</tbody></table></div>
            {!visibleApps.length && <div className="list-empty">{loadingApps ? <><LoaderCircle className="spin" size={24} /><p>Reading installed applications…</p></> : <><Search size={28} /><p>No applications found.</p></>}</div>}
            <div className="list-footer metadata-footer">{loadingMetadata || loadingApps ? <LoaderCircle size={14} className="spin" /> : <Info size={14} />}<span>{loadingApps ? 'Refreshing application list…' : loadingMetadata ? 'Loading new or changed app details…' : 'APK size excludes app data, cache and OBB files.'}</span><button className="text-button" disabled={clearingMetadata} onClick={() => metadataPaused ? resumeMetadata() : void clearMetadata()}>{clearingMetadata ? 'Clearing…' : metadataPaused ? 'Load app details' : 'Clear cached artwork'}</button></div>
          </section>}

          {page === 'files' && <>
            <div className="folder-shortcuts">{[{ name: 'Shared storage', path: '/sdcard', icon: HardDrive }, { name: 'Downloads', path: '/sdcard/Download', icon: ArrowDownToLine }, { name: 'Movies', path: '/sdcard/Movies', icon: Film }, { name: 'OBB files', path: '/sdcard/Android/obb', icon: Box }].map(shortcut => <button key={shortcut.path} className={path === shortcut.path ? 'selected' : ''} onClick={() => goToFolder(shortcut.path)}><shortcut.icon size={17} />{shortcut.name}</button>)}</div>
            <section className="card list-card"><div className="path-toolbar"><IconButton label="Parent folder" disabled={path === '/sdcard'} onClick={() => goToFolder(path.slice(0, path.lastIndexOf('/')) || '/sdcard')}><ArrowLeft size={18} /></IconButton><form onSubmit={e => { e.preventDefault(); goToFolder(pathInput.replace(/\/$/, '') || '/sdcard'); }}><Folder size={17} /><input aria-label="Current folder path" value={pathInput} onChange={e => setPathInput(e.target.value)} spellCheck={false} /><button type="submit" aria-label="Go to folder"><ArrowRight size={17} /></button></form><IconButton label="Refresh folder" onClick={refresh}><RefreshCw size={17} className={loadingFiles ? 'spin' : ''} /></IconButton></div>
              <div className="list-toolbar file-toolbar"><div className="file-toolbar-actions"><button className="button primary small" disabled={!canWrite} onClick={() => void upload(false)}><ArrowUpFromLine size={16} />Upload files</button><button className="button secondary small" disabled={!canWrite} onClick={() => void upload(true)}><FolderPlus size={16} />Upload folder</button><button className="button secondary small" disabled={!canWrite} onClick={() => { const target = serial; setNameAction({ title: 'New folder', initial: '', run: name => queue({ kind: 'mkdir', source: path, destination: name }, target) }); }}><Plus size={16} />New folder</button></div><div className="search-field compact"><Search size={16} /><input aria-label="Filter files" placeholder="Filter files…" value={fileQuery} onChange={e => setFileQuery(e.target.value)} /></div></div>
              {chosenFiles.length > 0 && <div className="selection-toolbar"><span>{chosenFiles.length} selected</span><button className="text-button" disabled={!canWrite} onClick={() => void download(chosenFiles)}><Download size={15} />Download</button><button className="text-button danger-text" disabled={!canWrite} onClick={() => requestDelete(chosenFiles)}><Trash2 size={15} />Delete</button><button className="text-button" onClick={() => setSelected(new Set())}>Clear selection</button></div>}
              <div className="table-scroll"><table className="file-table"><thead><tr><th className="checkbox-cell"><input aria-label="Select all visible files" type="checkbox" checked={visibleFiles.length > 0 && visibleFiles.every(file => selected.has(file.path))} onChange={e => setSelected(e.target.checked ? new Set(visibleFiles.map(file => file.path)) : new Set())} /></th><th>NAME</th><th>SIZE</th><th>MODIFIED</th><th className="align-right">ACTIONS</th></tr></thead><tbody>{visibleFiles.map(file => <tr key={file.path} className={selected.has(file.path) ? 'row-selected' : ''}><td className="checkbox-cell"><input aria-label={`Select ${file.name}`} type="checkbox" checked={selected.has(file.path)} onChange={e => setSelected(current => { const next = new Set(current); if (e.target.checked) next.add(file.path); else next.delete(file.path); return next; })} /></td><td><div className={`file-cell ${file.kind === 'directory' ? 'folder-cell' : ''}`}><FileIcon file={file} />{file.kind === 'directory' ? <button className="filename-button" onClick={() => goToFolder(file.path)}>{file.name}</button> : <span>{file.name}</span>}{file.kind === 'symlink' && <span className="type-badge">Link</span>}</div></td><td className="muted tabular">{file.kind === 'directory' ? '—' : bytes(file.size)}</td><td className="muted date-cell">{new Date(file.modifiedAt).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}</td><td><div className="row-actions"><IconButton label={`Download ${file.name}`} disabled={!canWrite} onClick={() => void download([file])}><Download size={16} /></IconButton><IconButton label={`Rename ${file.name}`} disabled={!canWrite} onClick={() => { const target = serial; setNameAction({ title: 'Rename item', initial: file.name, run: name => queue({ kind: 'rename', source: file.path, destination: name }, target) }); }}><Pencil size={15} /></IconButton><IconButton label={`Delete ${file.name}`} disabled={!canWrite} onClick={() => requestDelete([file])}><Trash2 size={16} /></IconButton></div></td></tr>)}</tbody></table></div>{!visibleFiles.length && <div className="list-empty">{loadingFiles ? <><LoaderCircle size={26} className="spin" /><p>Reading this folder…</p></> : <><FolderOpen size={32} /><p>{fileQuery ? 'No files match your search.' : 'This folder is empty.'}</p><span>Upload files or drop them here to get started.</span></>}</div>}<div className="list-footer"><span>{files.length} items</span><span>Shared storage · /sdcard</span></div>
            </section>
          </>}
        </>}
        <footer className="main-footer"><span>Quest Manager</span><span>Made for a little more control.</span><span><ShieldCheck size={13} />Local connection</span></footer>
      </main>
    </div>

    {showTasks && <Modal title="Task queue" onClose={() => setShowTasks(false)} wide><p className="modal-description">{running ? `${running} task(s) in progress. You can keep browsing while they run.` : 'Installs, transfers and file operations from this session.'}</p><div className="queue-toolbar"><button className="button secondary small" disabled={!isDesktop || clearing || !tasks.some(task => !active(task))} onClick={() => void clearCompleted()}>{clearing ? 'Clearing…' : 'Clear completed'}</button></div><div className="task-list">{orderedTasks.length ? orderedTasks.map(task => <TaskRow key={task.id} task={task} devices={devices} cancel={cancelTask} />) : <div className="list-empty"><ListTodo size={32} /><p>No tasks yet</p><span>Your next install or transfer will appear here.</span></div>}</div></Modal>}
    {installPaths && <Modal title="Install applications" onClose={() => { if (!installing) setInstallPaths(null); }} wide><InstallReview key={installPaths.join('|')} paths={installPaths} target={selectedDevice?.model} device={serial} appRevision={`${refreshToken}:${versions.apps}`} canInstall={canWrite} onQueue={queue} onClose={() => setInstallPaths(null)} onBusy={setInstalling} /></Modal>}
    {confirmAction && <Modal title={confirmAction.title} onClose={() => { if (!confirming) setConfirmAction(null); }}>
      {confirmAction.uninstall && <UninstallSummary selection={confirmAction.uninstall} />}
      <p className="modal-description preserve-lines">{confirmAction.description}</p>
      {!uninstallReady && <p className="inline-note">{isPreview ? 'Preview only. Uninstallation is disabled.' : 'The original headset connection is unavailable. Reconnect it to uninstall.'}</p>}
      <div className="modal-actions"><button className="button secondary" disabled={confirming} onClick={() => setConfirmAction(null)}>Cancel</button><button className={`button ${confirmAction.danger ? 'danger' : 'primary'}`} disabled={confirming || !uninstallReady} onClick={() => { setConfirming(true); void confirmAction.run().then(() => setConfirmAction(null)).catch(fail).finally(() => setConfirming(false)); }}>{confirming ? 'Queuing…' : confirmAction.action}</button></div>
    </Modal>}
    {nameAction && <NameDialog action={nameAction} onClose={() => setNameAction(null)} onError={fail} />}
    {inspecting && <Modal title="Application details" wide onClose={() => { setInspecting(null); setDetails(null); detailsRequest.current += 1; }}><ApplicationDetails key={inspecting.packageName} app={inspecting} source={sources[inspecting.packageName]} data={details} error={detailsError} onRetry={() => void inspectApp(inspecting)} /></Modal>}
    {showHelp && <Modal title="A little help getting connected" onClose={() => setShowHelp(false)}><div className="help-section"><Usb size={21} /><div><h3>Connect your headset</h3><p>Enable developer mode for your Quest. Connect a USB data cable, put on the headset and allow USB debugging.</p></div></div><div className="help-section"><Wifi size={21} /><div><h3>Already connected over Wi-Fi?</h3><p>Existing ADB Wi-Fi connections appear automatically. When both connections are available, USB is selected by default.</p></div></div><div className="help-section"><HardDrive size={21} /><div><h3>Know your storage</h3><p>Files manages shared storage, including accessible Android/data and Android/obb folders. Access depends on the headset's permissions.</p></div></div><div className="help-section"><Package size={21} /><div><h3>Install and transfer</h3><p>Drop APK files into this window to install them. Other files and folders can be dropped into File explorer. Existing files are never silently replaced.</p></div></div><div className="help-section"><ShieldCheck size={21} /><div><h3>Local APK signing keys</h3><p>Modified APKs use a stable key for each application. Back up the entire signing-keys folder, including its password files. In development it is in env/local-data/apk-install; in the installed app it is in %LOCALAPPDATA%/dev.questmanager.desktop/apk-install. Keep backups private. Clearing artwork does not delete keys. Restoring the same keys allows compatible modified updates; losing them can prevent updates without reinstalling.</p></div></div><div className="about-footer">Quest Manager {appVersion}<span>Tauri 2 · Local ADB connection</span></div></Modal>}
    {dragging && <div className="drop-overlay"><div><ArrowUpFromLine size={45} /><h2>Drop it here</h2><p>APKs install on your headset. Other files upload to the open folder.</p></div></div>}
    {running > 0 && !showTasks && <button className="floating-queue" onClick={() => setShowTasks(true)}><LoaderCircle size={17} className="spin" />{running} task{running > 1 ? 's' : ''} in progress<ChevronRight size={16} /></button>}
    {exitBlocked && <Modal title="Tasks are still running" onClose={() => setExitBlocked(false)}><p className="modal-description">Wait for queued and running tasks to finish before closing Quest Manager. Exiting now can interrupt work and leave temporary files. Tasks cannot resume after restart, and changes already made are not undone.</p><div className="modal-actions"><button className="button danger" onClick={() => void api.exitWithActiveTasks().catch(fail)}>Exit anyway</button><button autoFocus className="button primary" onClick={() => setExitBlocked(false)}>Keep waiting</button></div></Modal>}
  </div></ErrorContext.Provider>;
}
