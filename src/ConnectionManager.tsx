import { useRef, useState } from 'react';
import { LoaderCircle, RefreshCw, Unplug, Usb, Wifi } from 'lucide-react';
import { api, isDesktop } from './api';
import { WirelessSetup } from './WirelessSetup';
import { reconnectHint } from './deviceState';
import type { Device, ReconnectWirelessResult } from './types';

const message = (cause: unknown) => cause instanceof Error ? cause.message : String(cause);

export function ConnectionManager({ device, activeSerial, activeTasks, onRefresh, onUseWifi, onConnected, onBusy, onClose }: {
  device: Device;
  activeSerial: string;
  activeTasks: boolean;
  onRefresh: () => Promise<Device[]>;
  onUseWifi: (serial: string) => Promise<void>;
  onConnected: (serial: string) => Promise<void>;
  onBusy: (busy: boolean) => void;
  onClose: () => void;
}) {
  const [setup, setSetup] = useState(false);
  const [setupUsb, setSetupUsb] = useState<string>();
  const [busy, setBusy] = useState(false);
  const pending = useRef(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [disconnect, setDisconnect] = useState<string | null>(null);
  const [reconnection, setReconnection] = useState<ReconnectWirelessResult | null>(null);
  const [address, setAddress] = useState('');
  const readyWifi = device.transports.find(item => item.kind === 'wifi' && item.state === 'device');
  const blocked = busy || activeTasks || !isDesktop;

  const run = async (operation: () => Promise<void>) => {
    if (pending.current) return;
    pending.current = true; setBusy(true); onBusy(true); setError(''); setNotice('');
    try { await operation(); }
    catch (cause) { setError(message(cause)); }
    finally { pending.current = false; setBusy(false); onBusy(false); }
  };
  const reconnect = (manualAddress?: string) => {
    if (blocked) return;
    void run(async () => {
      setReconnection(null);
      try {
        const result = await api.reconnectWireless({ physicalId: device.id, ...(manualAddress ? { address: manualAddress } : reconnectHint(device)) });
        setReconnection(result);
        if (result.status === 'connected' && result.serial) {
          await onConnected(result.serial);
          setAddress('');
        }
      } finally { await onRefresh(); }
    });
  };
  const useExisting = () => {
    if (blocked) return;
    void run(async () => {
      const latest = (await onRefresh()).find(item => item.id === device.id);
      const wifi = latest?.transports.find(item => item.kind === 'wifi' && item.state === 'device');
      if (!wifi) {
        setNotice('Wi-Fi is no longer connected. Choose Reconnect to try existing authorization.');
        return;
      }
      await onUseWifi(wifi.serial);
      setNotice('Using the existing Wi-Fi connection. No new connection or pairing was created.');
    });
  };
  const openPairing = () => {
    if (blocked) return;
    setSetupUsb(device.transports.find(item => item.kind === 'usb' && item.state === 'device')?.serial);
    setSetup(true);
  };
  const confirmDisconnect = () => {
    if (blocked || !disconnect) return;
    const captured = disconnect;
    void run(async () => {
      try {
        await api.disconnectWireless({ device: captured, physicalId: device.id });
        setDisconnect(null);
        setReconnection(null);
        setNotice('Connection disconnected. Pairing is retained; ADB may reconnect automatically.');
      } finally { await onRefresh(); }
    });
  };

  if (setup) return <WirelessSetup devices={[device]} initialUsb={setupUsb} activeTasks={activeTasks}
    onConnected={async serial => { await onConnected(serial); setSetup(false); setReconnection(null); setNotice('Wi-Fi is ready. Your preferred connection is unchanged.'); }}
    onBusy={value => { setBusy(value); onBusy(value); }} onClose={() => setSetup(false)} />;

  return <div className="connection-manager">
    <p className="modal-description">USB and Wi-Fi can connect the same headset. Each address below is an actual connection; only the one marked In use is selected for new operations.</p>
    <div className="transport-list">
      {device.transports.map(transport => <div key={transport.serial} className={`transport-row ${transport.state === 'device' ? 'ready' : ''}`}>
        <span className="transport-icon">{transport.kind === 'usb' ? <Usb size={17} /> : <Wifi size={17} />}</span>
        <span><strong>{transport.kind === 'usb' ? 'USB' : 'Wi-Fi'}{transport.serial === activeSerial && ' · In use'}</strong><small className="mono">{transport.serial}</small></span>
        <span className={`transport-state ${transport.state === 'device' ? 'ready' : ''}`}>{transport.state === 'device' ? 'Connected' : transport.state === 'unauthorized' ? 'Authorization needed' : 'Offline'}</span>
        {transport.kind === 'wifi' && transport.state === 'device' && <button className="button secondary small" disabled={blocked} aria-label={`Disconnect ${transport.serial}`} onClick={() => { setDisconnect(transport.serial); setError(''); setNotice(''); }}><Unplug size={14} />Disconnect</button>}
      </div>)}
      {!device.transports.length && <p className="wireless-hint">No active connections detected. Choose Connect to discover wireless debugging services.</p>}
    </div>
    <p className="wireless-hint">Unplug the cable to disconnect USB. Offline entries are connection history for this session.</p>
    {readyWifi && <p className="wireless-hint">Wi-Fi is already available. Use it directly without pairing again. This sets the device's preferred connection to Wi-Fi only.</p>}
    {!readyWifi && <p className="wireless-hint">Pairing status: unverified. Connect reuses existing authorization and does not pair again. Keep wireless debugging enabled and the headset awake on the same network.</p>}
    {reconnection && <section className="connection-reconnect" aria-label="Wi-Fi connection result">
      <p className={reconnection.status === 'connected' ? 'wireless-success' : 'wireless-hint'} role="status">{reconnection.message}</p>
      {reconnection.status !== 'connected' && <>
        {reconnection.endpoints.length > 0 && <><label className="field-label" htmlFor="discovered-wifi">Discovered wireless debugging services</label><select className="text-input" id="discovered-wifi" disabled={busy} value={address} onChange={event => setAddress(event.target.value)}><option value="">Choose the headset's connection address</option>{reconnection.endpoints.map(endpoint => <option key={`${endpoint.service}:${endpoint.address}`} value={endpoint.address}>{endpoint.service} · {endpoint.address}</option>)}</select></>}
        <label className="field-label" htmlFor="reconnect-address">Connection IP address and port</label><input id="reconnect-address" className="text-input" disabled={busy} value={address} onChange={event => setAddress(event.target.value)} placeholder="192.0.2.10:37001" />
        <p className="wireless-hint">Use the address on the headset's Wireless debugging page, not the pairing-code port. Saved device identity is checked before selecting the connection.</p>
        <button className="button secondary small" disabled={blocked || !address.trim()} onClick={() => reconnect(address.trim())}>Connect to this address</button>
        {reconnection.status === 'pairingRequired' && <button className="button secondary small" disabled={blocked} onClick={openPairing}>Pair device</button>}
      </>}
    </section>}
    <details className="connection-pairing"><summary>First-time setup / pairing</summary><p className="wireless-hint">Use pairing for a new device or after authorization is revoked. Connection failures alone do not mean pairing was lost. USB setup can also enable wireless debugging when needed.</p><button className="button secondary small" disabled={blocked} onClick={openPairing}>Open pairing setup</button></details>
    {disconnect && <section className="connection-disconnect-confirm" role="group" aria-label="Confirm Wi-Fi disconnection">
      <strong>Disconnect this Wi-Fi connection?</strong><p className="mono">{disconnect}</p>
      <p>This can interrupt other ADB tools using this connection. Pairing is kept. ADB may reconnect automatically.</p>
      <div className="modal-actions"><button className="button secondary small" disabled={busy} onClick={() => setDisconnect(null)}>Cancel</button><button className="button secondary small" disabled={blocked} onClick={confirmDisconnect}>Disconnect connection</button></div>
    </section>}
    {activeTasks && <p className="wireless-hint">Wait for queued tasks, running tasks or wireless setup to finish before changing connections.</p>}
    {!isDesktop && <p className="wireless-hint">Preview only. Connection changes are disabled.</p>}
    {error && <p className="dialog-error" role="alert">{error}</p>}
    {notice && <p className="wireless-hint" role="status">{notice}</p>}
    <div className="modal-actions"><button className="button secondary" disabled={busy} onClick={onClose}>Close</button><button className="button secondary" disabled={busy} onClick={() => void run(async () => { await onRefresh(); setReconnection(null); })}><RefreshCw size={15} />Refresh</button><button className="button primary" disabled={blocked} onClick={() => readyWifi ? useExisting() : reconnect()}>{busy ? <LoaderCircle size={15} className="spin" /> : <Wifi size={15} />}{busy ? 'Working…' : readyWifi ? 'Use existing Wi-Fi' : device.transports.some(item => item.kind === 'wifi') ? 'Reconnect' : 'Connect'}</button></div>
  </div>;
}
