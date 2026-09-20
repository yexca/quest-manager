import { CheckCircle2, CircleHelp, Pencil, Plus, RefreshCw, Settings2, Smartphone, Trash2, Usb, Wifi, Zap } from 'lucide-react';
import { isDesktop } from './api';
import { connectionSummary, selectTransport } from './deviceState';
import type { ConnectionPreference, Device, DevicePowerSettings, DeviceProfile } from './types';

function transportLabel(kind: string) { return kind === 'usb' ? 'USB' : 'Wi-Fi'; }

export function Devices({
  devices, selectedId, onSelect, onAddConnection, onRefresh, loading, power, powerLoading, powerError,
  onStayAwake, onLightning, autoSwitch, onAutoSwitch, onSaveProfile,
  onRename, onManageConnections,
  onRemove,
}: {
  devices: Device[];
  selectedId: string;
  onSelect: (id: string) => void;
  onAddConnection: () => void;
  onRefresh: () => void;
  loading: boolean;
  power: DevicePowerSettings | null;
  powerLoading: boolean;
  powerError: string | null;
  onStayAwake: (enabled: boolean) => void;
  onLightning: () => void;
  autoSwitch: boolean;
  onAutoSwitch: (enabled: boolean) => void;
  onSaveProfile: (profile: DeviceProfile) => Promise<void>;
  onRename: (device: Device) => void;
  onManageConnections: (device: Device) => void;
  onRemove: (device: Device) => void;
}) {
  const selected = devices.find(device => device.id === selectedId);
  const current = selectTransport(selected);
  const ready = Boolean(current);
  return <div className="devices-page">
    <div className="devices-toolbar">
      <div><strong>{devices.length} {devices.length === 1 ? 'device' : 'devices'}</strong><span>Known device names and connection preferences are saved locally.</span></div>
      <div className="devices-toolbar-actions"><button className="button secondary small" onClick={onRefresh} disabled={loading}><RefreshCw size={15} className={loading ? 'spin' : ''} />Refresh</button><button className="button primary small" onClick={onAddConnection}><Plus size={15} />Add connection</button></div>
    </div>
    {!devices.length ? <section className="card devices-empty"><Smartphone size={30} /><h2>No saved devices</h2><p>Connect a Quest with a USB data cable or add a Wi-Fi connection.</p><button className="button primary" onClick={onAddConnection}><Plus size={16} />Add connection</button></section> : <div className="devices-layout">
      <section className="card device-list-panel">
        <div className="devices-panel-heading"><h2>Your devices</h2><span>{devices.length}</span></div>
        <div className="device-rows" role="group" aria-label="Your devices">
          {devices.map(device => {
            const connection = selectTransport(device);
            const active = device.id === selected?.id;
            return <div key={device.id} className={`device-list-item ${active ? 'selected' : ''}`}>
              <button className="device-select" aria-pressed={active} onClick={() => onSelect(device.id)}>
                <span className={`device-list-icon ${connection ? 'connected' : ''}`}><Smartphone size={19} /></span>
                <span className="device-list-copy"><strong>{device.displayName || device.model}{active && <span className="device-selected-label">Selected</span>}</strong><small>{device.model} · {device.id}</small>
                  <span className="device-transport-summary">{device.transports.length ? connectionSummary(device).map(transport => <span key={transport.serial} title={transport.serial} className={transport.state === 'device' ? 'ready' : ''}>{transport.kind === 'usb' ? <Usb size={12} /> : <Wifi size={12} />}{transportLabel(transport.kind)} · {transport.state === 'device' ? 'Connected' : transport.state === 'unauthorized' ? 'Authorization needed' : 'Offline'}{active && connection?.serial === transport.serial ? ' · In use' : ''}</span>) : <span>Offline</span>}</span>
                </span>
              </button>
              <div className="device-row-actions"><button className="button secondary small" aria-label={`Rename ${device.displayName || device.model}`} onClick={() => onRename(device)}><Pencil size={14} />Edit name</button><button className="button secondary small" aria-label={`Manage connections for ${device.displayName || device.model}`} onClick={() => onManageConnections(device)}><Settings2 size={14} />Manage connections</button><button className="button secondary small" aria-label={`Remove saved device ${device.displayName || device.model}`} onClick={() => onRemove(device)}><Trash2 size={14} />Remove saved device</button></div>
            </div>;
          })}
        </div>
        {selected && <div className="device-list-footer">
          <label className="device-preference"><span>Preferred connection</span><select aria-label="Preferred connection" value={selected.connectionPreference || 'auto'} onChange={event => void onSaveProfile({ id: selected.id, model: selected.model, displayName: selected.displayName || selected.model, connectionPreference: event.target.value as ConnectionPreference })}><option value="auto">Automatic · USB first</option><option value="usb">USB only</option><option value="wifi">Wi-Fi only</option></select></label>
          {devices.length > 1 && <label className="device-toggle device-auto-switch"><span><strong>Automatic device switching</strong><small>Switch to a known headset when no task is active.</small></span><input type="checkbox" checked={autoSwitch} onChange={event => onAutoSwitch(event.target.checked)} /><span className="toggle-track" aria-hidden="true"><span /></span></label>}
          <p className="connection-priority"><CheckCircle2 size={15} />Current connection: {current ? transportLabel(current.kind) : 'Disconnected'}</p>
        </div>}
      </section>
      <section className="device-settings-column">
        <section className="card device-setting-card"><div className="setting-heading"><span className="setting-icon"><Smartphone size={18} /></span><div><h2>Keep awake while charging</h2><p>Keep the headset awake while it is charging to improve long Wi-Fi transfers.</p></div></div>{!ready ? <p className="setting-unavailable"><CircleHelp size={15} />Connect and authorize this device to read its setting.</p> : <><label className="device-toggle"><span><strong>{powerLoading ? 'Reading setting…' : power?.stayAwake == null ? 'Setting unavailable' : power?.stayAwake ? 'Enabled' : 'Disabled'}</strong><small>Uses the headset's global charging sleep setting.</small></span><input type="checkbox" checked={power?.stayAwake === true} disabled={!isDesktop || powerLoading || power?.stayAwake == null} onChange={event => onStayAwake(event.target.checked)} /><span className="toggle-track" aria-hidden="true"><span /></span></label>{!powerLoading && power?.stayAwake == null && <div className="setting-unavailable" role="status"><CircleHelp size={15} /><span>{powerError || 'The headset did not return a readable charging sleep setting.'}</span><button className="button secondary small" onClick={onRefresh}><RefreshCw size={14} />Retry</button></div>}</>}</section>
        <section className="card device-setting-card"><div className="setting-heading"><span className="setting-icon lightning"><Zap size={18} /></span><div><h2>Lightning Launcher</h2><p>Install or update Lightning Launcher and its optional Meta button service.</p></div></div><button className="button secondary small" disabled={!ready} onClick={onLightning}><Zap size={15} />Open setup</button></section>
      </section>
    </div>}
  </div>;
}
