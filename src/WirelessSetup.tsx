import { useRef, useState, type FormEvent } from 'react';
import { CheckCircle2, Hash, LoaderCircle, Moon, QrCode, Usb } from 'lucide-react';
import { api, isDesktop, isPreview } from './api';
import type { Device, WirelessRequest, WirelessResult } from './types';
import { QrPairing } from './QrPairing';

type Method = WirelessRequest['method'] | 'qr';
export function WirelessSetup({ devices, activeTasks, onConnected, onBusy, onClose }: {
  devices: Device[]; activeTasks: boolean; onConnected: (serial: string) => Promise<void>;
  onBusy: (busy: boolean) => void; onClose: () => void;
}) {
  const [method, setMethod] = useState<Method>('usb');
  const [pairAddress, setPairAddress] = useState('');
  const [code, setCode] = useState('');
  const [usb, setUsb] = useState('');
  const [busy, setBusy] = useState(false);
  const pending = useRef(false);
  const [result, setResult] = useState<WirelessResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const usbDevices = devices.flatMap(device => device.transports.filter(t => t.kind === 'usb' && t.state === 'device').map(t => ({ serial: t.serial, model: device.model })));
  // Never silently substitute another headset when the chosen USB cable disappears.
  const usbReady = usbDevices.some(t => t.serial === usb);
  const existingWifi = devices.find(device => device.transports.some(t => t.serial === usb))?.transports.some(t => t.kind === 'wifi' && t.state === 'device');
  const blocked = busy || !isDesktop || activeTasks;
  const switchMethod = (next: Method) => { setMethod(next); setError(null); setResult(null); setCode(''); };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (blocked || pending.current || method === 'qr' || (method === 'usb' && !usbReady)) return;
    const request: WirelessRequest = method === 'pair' ? { method, address: pairAddress.trim(), code: code.trim() } : { method, device: usb };
    pending.current = true; setBusy(true); onBusy(true); setError(null); setResult(null); setCode('');
    try {
      const response = await api.wirelessConnection(request);
      if (response.serial) await onConnected(response.serial);
      setResult(response);
    } catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { pending.current = false; setBusy(false); onBusy(false); }
  };
  return <div className="wireless-setup">
    <p className="wireless-sleep-note"><Moon size={17} /><span>Wi-Fi disconnects during deep sleep and reconnects when the headset wakes on the same network. Keep wireless debugging enabled.</span></p>
    <p className="modal-description">Wireless pairing cannot start while the headset is in deep sleep. Put it on or press the power button to wake it, and keep the display on until setup finishes. Connect both devices to the same local network and enable developer mode.</p>
    <div className="wireless-methods" role="tablist" aria-label="Wireless setup method" onKeyDown={event => {
      if (busy || !['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
      event.preventDefault();
      const methods: Method[] = ['usb', 'qr', 'pair'];
      const index = event.key === 'Home' ? 0 : event.key === 'End' ? 2 : (methods.indexOf(method) + (event.key === 'ArrowRight' ? 1 : 2)) % 3;
      switchMethod(methods[index]);
      event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]')[index]?.focus();
    }}>
      {([{ id: 'usb', label: 'USB setup', icon: Usb }, { id: 'qr', label: 'QR code', icon: QrCode }, { id: 'pair', label: 'Pairing code', icon: Hash }] as const).map(({ id, label, icon: Icon }) =>
        <button key={id} id={`wireless-tab-${id}`} role="tab" type="button" aria-selected={method === id} aria-controls="wireless-panel" tabIndex={method === id ? 0 : -1} disabled={busy} onClick={() => switchMethod(id)}><Icon size={17} /><span>{label}</span></button>)}
    </div>
    <div id="wireless-panel" role="tabpanel" aria-labelledby={`wireless-tab-${method}`}>
    {method === 'qr' ? <QrPairing activeTasks={activeTasks} onBusy={value => { setBusy(value); onBusy(value); }} onConnected={onConnected} onClose={onClose} /> : <form onSubmit={event => void submit(event)}>
      <fieldset disabled={busy}>
        {method === 'pair' && <>
          <h3>Pair with a six-digit code</h3>
          <p className="wireless-guidance">In the headset's Wireless debugging settings, choose Pair device with pairing code. Keep that screen open. If your headset does not offer it, use USB setup.</p>
          <label className="field-label" htmlFor="pair-address">IP address and pairing port</label>
          <input id="pair-address" className="text-input" value={pairAddress} onChange={e => setPairAddress(e.target.value)} placeholder="192.0.2.10:37123" autoComplete="off" spellCheck={false} maxLength={80} required />
          <label className="field-label" htmlFor="pair-code">Pairing code</label>
          <input id="pair-code" className="text-input pairing-code" value={code} onChange={e => setCode(e.target.value)} placeholder="6 digits" inputMode="numeric" autoComplete="off" pattern="[0-9]{6}" maxLength={6} required />
          <p className="wireless-hint">Pairing and connection follow automatically. Use the pairing port shown beside the six-digit code.</p>
        </>}
        {method === 'usb' && <>
          <h3>Enable Wi-Fi using a USB cable</h3>
          <p className="wireless-guidance">Connect a USB data cable and accept the debugging prompt in the headset. We check for an existing connection and pairing first, then pair only if needed. No QR scanner or pairing code is needed.</p>
          <label className="field-label" htmlFor="wireless-usb">Authorized USB headset</label>
          <select id="wireless-usb" className="text-input" value={usb} onChange={e => { setUsb(e.target.value); setError(null); setResult(null); }} required>
            <option value="">Choose a USB headset</option>
            {usb && !usbReady && <option value={usb}>Selected USB connection unavailable · {usb}</option>}
            {usbDevices.map(t => <option key={t.serial} value={t.serial}>{t.model} · {t.serial}</option>)}
          </select>
          {!usbDevices.length && <p className="wireless-hint">No authorized USB headset found. Connect and authorize it; this list refreshes automatically.</p>}
          {usbReady && existingWifi && <p className="wireless-hint" role="status">This headset is already connected over Wi-Fi. Its existing authorization will be used.</p>}
          <p className="wireless-hint">Use a trusted local network. Unplug only after connection succeeds. You may need to repeat setup after restarting the headset.</p>
        </>}
      </fieldset>
      {result && <div className="wireless-success" role="status"><CheckCircle2 size={18} /><p>{result.message}{result.serial && <strong>{result.serial}</strong>}</p></div>}
      {error && <p className="dialog-error" role="alert">{error}</p>}
      {!isDesktop && <p className="wireless-hint">{isPreview ? 'Preview only. Connections and pairing are disabled.' : 'Open the desktop app to connect to a headset.'}</p>}
      {isDesktop && activeTasks && <p className="wireless-hint">Wait for queued and running tasks to finish before setting up a connection.</p>}
      <div className="modal-actions"><button type="button" className="button secondary" disabled={busy} onClick={onClose}>{result?.serial ? 'Done' : 'Close'}</button><button className="button primary" disabled={blocked || (method === 'usb' && !usbReady)}>{busy && <LoaderCircle size={16} className="spin" />}{busy ? 'Connecting…' : method === 'pair' ? 'Pair and connect' : 'Connect over Wi-Fi'}</button></div>
    </form>}
    </div>
  </div>;
}
