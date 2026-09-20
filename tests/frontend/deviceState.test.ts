import assert from 'node:assert/strict';
import { test } from 'node:test';
import { applyDeviceProfiles, connectionSummary, createDeviceDiscovery, decideDeviceArrival, includeDeviceProfiles, mergeDeviceSnapshots, selectDevice, selectKnownDevice, selectTransport, reconnectHint, unregisteredDevices } from '../../src/deviceState.ts';
import type { Device, DevicePreferences } from '../../src/types.ts';

const ready: Device = { id: 'DEMO-HEADSET', model: 'Quest 3', transports: [
  { serial: 'DEMO-GUID._adb-tls-connect._tcp', kind: 'wifi', state: 'device' },
] };
const offline: Device = { id: '192.0.2.10:37001', model: 'Quest 3', transports: [
  { serial: '192.0.2.10:37001', kind: 'wifi', state: 'offline' },
] };
const tick = () => new Promise<void>(resolve => setImmediate(resolve));
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

test('an offline address sorted before a ready TLS transport does not hide the available headset', () => {
  assert.equal(selectDevice([offline, ready], ''), ready);
  assert.equal(selectDevice([offline, ready], 'DEMO-REMOVED-USB'), ready);
  assert.equal(selectDevice([offline], ''), offline);
  assert.equal(selectDevice([], ''), undefined);
});

test('explicit device selection is preserved even when only another headset is ready', () => {
  assert.equal(selectDevice([offline, ready], offline.id), offline);
  const mixed = { ...ready, transports: [...offline.transports, ...ready.transports] };
  assert.equal(selectDevice([offline, mixed], ''), mixed);
  assert.equal(mixed.transports[0].state, 'offline');
});

test('USB is preferred over Wi-Fi for a device with two ready transports', () => {
  const mixed = { ...ready, transports: [
    { serial: 'DEMO-WIFI-001', kind: 'wifi' as const, state: 'device' },
    { serial: 'DEMO-USB-001', kind: 'usb' as const, state: 'device' },
  ] };
  assert.equal(selectTransport(mixed)?.kind, 'usb');
  assert.equal(selectTransport({ ...mixed, transports: [mixed.transports[0]] })?.kind, 'wifi');
});

test('saved profiles merge names and preserve offline devices', () => {
  const preferences: DevicePreferences = {
    profiles: [
      { id: 'DEMO-HEADSET', displayName: 'Living room', model: 'Quest 3', connectionPreference: 'wifi' },
      { id: 'DEMO-OFFLINE', displayName: 'Office', model: 'Quest 2', connectionPreference: 'auto' },
    ],
    autoSwitch: false,
  };
  const merged = includeDeviceProfiles([{ ...ready, transports: [
    { serial: 'DEMO-USB-001', kind: 'usb', state: 'device' },
    { serial: 'DEMO-WIFI-001', kind: 'wifi', state: 'device' },
  ] }], preferences);
  assert.equal(merged[0].displayName, 'Living room');
  assert.equal(selectTransport(merged[0])?.kind, 'wifi');
  assert.deepEqual(merged.find(device => device.id === 'DEMO-OFFLINE')?.transports, []);
  assert.equal(selectKnownDevice(merged, '', new Set(preferences.profiles.map(profile => profile.id)))?.id, 'DEMO-HEADSET');
});

test('connection preferences restrict the active transport', () => {
  const mixed = { ...ready, transports: [
    { serial: 'DEMO-WIFI-001', kind: 'wifi' as const, state: 'device' },
    { serial: 'DEMO-USB-001', kind: 'usb' as const, state: 'device' },
  ] };
  const profiles: DevicePreferences = {
    profiles: [
      { id: mixed.id, displayName: 'USB only', model: mixed.model, connectionPreference: 'usb' },
    ],
    autoSwitch: false,
  };
  const usbOnly = applyDeviceProfiles([mixed], profiles)[0];
  assert.equal(selectTransport(usbOnly)?.kind, 'usb');
  const wifiOnly = applyDeviceProfiles([mixed], { ...profiles, profiles: [{ ...profiles.profiles[0], connectionPreference: 'wifi' }] })[0];
  assert.equal(selectTransport(wifiOnly)?.kind, 'wifi');
  const noWifi = { ...wifiOnly, transports: [mixed.transports[1]] };
  assert.equal(selectTransport(noWifi), undefined);
});

test('a saved USB headset is selected on discovery without visiting Devices', () => {
  const knownIds = new Set([ready.id, 'DEMO-OFFLINE']);
  const usb: Device = { ...ready, transports: [{ serial: 'DEMO-USB', kind: 'usb', state: 'device' }] };
  const savedOffline: Device = { id: 'DEMO-OFFLINE', model: 'Quest 2', transports: [] };
  assert.equal(selectKnownDevice([savedOffline, usb], '', knownIds), usb);
  assert.equal(selectKnownDevice([savedOffline, usb], 'DEMO-REMOVED', knownIds), usb);
  assert.equal(selectKnownDevice([savedOffline, usb], savedOffline.id, knownIds), savedOffline);
  assert.equal(selectKnownDevice([usb], usb.id, new Set()), undefined);
  assert.equal(selectKnownDevice([usb], '', new Set()), undefined);
  assert.deepEqual(unregisteredDevices([usb], knownIds), []);
});

test('unregistered ready and unauthorized headsets prompt for registration, offline history does not', () => {
  const unauthorized: Device = { id: 'DEMO-NEW', model: 'Quest 3S', transports: [{ serial: 'DEMO-NEW', kind: 'usb', state: 'unauthorized' }] };
  assert.deepEqual(unregisteredDevices([offline, ready, unauthorized], new Set()), [ready, unauthorized]);
  assert.deepEqual(unregisteredDevices([offline, ready, unauthorized], new Set([ready.id])), [unauthorized]);
});

test('device arrivals use the first completed snapshot as a baseline', () => {
  const other: Device = { id: 'DEMO-OTHER', model: 'Quest 3S', transports: [{ serial: 'DEMO-USB-OTHER', kind: 'usb', state: 'device' }] };
  const knownIds = new Set([ready.id, other.id]);
  const initial = decideDeviceArrival([ready, other], null, knownIds, undefined, true, false);
  assert.equal(initial.kind, 'baseline');
  assert.deepEqual([...initial.readyIds].sort(), [other.id, ready.id].sort());
  const next = decideDeviceArrival([ready, other], initial.readyIds, knownIds, ready.id, true, false);
  assert.equal(next.kind, 'none');
});

test('saved device arrivals switch, notify, or wait according to preferences and task state', () => {
  const other: Device = { id: 'DEMO-OTHER', model: 'Quest 3S', transports: [{ serial: 'DEMO-USB-OTHER', kind: 'usb', state: 'device' }] };
  const knownIds = new Set([ready.id, other.id]);
  const previous = new Set([ready.id]);
  assert.equal(decideDeviceArrival([ready, other], previous, knownIds, ready.id, true, false).kind, 'autoSwitch');
  assert.equal(decideDeviceArrival([ready, other], previous, knownIds, ready.id, true, true).kind, 'defer');
  assert.equal(decideDeviceArrival([ready, other], previous, knownIds, ready.id, false, false).kind, 'notify');
  assert.equal(decideDeviceArrival([ready, other], previous, new Set([ready.id]), ready.id, true, false).kind, 'none');
});

test('a single arrival decision retains every saved device that arrived together', () => {
  const first: Device = { id: 'DEMO-FIRST', model: 'Quest 3', transports: [{ serial: 'DEMO-FIRST-USB', kind: 'usb', state: 'device' }] };
  const second: Device = { id: 'DEMO-SECOND', model: 'Quest 3S', transports: [{ serial: 'DEMO-SECOND-USB', kind: 'usb', state: 'device' }] };
  const decision = decideDeviceArrival([first, second], new Set(), new Set([first.id, second.id]), undefined, false, false);
  assert.equal(decision.kind, 'notify');
  assert.deepEqual(decision.devices.map(device => device.id), [first.id, second.id]);
});

test('a saved device that disappears can trigger an arrival again', () => {
  const knownIds = new Set([ready.id]);
  const gone = decideDeviceArrival([], new Set([ready.id]), knownIds, undefined, true, false);
  assert.equal(gone.kind, 'none');
  const returned = decideDeviceArrival([ready], gone.readyIds, knownIds, undefined, true, false);
  assert.equal(returned.kind, 'autoSwitch');
});

test('duplicate Wi-Fi transports share one ready status without removing command targets', () => {
  const mixed: Device = { ...ready, transports: [
    { serial: 'DEMO-USB', kind: 'usb', state: 'device' },
    { serial: '192.0.2.10:37001', kind: 'wifi', state: 'offline' },
    ...ready.transports,
    { serial: '192.0.2.10:37002', kind: 'wifi', state: 'device' },
  ] };
  const summary = connectionSummary(mixed);
  assert.deepEqual(summary.map(item => [item.kind, item.state]), [['usb', 'device'], ['wifi', 'device']]);
  assert.equal(mixed.transports.length, 4);
  assert.equal(selectTransport(mixed)?.serial, 'DEMO-USB');
  const wifiOnly = { ...mixed, connectionPreference: 'wifi' as const };
  assert.equal(connectionSummary(wifiOnly)[1].serial, selectTransport(wifiOnly)?.serial);
  const disconnected = mergeDeviceSnapshots([mixed], [])[0];
  assert.deepEqual(connectionSummary(disconnected).map(item => item.state), ['offline', 'offline']);
});

test('a disconnected device remains in the session directory as offline', () => {
  const first = { ...ready, transports: [
    { serial: 'DEMO-USB-001', kind: 'usb' as const, state: 'device' },
    { serial: 'DEMO-WIFI-001', kind: 'wifi' as const, state: 'device' },
  ] };
  const merged = mergeDeviceSnapshots([first], []);
  assert.equal(merged.length, 1);
  assert.ok(merged[0].transports.every(transport => transport.state === 'offline'));
});

test('identity re-resolution rebinds a transport without retaining a fallback duplicate', () => {
  const fallback: Device = { id: 'DEMO-USB-001', model: 'Quest 3', transports: [
    { serial: 'DEMO-USB-001', kind: 'usb', state: 'device' },
  ] };
  const resolved: Device = { id: 'DEMO-PHYSICAL-001', model: 'Quest 3', transports: [
    { serial: 'DEMO-USB-001', kind: 'usb', state: 'device' },
  ] };
  const merged = mergeDeviceSnapshots([fallback], [resolved]);
  assert.deepEqual(merged, [resolved]);
});

test('manual refresh suppresses an older offline poll and waits for a new snapshot', async () => {
  const old = deferred<Device[]>();
  const fresh = deferred<Device[]>();
  const snapshots: Device[][] = [];
  let reads = 0;
  const discovery = createDeviceDiscovery(() => ++reads === 1 ? old.promise : fresh.promise, value => snapshots.push(value));
  const poll = discovery.poll();
  await tick();
  const refresh = discovery.refresh();
  assert.equal(discovery.refresh(), refresh);
  assert.equal(discovery.poll(), refresh);
  old.resolve([offline]);
  await poll;
  await tick();
  assert.equal(reads, 2);
  assert.deepEqual(snapshots, []);
  fresh.resolve([ready]);
  assert.deepEqual(await refresh, [ready]);
  assert.deepEqual(snapshots, [[ready]]);
});

test('a fresh offline result replaces a previously connected snapshot without faking a connection', async () => {
  let reads = 0;
  const snapshots: Device[][] = [];
  const discovery = createDeviceDiscovery(async () => ++reads === 1 ? [ready] : [offline], value => snapshots.push(value));
  await discovery.poll();
  await discovery.refresh();
  assert.deepEqual(snapshots, [[ready], [offline]]);
});

test('refresh recovers a failed pending read and polling resumes after completion', async () => {
  const old = deferred<Device[]>();
  let reads = 0;
  const discovery = createDeviceDiscovery(() => ++reads === 1 ? old.promise : Promise.resolve([ready]), () => {});
  const poll = discovery.poll();
  const failure = assert.rejects(poll, /disconnected/);
  const refresh = discovery.refresh();
  old.reject(new Error('disconnected'));
  await failure;
  assert.deepEqual(await refresh, [ready]);
  await discovery.poll();
  assert.equal(reads, 3);
});

test('disposing during refresh prevents late publication and a queued device query', async () => {
  const old = deferred<Device[]>();
  let reads = 0;
  const snapshots: Device[][] = [];
  const discovery = createDeviceDiscovery(() => { reads++; return old.promise; }, value => snapshots.push(value));
  const poll = discovery.poll();
  await tick();
  const refresh = discovery.refresh();
  const stopped = assert.rejects(refresh, /stopped/);
  discovery.dispose();
  old.resolve([ready]);
  await poll;
  await stopped;
  assert.equal(reads, 1);
  assert.deepEqual(snapshots, []);
});

test('reconnection resolves the known service at its current port and does not reuse stale numeric addresses', () => {
  assert.deepEqual(reconnectHint(ready), { service: 'DEMO-GUID' });
  assert.deepEqual(reconnectHint(offline), {});
  assert.deepEqual(reconnectHint({ ...ready, transports: [] }), {});
  assert.deepEqual(reconnectHint({ ...ready, transports: [...offline.transports, ...ready.transports] }), { service: 'DEMO-GUID' });
});
