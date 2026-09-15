import { useEffect, useRef, useState } from 'react';
import { ArrowUpRight, CheckCircle2, Download, Info, LoaderCircle, RefreshCw, ShieldCheck, Zap } from 'lucide-react';
import { api, isDesktop, isPreview } from './api';
import { lightningPackage, lightningVariants, matchingInstalledRelease, matchingRelease, navigatorPackage, recommendedService, serviceChoiceFor } from './lightningState';
import type { ServiceChoice } from './lightningState';
import { isActiveTask } from './taskState';
import { formatBytes } from './AppMetadata';
import type { AppPackage, LightningCatalog, LightningRecommendation, Task, TaskRequest } from './types';

const message = (error: unknown) => error instanceof Error ? error.message : String(error);
type Installed = { apps: AppPackage[]; launcherName: string | null; launcherVersion: string; serviceVersion: string; enabled: boolean | null };

export function LightningSetup({ device, target, available, revision, tasks, onQueue, onClose }: {
  device: string; target: string; available: boolean; revision: number; tasks: Task[];
  onQueue: (request: TaskRequest) => Promise<Task>; onClose: () => void;
}) {
  const [catalog, setCatalog] = useState<LightningCatalog | null>(null);
  const [releaseError, setReleaseError] = useState('');
  const [releaseLoading, setReleaseLoading] = useState(true);
  const [reload, setReload] = useState(0);
  const [launcherId, setLauncherId] = useState<number | null>(null);
  const [installLauncher, setInstallLauncher] = useState(true);
  const [installService, setInstallService] = useState(false);
  const [serviceChoice, setServiceChoice] = useState<ServiceChoice | null>(null);
  const [recommendation, setRecommendation] = useState<{ key: string; data?: LightningRecommendation; error?: string } | null>(null);
  const [installed, setInstalled] = useState<Installed | null>(null);
  const [inventoryError, setInventoryError] = useState('');
  const [inventoryLoading, setInventoryLoading] = useState(true);
  const [check, setCheck] = useState(0);
  const initialized = useRef(false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState('');
  const [taskId, setTaskId] = useState<string | null>(null);
  const [submittedTask, setSubmittedTask] = useState<Task | null>(null);
  const task = tasks.find(task => task.id === taskId) ?? submittedTask;
  const busy = submitting || !!(task && isActiveTask(task));
  const launcher = catalog?.launchers.find(item => item.assetId === launcherId);
  const installedRelease = installed ? matchingRelease(installed.launcherVersion, catalog?.launchers ?? []) : undefined;
  const installedLauncherRelease = matchingInstalledRelease(installed?.launcherName, installed?.launcherVersion ?? '', catalog);
  const installedServiceRelease = matchingInstalledRelease(navigatorPackage, installed?.serviceVersion ?? '', catalog);
  const matchTag = installLauncher ? launcher?.tag : installedRelease?.tag;
  const matchKey = `${matchTag ?? ''}:${reload}`;
  const match = recommendation?.key === matchKey ? recommendation : null;
  const recommended = recommendedService(match?.data?.launcherTag === matchTag ? match?.data?.navigatorTag : null, catalog);
  const { assetId: serviceId, otherVersions } = serviceChoiceFor(matchKey, serviceChoice, recommended);
  const service = catalog?.navigators.find(item => item.assetId === serviceId);
  const hasLauncher = !!installed?.launcherName;
  const hasService = !!installed?.apps.some(app => app.packageName === navigatorPackage);
  const serviceChoices = otherVersions ? catalog?.navigators ?? [] : recommended ? [recommended] : [];
  const serviceAllowed = !!service && (!!recommended && service.tag === recommended.tag || otherVersions);
  const canSubmit = isDesktop && available && !busy && !taskId && !inventoryLoading && !inventoryError && !!installed
    && !releaseLoading && !releaseError && (installLauncher || installService)
    && (!installLauncher || !!launcher) && (!installService || serviceAllowed) && (installLauncher || hasLauncher);

  useEffect(() => {
    let alive = true;
    setReleaseLoading(true); setReleaseError('');
    void api.lightningReleases(reload > 0).then(data => {
      if (!alive) return;
      setCatalog(data);
      setLauncherId(current => data.launchers.some(item => item.assetId === current) ? current : data.launchers[0]?.assetId ?? null);
    }).catch(error => { if (alive) setReleaseError(message(error)); }).finally(() => { if (alive) setReleaseLoading(false); });
    return () => { alive = false; };
  }, [reload]);

  useEffect(() => {
    let alive = true;
    setInventoryLoading(true); setInventoryError('');
    void (async () => {
      try {
        const apps = await api.apps(device, true);
        const main = lightningVariants.map(name => apps.find(app => app.packageName === name)).find(Boolean);
        const navigator = apps.find(app => app.packageName === navigatorPackage);
        const [mainDetails, serviceDetails, enabled] = await Promise.all([
          main ? api.details(device, main.packageName).catch(() => null) : null,
          navigator ? api.details(device, navigatorPackage).catch(() => null) : null,
          navigator ? api.navigatorEnabled(device).catch(() => null) : false,
        ]);
        if (!alive) return;
        setInstalled({ apps, launcherName: main?.packageName ?? null, launcherVersion: mainDetails?.versionName ?? '', serviceVersion: serviceDetails?.versionName ?? '', enabled });
        if (!initialized.current) { setInstallLauncher(!main); initialized.current = true; }
      } catch (error) { if (alive) setInventoryError(message(error)); }
      finally { if (alive) setInventoryLoading(false); }
    })();
    return () => { alive = false; };
  }, [device, revision, check, task?.status]);

  useEffect(() => {
    let alive = true;
    setRecommendation(null); setServiceChoice(null);
    if ((!installService && !hasService) || !matchTag) return;
    void api.lightningRecommendation(matchTag).then(data => {
      if (!alive) return;
      setRecommendation({ key: matchKey, data });
    }).catch(error => { if (alive) setRecommendation({ key: matchKey, error: message(error) }); });
    return () => { alive = false; };
  }, [installService, hasService, matchTag, matchKey, catalog]);

  const submit = async () => {
    if (!canSubmit) return;
    setSubmitting(true); setError('');
    try {
      const result = await onQueue({ device, kind: 'install', lightning: { launcherAsset: installLauncher ? launcherId : null, navigatorAsset: installService ? serviceId : null } });
      setTaskId(result.id); setSubmittedTask(result);
    } catch (error) { setError(message(error)); }
    finally { setSubmitting(false); }
  };

  return <div className="lightning-setup">
    <div className="lightning-intro"><div className="lightning-mark"><Zap size={28} /></div><div><h3>Your apps, one button away.</h3><p>A launcher for your headset, with an optional Meta button shortcut.</p><button className="text-button" onClick={() => void api.openLightningRepository().catch(error => setError(message(error)))}>By threethan · View on GitHub<ArrowUpRight size={13} /></button></div></div>
    <div className="lightning-target"><span className="quiet-label">INSTALL ON</span><strong>{target}</strong><small>{device}</small></div>
    {!available && <p className="dialog-error" role="alert">The selected connection is unavailable. Reconnect this headset to install.</p>}
    {isPreview && <p className="lightning-note"><Info size={15} />Preview uses fictional versions. Downloads and device writes are disabled.</p>}
    {inventoryError && <p className="dialog-error" role="alert">Device check failed: {inventoryError} <button className="text-button" onClick={() => setCheck(value => value + 1)}>Retry device check</button></p>}
    {releaseLoading ? <div className="lightning-loading" role="status"><LoaderCircle size={19} className="spin" />Loading releases from GitHub…</div> : releaseError ? <div className="dialog-error" role="alert">{releaseError}<button className="text-button" onClick={() => setReload(value => value + 1)}>Retry loading versions</button></div> : <>
      <fieldset className="lightning-options" disabled={busy || !!taskId || inventoryLoading}>
        <section className="lightning-component">
          <label className="lightning-choice"><input type="checkbox" checked={installLauncher} onChange={event => setInstallLauncher(event.target.checked)} /><span><strong>{hasLauncher ? 'Install / update Lightning Launcher' : 'Install Lightning Launcher'}</strong><small>{inventoryLoading ? 'Checking this headset…' : hasLauncher ? `Installed${installed?.launcherVersion ? ` · ${installed.launcherVersion}` : ' · version unavailable'}` : 'Not installed on this headset'}</small></span>{hasLauncher && <CheckCircle2 size={17} />}</label>
          {hasLauncher && installed.launcherName !== lightningPackage && <p className="lightning-note">A store variant is installed. The GitHub edition installs alongside it.</p>}
          {installLauncher && <div className="lightning-version"><label htmlFor="lightning-version">Launcher version</label><select id="lightning-version" value={launcherId ?? ''} onChange={event => setLauncherId(Number(event.target.value))}>{catalog?.launchers.map((release, index) => <option key={release.assetId} value={release.assetId}>{release.tag}{release.assetId === installedLauncherRelease?.assetId ? ' · Installed' : ''}{index === 0 ? ' · Latest stable' : ''}</option>)}</select>{launcher && <><small>{formatBytes(launcher.size)} · Released {launcher.publishedAt.slice(0, 10)}{!launcher.sha256 && ' · Publisher checksum unavailable; APK signature will be verified'}</small>{launcher.assetId === installedLauncherRelease?.assetId && <p className="lightning-installed" role="status"><CheckCircle2 size={15} />This Launcher version is already installed.</p>}</>}</div>}
        </section>

        <section className={`lightning-component ${installService ? 'selected' : ''}`}>
          <label className="lightning-choice"><input type="checkbox" checked={installService} onChange={event => { setInstallService(event.target.checked); setServiceChoice(null); }} /><span><strong>{hasService ? 'Install / update' : 'Install'} Navigator Button Redirection Service</strong><small>{hasService ? `Installed${installed?.serviceVersion ? ` · ${installed.serviceVersion}` : ' · version unavailable'} · ${installed?.enabled === null ? 'Activation status unavailable' : installed?.enabled ? 'Enabled' : 'Not enabled'}` : 'Optional · Open Lightning Launcher with the Meta button'}</small></span></label>
          {(installService || hasService) && <div className="lightning-compatibility" aria-live="polite">
            {!matchTag ? <p className="lightning-note">The installed launcher version could not be matched to a release. Select a launcher version to install, or review other service versions.</p> : !match ? <p className="lightning-note"><LoaderCircle size={15} className="spin" />Checking the author’s recommendation for {matchTag}…</p> : match.error ? <p className="lightning-note">Compatibility information unavailable: {match.error}</p> : !recommended ? <p className="lightning-note">No available Navigator release is explicitly recommended for this launcher version.</p> : <>
              <p className="lightning-note">Launcher {matchTag} → recommended Service {recommended.tag.replace(/^addons/, '')}.</p>
              {hasService && (installedServiceRelease?.assetId === recommended.assetId ? <p className="lightning-note"><CheckCircle2 size={15} />The installed service matches the author’s recommendation.</p> : <p className="lightning-warning">{installed?.serviceVersion ? `Installed Service ${installed.serviceVersion} differs from the recommendation. Compatibility is unverified.` : 'The installed service version is unavailable. Compatibility could not be checked.'}{!installService && ' Select the service above to install the recommended version with this launcher.'}</p>)}
            </>}
          </div>}
          {installService && <div className="lightning-service-body">
            {!installLauncher && !hasLauncher && <p className="lightning-warning">Select Lightning Launcher above to install both. The Navigator service needs an installed launcher.</p>}
            <p>Press the Meta button to open Lightning Launcher. Once it is open, press again to access the original Navigator.</p>
            <div className="lightning-permission"><Info size={17} /><span><strong>One step in your headset</strong>After installation, enable the service in Android Accessibility settings. Quest Manager does not enable it automatically. Requires the Navigator interface.</span></div>
            <div className="lightning-version"><label htmlFor="navigator-version">Service version</label><select id="navigator-version" value={serviceId ?? ''} onChange={event => setServiceChoice({ key: matchKey, assetId: Number(event.target.value), otherVersions })} disabled={!serviceChoices.length}><option value="" disabled>Select a service version</option>{serviceChoices.map(release => <option key={release.assetId} value={release.assetId}>{release.tag.replace(/^addons/, '')}{release.assetId === installedServiceRelease?.assetId ? ' · Installed' : ''}{release.tag === recommended?.tag ? ` · Recommended for ${matchTag}` : ' · Compatibility unverified'}</option>)}</select>{service && <><small>{formatBytes(service.size)} · Released {service.publishedAt.slice(0, 10)}</small>{service.assetId === installedServiceRelease?.assetId && <p className="lightning-installed" role="status"><CheckCircle2 size={15} />This Service version is already installed.</p>}</>}</div>
            <label className="checkbox-label lightning-other"><input type="checkbox" checked={otherVersions} onChange={event => setServiceChoice({ key: matchKey, assetId: recommended?.assetId ?? null, otherVersions: event.target.checked })} />Show other service versions</label>
            {service && service.tag !== recommended?.tag && <p className="lightning-warning">Compatibility with this launcher version is unverified. This selection may not provide a working Meta button shortcut.</p>}
          </div>}
        </section>
      </fieldset>
      <div className="lightning-source"><ShieldCheck size={15} /><span>Original APKs from the author’s GitHub releases. Published checksums are checked when available; APK signatures are verified. No re-signing.</span><button className="text-button" disabled={busy || !!taskId} onClick={() => setReload(value => value + 1)}><RefreshCw size={13} />Refresh versions</button></div>
    </>}
    {error && <p className="dialog-error" role="alert">{error}</p>}
    {task && <div className={`lightning-result ${task.status === 'failed' ? 'failed' : ''}`} role="status"><strong>{task.status === 'queued' ? 'Waiting in the task queue' : task.status === 'running' ? 'Installing on your headset' : task.status === 'success' ? 'Installation complete' : task.status === 'cancelled' ? 'Installation cancelled' : 'Setup needs attention'}</strong><p>{task.detail}</p>{isActiveTask(task) && <div className={`progress-track ${task.progress === null ? 'indeterminate' : ''}`}><span style={{ width: `${task.progress ?? 38}%` }} /></div>}</div>}
    {hasService && !task && <div className="lightning-guide"><strong>{installed?.enabled ? 'Navigator shortcut is enabled' : 'Enable the shortcut in your headset'}</strong><p>Lightning Launcher → Settings → Shortcuts → Activate → Accessibility settings. Turn on “Open Lightning Launcher with Navigator Button”.</p><p>If Android shows “Restricted settings”, follow the instructions in Lightning Launcher before enabling the service.</p><button className="text-button" disabled={inventoryLoading} onClick={() => setCheck(value => value + 1)}>Check activation status</button></div>}
    <div className="modal-actions"><span className="lightning-total">{!task && `Download ${formatBytes((installLauncher ? launcher?.size ?? 0 : 0) + (installService ? service?.size ?? 0 : 0))}`}</span><button className="button secondary" onClick={onClose}>{task ? 'Close' : 'Cancel'}</button>{task && !isActiveTask(task) ? <button className="button primary" onClick={() => { setTaskId(null); setSubmittedTask(null); setCheck(value => value + 1); }}>Back to setup</button> : !task && <button className="button primary" disabled={!canSubmit} onClick={() => void submit()}>{submitting ? <LoaderCircle size={16} className="spin" /> : <Download size={16} />}{submitting ? 'Queueing…' : 'Download & install'}</button>}</div>
  </div>;
}
