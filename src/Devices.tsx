import { CheckCircle2, CircleHelp, Pencil, Plus, RefreshCw, Settings2, Smartphone, Usb, Wifi, Zap } from 'lucide-react';
import { isDesktop } from './api';
import { selectTransport } from './deviceState';
import type { ConnectionPreference, Device, DevicePowerSettings, DeviceProfile } from './types';

function transportLabel(kind: string) { return kind === 'usb' ? 'USB' : 'Wi-Fi'; }

export function Devices({
  devices, selectedId, onSelect, onAddConnection, onRefresh, loading, power, powerLoading,
  onStayAwake, onLightning, autoSwitch, onAutoSwitch, onSaveProfile,
  onRename,
}: {
  devices: Device[];
  selectedId: string;
  onSelect: (id: string) => void;
  onAddConnection: () => void;
  onRefresh: () => void;
  loading: boolean;
  power: DevicePowerSettings | null;
  powerLoading: boolean;
  onStayAwake: (enabled: boolean) => void;
  onLightning: () => void;
  autoSwitch: boolean;
  onAutoSwitch: (enabled: boolean) => void;
  onSaveProfile: (profile: DeviceProfile) => Promise<void>;
  onRename: (device: Device) => void;
}) {
  const selected = devices.find(device => device.id === selectedId) ?? devices[0];
  const current = selectTransport(selected);
  const ready = Boolean(current);
  return <div className="devices-page">
    <div className="devices-toolbar">
      <div><strong>{devices.length} {devices.length === 1 ? 'device' : 'devices'}</strong><span>Known device names and connection preferences are saved locally.</span></div>
      <div className="devices-toolbar-actions"><button className="button secondary small" onClick={onRefresh} disabled={loading}><RefreshCw size={15} className={loading ? 'spin' : ''} />Refresh</button><button className="button primary small" onClick={onAddConnection}><Plus size={15} />Add connection</button></div>
    </div>
    {!devices.length ? <section className="card devices-empty"><Smartphone size={30} /><h2>No devices found</h2><p>Connect a Quest with a USB data cable or add a Wi-Fi connection.</p><button className="button primary" onClick={onAddConnection}><Plus size={16} />Add connection</button></section> : <div className="devices-layout">
      <section className="card device-list-panel"><div className="devices-panel-heading"><h2>Your devices</h2><span>{devices.length}</span></div>{devices.map(device => {
        const connected = Boolean(selectTransport(device));
        const active = device.id === selected?.id;
        return <button key={device.id} className={`device-list-item ${active ? 'selected' : ''}`} onClick={() => onSelect(device.id)}>
          <span className={`device-list-icon ${connected ? 'connected' : ''}`}><Smartphone size={19} /></span>
          <span className="device-list-copy"><strong>{device.displayName || device.model}</strong><small>{device.model} · {device.id}</small><span className="device-transport-summary">{device.transports.map(transport => <span key={transport.serial} className={transport.state === 'device' ? 'ready' : ''}>{transport.kind === 'usb' ? <Usb size={12} /> : <Wifi size={12} />}{transportLabel(transport.kind)} · {transport.state === 'device' ? 'Connected' : transport.state === 'unauthorized' ? 'Authorization needed' : 'Offline'}</span>)}</span></span>
          {connected && <CheckCircle2 size={17} className="device-list-check" />}
        </button>;
      })}</section>
      <section className="device-settings-column">
        <section className="card device-connection-card"><div className="devices-panel-heading"><div><p className="eyebrow">SELECTED DEVICE</p><h2>{selected?.displayName ?? selected?.model ?? 'Device'}</h2></div><Settings2 size={21} /></div>{selected && <><p className="device-id mono">{selected.model} · {selected.id}</p><div className="transport-list">{selected.transports.map(transport => <div key={transport.serial} className={`transport-row ${transport.state === 'device' ? 'ready' : ''}`}><span className="transport-icon">{transport.kind === 'usb' ? <Usb size={17} /> : <Wifi size={17} />}</span><span><strong>{transportLabel(transport.kind)}</strong><small className="mono">{transport.serial}</small></span><span className={`transport-state ${transport.state === 'device' ? 'ready' : ''}`}>{transport.state === 'device' ? 'Connected' : transport.state === 'unauthorized' ? 'Authorization needed' : 'Offline'}</span></div>)}</div><p className="connection-priority"><CheckCircle2 size={15} />Current connection: {current?.kind === 'usb' ? 'USB' : current?.kind === 'wifi' ? 'Wi-Fi' : 'Disconnected'}. Automatic mode prefers USB.</p><div className="device-profile-actions"><button className="button secondary small" onClick={() => onRename(selected)}><Pencil size={14} />Rename</button><label className="device-preference"><span>Preferred connection</span><select value={selected.connectionPreference || 'auto'} onChange={event => void onSaveProfile({ id: selected.id, model: selected.model, displayName: selected.displayName || selected.model, connectionPreference: event.target.value as ConnectionPreference })}><option value="auto">Automatic · USB first</option><option value="usb">USB only</option><option value="wifi">Wi-Fi only</option></select></label></div></>}</section>
        <section className="card device-setting-card"><div className="setting-heading"><span className="setting-icon"><Settings2 size={18} /></span><div><h2>Automatic device switching</h2><p>When another known headset appears, switch only when there is no active task.</p></div></div><label className="device-toggle"><span><strong>{autoSwitch ? 'Enabled' : 'Disabled'}</strong><small>When disabled, Quest Manager keeps the selected device and asks before switching.</small></span><input type="checkbox" checked={autoSwitch} onChange={event => onAutoSwitch(event.target.checked)} /><span className="toggle-track" aria-hidden="true"><span /></span></label></section>
        <section className="card device-setting-card"><div className="setting-heading"><span className="setting-icon"><Smartphone size={18} /></span><div><h2>Keep awake while charging</h2><p>Keep the headset awake while it is charging to improve long Wi-Fi transfers.</p></div></div>{!ready ? <p className="setting-unavailable"><CircleHelp size={15} />Connect and authorize this device to read its setting.</p> : <label className="device-toggle"><span><strong>{powerLoading ? 'Reading setting…' : power?.stayAwake === null ? 'Setting unavailable' : power?.stayAwake ? 'Enabled' : 'Disabled'}</strong><small>Uses the headset's global charging sleep setting.</small></span><input type="checkbox" checked={power?.stayAwake === true} disabled={!isDesktop || powerLoading || power?.stayAwake === null} onChange={event => onStayAwake(event.target.checked)} /><span className="toggle-track" aria-hidden="true"><span /></span></label>}</section>
        <section className="card device-setting-card"><div className="setting-heading"><span className="setting-icon lightning"><Zap size={18} /></span><div><h2>Lightning Launcher</h2><p>Install or update Lightning Launcher and its optional Meta button service.</p></div></div><button className="button secondary small" disabled={!ready} onClick={onLightning}><Zap size={15} />Open setup</button></section>
      </section>
    </div>}
  </div>;
}
