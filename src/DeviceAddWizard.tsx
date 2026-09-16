import { useEffect, useMemo, useState } from 'react';
import { CheckCircle2, Hash, LoaderCircle, QrCode, RefreshCw, Usb } from 'lucide-react';
import { api, isDesktop } from './api';
import type { ConnectionPreference, Device, DeviceProfile, WirelessRequest, WirelessResult } from './types';
import { QrPairing } from './QrPairing';

type Method = WirelessRequest['method'] | 'qr';
type Stage = 'choose' | 'wireless-offer' | 'done';

function errorText(cause: unknown) { return cause instanceof Error ? cause.message : String(cause); }

export function DeviceAddWizard({ devices, activeTasks, onRefresh, onSave, onConnected, onBusy, onClose }: {
  devices: Device[];
  activeTasks: boolean;
  onRefresh: () => Promise<Device[]>;
  onSave: (profile: DeviceProfile) => Promise<void>;
  onConnected: (serial: string) => Promise<void>;
  onBusy: (busy: boolean) => void;
  onClose: () => void;
}) {
  const [method, setMethod] = useState<Method>('usb');
  const [stage, setStage] = useState<Stage>('choose');
  const [usb, setUsb] = useState('');
  const [pairAddress, setPairAddress] = useState('');
  const [code, setCode] = useState('');
  const [name, setName] = useState('');
  const [preference, setPreference] = useState<ConnectionPreference>('auto');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<WirelessResult | null>(null);
  const [candidate, setCandidate] = useState<Device | null>(null);
  const [wirelessSerial, setWirelessSerial] = useState<string | null>(null);
  const usbDevices = useMemo(() => devices.flatMap(device => device.transports.filter(t => t.kind === 'usb').map(transport => ({ device, transport }))), [devices]);
  const selectedUsb = usbDevices.find(item => item.transport.serial === usb);
  const readyUsb = selectedUsb?.transport.state === 'device';
  const methods: { id: Method; label: string; icon: typeof Usb }[] = [{ id: 'usb', label: 'USB', icon: Usb }, { id: 'qr', label: 'QR code', icon: QrCode }, { id: 'pair', label: 'Pairing code', icon: Hash }];

  useEffect(() => {
    if (method === 'usb' && !usb && usbDevices.length) setUsb(usbDevices[0].transport.serial);
  }, [method, usb, usbDevices]);
  useEffect(() => {
    if (method === 'usb' && selectedUsb && !name) setName(selectedUsb.device.displayName || selectedUsb.device.model);
  }, [method, selectedUsb, name]);
  useEffect(() => {
    if (method !== 'usb' || stage !== 'choose') return;
    const timer = window.setInterval(() => { void onRefresh().catch(() => {}); }, 2500);
    return () => window.clearInterval(timer);
  }, [method, stage, onRefresh]);

  const profileFor = (device: Device): DeviceProfile => ({
    id: device.id, model: device.model, displayName: name.trim() || device.displayName || device.model, connectionPreference: preference,
  });
  const complete = async (serial: string, sourceDevice?: Device, offerWireless = false) => {
    let device = sourceDevice ?? devices.find(item => item.transports.some(transport => transport.serial === serial)) ?? candidate;
    if (!device) {
      const latest = await onRefresh();
      device = latest.find(item => item.transports.some(transport => transport.serial === serial)) ?? null;
    }
    if (!device) throw new Error('The paired device could not be identified. Refresh and try again.');
    const profile = profileFor(device);
    setBusy(true); onBusy(true); setError(null);
    try {
      await onSave(profile);
      await onConnected(serial);
      setCandidate(device); setWirelessSerial(serial); setStage(offerWireless ? 'wireless-offer' : 'done');
    } catch (cause) { setError(errorText(cause)); }
    finally { setBusy(false); onBusy(false); }
  };
  const runPair = async (request: WirelessRequest) => {
    if (busy || activeTasks) return;
    setBusy(true); onBusy(true); setError(null); setResult(null);
    try {
      const response = await api.wirelessConnection(request);
      setResult(response);
      if (response.serial) await complete(response.serial);
    } catch (cause) { setError(errorText(cause)); }
    finally { setBusy(false); onBusy(false); }
  };
  const submitUsb = async () => {
    if (!selectedUsb) { setError('Connect a headset with a USB data cable first.'); return; }
    if (!readyUsb) { setError('Accept USB debugging in the headset, then choose Next again.'); return; }
    setCandidate(selectedUsb.device);
    await complete(selectedUsb.transport.serial, selectedUsb.device, true);
  };
  const enableWireless = async () => {
    if (!wirelessSerial || !candidate || busy) return;
    setBusy(true); onBusy(true); setError(null);
    try {
      const response = await api.wirelessConnection({ method: 'usb', device: wirelessSerial });
      setResult(response);
      if (response.serial) await onConnected(response.serial);
      setStage('done');
    } catch (cause) { setError(errorText(cause)); }
    finally { setBusy(false); onBusy(false); }
  };
  if (stage === 'wireless-offer') return <div className="device-add-wizard">
    <div className="wireless-success" role="status"><CheckCircle2 size={19} /><p><strong>{candidate?.displayName || candidate?.model || 'Device'} added.</strong> USB is ready and the profile is saved.</p></div>
    <h3>Try wireless connection too?</h3>
    <p className="modal-description">Quest Manager can run the existing USB-assisted wireless setup now. This keeps both USB and Wi-Fi available; USB remains the preferred transport.</p>
    {result && <p className="wireless-hint" role="status">{result.message}</p>}
    {error && <p className="dialog-error" role="alert">{error}</p>}
    <div className="modal-actions"><button className="button secondary" disabled={busy} onClick={() => { setStage('done'); }}>{busy ? 'Working…' : 'No, finish with USB'}</button><button className="button primary" disabled={busy || !isDesktop || activeTasks} onClick={() => void enableWireless()}>{busy && <LoaderCircle size={16} className="spin" />}Yes, enable Wi-Fi</button></div>
  </div>;
  if (stage === 'done') return <div className="device-add-wizard"><div className="wireless-success" role="status"><CheckCircle2 size={19} /><p>Device added. You can reconnect it automatically whenever its USB or Wi-Fi transport appears.</p></div><div className="modal-actions"><button className="button primary" onClick={onClose}>Done</button></div></div>;
  return <div className="device-add-wizard">
    <p className="modal-description">Add a headset to this computer. A device profile stores its name and connection preference; pairing credentials remain managed by ADB.</p>
    <div className="wireless-methods" role="tablist" aria-label="Add device method">{methods.map(({ id, label, icon: Icon }) => <button key={id} role="tab" type="button" aria-selected={method === id} disabled={busy} onClick={() => { setMethod(id); setError(null); setResult(null); }}>{<Icon size={17} />}<span>{label}</span></button>)}</div>
    {method === 'usb' && <section role="tabpanel"><h3>Add through USB</h3><p className="wireless-guidance">Connect the headset with a USB data cable. If the status is unauthorized, put on the headset and allow USB debugging, then refresh.</p><label className="field-label" htmlFor="add-usb">USB connection</label><select id="add-usb" className="text-input" value={usb} onChange={event => { setUsb(event.target.value); setError(null); }}><option value="">Choose a USB connection</option>{usbDevices.map(({ device, transport }) => <option key={transport.serial} value={transport.serial}>{device.model} · {transport.state === 'device' ? 'Authorized' : 'Authorization needed'} · {transport.serial}</option>)}</select>{!usbDevices.length && <p className="wireless-hint"><RefreshCw size={14} />Waiting for a USB headset…</p>}{selectedUsb?.transport.state === 'unauthorized' && <p className="wireless-hint" role="alert">USB debugging authorization is required. Accept the prompt in the headset, then choose Next.</p>}</section>}
    {method === 'pair' && <section role="tabpanel"><h3>Add with a pairing code</h3><p className="wireless-guidance">Open Wireless debugging → Pair device with pairing code in the headset and keep it visible.</p><label className="field-label" htmlFor="add-address">IP address and pairing port</label><input id="add-address" className="text-input" value={pairAddress} onChange={event => setPairAddress(event.target.value)} placeholder="192.0.2.10:37123" required /><label className="field-label" htmlFor="add-code">Pairing code</label><input id="add-code" className="text-input pairing-code" value={code} onChange={event => setCode(event.target.value)} inputMode="numeric" pattern="[0-9]{6}" maxLength={6} placeholder="6 digits" required /></section>}
    {method === 'qr' && <section role="tabpanel"><h3>Add with a QR code</h3><p className="wireless-guidance">Scan the generated code from the headset's Wireless debugging settings.</p><QrPairing activeTasks={activeTasks} onBusy={value => { setBusy(value); onBusy(value); }} onConnected={serial => complete(serial)} onClose={onClose} /></section>}
    <label className="field-label" htmlFor="device-name">Device name</label><input id="device-name" className="text-input" value={name} onChange={event => setName(event.target.value)} placeholder={candidate?.model || 'Quest 3'} maxLength={80} required /><p className="wireless-hint">Names must be unique. The model is used as the initial suggestion.</p>
    <label className="field-label" htmlFor="connection-preference">Preferred connection</label><select id="connection-preference" className="text-input" value={preference} onChange={event => setPreference(event.target.value as ConnectionPreference)}><option value="auto">Automatic (USB first, Wi-Fi fallback)</option><option value="usb">USB only</option><option value="wifi">Wi-Fi only</option></select>
    {result && <p className="wireless-success" role="status"><CheckCircle2 size={18} /><span>{result.message}</span></p>}{error && <p className="dialog-error" role="alert">{error}</p>}
    <div className="modal-actions"><button className="button secondary" disabled={busy} onClick={onClose}>Cancel</button>{method === 'qr' ? null : <button className="button primary" disabled={busy || !isDesktop || activeTasks || (method === 'usb' && !selectedUsb) || (method === 'pair' && (!pairAddress.trim() || code.length !== 6))} onClick={() => void (method === 'usb' ? submitUsb() : runPair({ method: 'pair', address: pairAddress.trim(), code: code.trim() }))}>{busy && <LoaderCircle size={16} className="spin" />}{method === 'usb' ? 'Next' : 'Pair and add'}</button>}</div>
  </div>;
}
