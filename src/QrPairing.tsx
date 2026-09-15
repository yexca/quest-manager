import { useEffect, useRef, useState } from 'react';
import { CheckCircle2, LoaderCircle, QrCode } from 'lucide-react';
import { api, isDesktop, isPreview } from './api';
import type { WirelessQrSnapshot } from './types';

const active = (status: WirelessQrSnapshot['status']) => ['waiting', 'pairing', 'connecting'].includes(status);
const message = (cause: unknown) => cause instanceof Error ? cause.message : String(cause);

export function QrPairing({ activeTasks, onBusy, onConnected, onClose }: {
  activeTasks: boolean; onBusy: (busy: boolean) => void;
  onConnected: (serial: string) => Promise<void>; onClose: () => void;
}) {
  const [session, setSession] = useState<WirelessQrSnapshot | null>(null);
  const [busy, setBusy] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [now, setNow] = useState(Date.now());
  const pending = useRef(false);
  const mounted = useRef(true);
  const sessionId = useRef<string | null>(null);
  const qrDisplay = useRef<HTMLDivElement>(null);
  const callbacks = useRef({ onBusy, onConnected });
  callbacks.current = { onBusy, onConnected };
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      if (sessionId.current) void api.cancelWirelessQr(sessionId.current).catch(() => {});
    };
  }, []);
  useEffect(() => {
    if (!busy) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [busy]);
  useEffect(() => {
    if (session?.id) qrDisplay.current?.scrollIntoView({ block: 'center' });
  }, [session?.id]);

  const generate = async () => {
    if (pending.current || !isDesktop || activeTasks) return;
    pending.current = true; setBusy(true); callbacks.current.onBusy(true);
    setSession(null); setError(null); setCancelling(false); setNow(Date.now());
    try {
      let next = await api.startWirelessQr();
      if (!mounted.current) { await api.cancelWirelessQr(next.id); return; }
      sessionId.current = next.id;
      setSession(next); setNow(Date.now());
      while (active(next.status)) {
        await new Promise(resolve => window.setTimeout(resolve, 500));
        if (!mounted.current) return;
        next = await api.wirelessQrStatus(next.id);
        if (!mounted.current) return;
        setSession(next);
      }
      sessionId.current = null;
      if (next.status === 'connected' && next.serial) await callbacks.current.onConnected(next.serial);
    } catch (cause) {
      const interrupted = sessionId.current !== null;
      if (sessionId.current) await api.cancelWirelessQr(sessionId.current).catch(() => {});
      sessionId.current = null;
      if (mounted.current) {
        setSession(previous => previous ? { ...previous, qrDataUrl: null, ...(interrupted ? { status: 'failed', message: 'Could not monitor QR setup. Cancellation was requested; the session also expires automatically.' } as const : {}) } : null);
        setError(message(cause));
      }
    } finally {
      pending.current = false;
      if (mounted.current) { setBusy(false); setCancelling(false); callbacks.current.onBusy(false); }
    }
  };
  const cancel = async () => {
    if (!sessionId.current || cancelling) return;
    setCancelling(true); setSession(previous => previous ? { ...previous, qrDataUrl: null } : null);
    try { await api.cancelWirelessQr(sessionId.current); }
    catch (cause) { if (mounted.current) { setError(message(cause)); setCancelling(false); } }
  };
  const seconds = session ? Math.max(0, Math.ceil((session.expiresAt - now) / 1000)) : 0;
  return <section className="qr-pairing">
    <h3>Pair with an Android debugging QR code</h3>
    <p className="wireless-guidance">In Lightning Launcher, open <strong>Android Settings → System → Developer options</strong>. In the <strong>Debugging</strong> section, open <strong>Wireless debugging → Pair device with QR code</strong>, then scan this screen.</p>
    <p className="wireless-hint qr-compatibility">Need Lightning Launcher? Use <strong>Lightning Launcher setup</strong> in Quest Manager. Settings availability varies by headset software; use USB setup if the scanner is unavailable. Pairing and connection follow automatically.</p>
    <div className="qr-display" ref={qrDisplay}>
      {session?.qrDataUrl && busy && !cancelling && seconds > 0
        ? <img className="pairing-qr" src={session.qrDataUrl} alt="ADB wireless debugging pairing QR code" draggable={false} />
        : <div className="qr-placeholder">{session?.status === 'connected' ? <CheckCircle2 size={44} /> : busy ? <LoaderCircle size={36} className="spin" /> : <QrCode size={44} />}<span>{session?.status === 'connected' ? 'Connected over Wi-Fi' : busy ? (cancelling ? 'Cancelling…' : 'Waiting for pairing status…') : 'Generate a QR code to begin'}</span></div>}
    </div>
    <p className="qr-status" role="status">{cancelling ? 'Cancelling QR setup…' : session?.message || 'Each QR code is valid for two minutes.'}</p>
    {busy && session && <p className="wireless-hint">{seconds > 0 ? `Session expires in ${seconds}s` : 'Session expired. Finishing setup…'}</p>}
    {error && <p className="dialog-error" role="alert">{error}</p>}
    {!isDesktop && <p className="wireless-hint">{isPreview ? 'Preview only. QR generation and pairing are disabled.' : 'Open the desktop app to pair a headset.'}</p>}
    {isDesktop && activeTasks && <p className="wireless-hint">Wait for queued and running tasks to finish before pairing.</p>}
    <div className="modal-actions">
      <button type="button" className="button secondary" disabled={busy} onClick={onClose}>{session?.status === 'connected' ? 'Done' : 'Close'}</button>
      {busy ? <button type="button" className="button secondary" disabled={!session || cancelling} onClick={() => void cancel()}>Cancel pairing</button>
        : <button type="button" className="button primary" disabled={!isDesktop || activeTasks} onClick={() => void generate()}><QrCode size={16} />{session ? 'Generate new QR code' : 'Generate QR code'}</button>}
    </div>
  </section>;
}
