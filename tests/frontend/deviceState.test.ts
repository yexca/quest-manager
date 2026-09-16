import assert from 'node:assert/strict';
import { test } from 'node:test';
import { applyDeviceProfiles, createDeviceDiscovery, includeDeviceProfiles, mergeDeviceSnapshots, selectDevice, selectKnownDevice, selectTransport } from '../../src/deviceState.ts';
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

test('a disconnected device remains in the session directory as offline', () => {
  const first = { ...ready, transports: [
    { serial: 'DEMO-USB-001', kind: 'usb' as const, state: 'device' },
    { serial: 'DEMO-WIFI-001', kind: 'wifi' as const, state: 'device' },
  ] };
  const merged = mergeDeviceSnapshots([first], []);
  assert.equal(merged.length, 1);
  assert.ok(merged[0].transports.every(transport => transport.state === 'offline'));
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
