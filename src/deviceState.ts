import type { ConnectionPreference, Device, DevicePreferences, Transport } from './types.ts';

export function applyDeviceProfiles(devices: Device[], preferences: DevicePreferences): Device[] {
  const profiles = new Map(preferences.profiles.map(profile => [profile.id, profile]));
  return devices.map(device => {
    const profile = profiles.get(device.id);
    return profile
      ? { ...device, displayName: profile.displayName, connectionPreference: profile.connectionPreference }
      : { ...device, connectionPreference: device.connectionPreference ?? 'auto' };
  });
}

/** Keep saved devices visible between sessions without inventing a transport. */
export function includeDeviceProfiles(devices: Device[], preferences: DevicePreferences): Device[] {
  const applied = applyDeviceProfiles(devices, preferences);
  const seen = new Set(applied.map(device => device.id));
  for (const profile of preferences.profiles) {
    if (seen.has(profile.id)) continue;
    applied.push({
      id: profile.id,
      model: profile.model,
      displayName: profile.displayName,
      connectionPreference: profile.connectionPreference,
      transports: [],
    });
  }
  return applied;
}

/** Select a ready device from the saved profiles, leaving unregistered candidates alone. */
export function selectKnownDevice(devices: Device[], deviceId: string, knownIds: ReadonlySet<string>) {
  const known = devices.filter(device => knownIds.has(device.id));
  const selected = known.find(device => device.id === deviceId);
  if (selected) return selected;
  return known.find(device => selectTransport(device)) ?? known[0];
}

/** A detected transport is a candidate, not an implicitly saved device. */
export function unregisteredDevices(devices: Device[], knownIds: ReadonlySet<string>): Device[] {
  return devices.filter(device => !knownIds.has(device.id)
    && device.transports.some(transport => transport.state === 'device' || transport.state === 'unauthorized'));
}

export type DeviceArrivalAction =
  | { kind: 'baseline'; readyIds: Set<string>; devices: Device[] }
  | { kind: 'none'; readyIds: Set<string>; devices: Device[] }
  | { kind: 'autoSwitch'; readyIds: Set<string>; devices: Device[]; device: Device }
  | { kind: 'notify'; readyIds: Set<string>; devices: Device[]; device: Device }
  | { kind: 'defer'; readyIds: Set<string>; devices: Device[]; device: Device };

/**
 * Decide what a newly ready saved device should do. The first snapshot is only
 * a baseline, so devices already connected when the app starts never look new.
 * Unknown devices are deliberately excluded; their registration prompt is
 * driven by `unregisteredDevices` and must never be selected implicitly.
 */
export function decideDeviceArrival(
  devices: Device[],
  previousReadyIds: ReadonlySet<string> | null,
  knownIds: ReadonlySet<string>,
  selectedId: string | undefined,
  autoSwitch: boolean,
  activeTasks: boolean,
): DeviceArrivalAction {
  const readyIds = new Set(devices.filter(device => Boolean(selectTransport(device))).map(device => device.id));
  if (previousReadyIds === null) return { kind: 'baseline', readyIds, devices: [] };
  const newcomers = devices.filter(device => knownIds.has(device.id)
    && readyIds.has(device.id)
    && !previousReadyIds.has(device.id)
    && device.id !== selectedId);
  if (!newcomers.length) return { kind: 'none', readyIds, devices: [] };
  const device = newcomers[0];
  if (autoSwitch && !activeTasks) return { kind: 'autoSwitch', readyIds, devices: newcomers, device };
  if (autoSwitch) return { kind: 'defer', readyIds, devices: newcomers, device };
  return { kind: 'notify', readyIds, devices: newcomers, device };
}

/** One status per connection method; retain every actual transport for IPC/tasks. */
export function connectionSummary(device: Device): Transport[] {
  const current = selectTransport(device);
  return (['usb', 'wifi'] as const).flatMap(kind => {
    const candidates = device.transports.filter(transport => transport.kind === kind);
    const representative = candidates.find(transport => transport.serial === current?.serial)
      ?? candidates.find(transport => transport.state === 'device')
      ?? candidates.find(transport => transport.state === 'unauthorized')
      ?? candidates[0];
    return representative ? [representative] : [];
  });
}

/** Resolve a previously observed service at its current port; never guess from an old address. */
export function reconnectHint(device: Device): { service?: string; address?: string } {
  const wifi = device.transports.filter(transport => transport.kind === 'wifi');
  const service = wifi.find(transport => /\._adb-tls-connect\._tcp\.?$/.test(transport.serial));
  if (service) return { service: service.serial.replace(/\._adb-tls-connect\._tcp\.?$/, '') };
  return {};
}

/** Select the active transport for a physical device. USB is the stable default. */
export function selectTransport(device: Device | undefined): Transport | undefined {
  const ready = device?.transports.filter(transport => transport.state === 'device') ?? [];
  const preference: ConnectionPreference = device?.connectionPreference ?? 'auto';
  if (preference === 'usb') return ready.find(transport => transport.kind === 'usb');
  if (preference === 'wifi') return ready.find(transport => transport.kind === 'wifi');
  return ready.find(transport => transport.kind === 'usb') ?? ready.find(transport => transport.kind === 'wifi');
}

/** Keep devices seen during this session so a disconnect does not retarget work. */
export function mergeDeviceSnapshots(previous: Device[], next: Device[]): Device[] {
  const incoming = new Map(next.map(device => [device.id, device]));
  const consumed = new Set<Device>();
  const merged = next.map(device => {
    // ADB may expose a transport serial before `ro.serialno` is readable, then
    // group the same transport under its physical identity on the next poll.
    // Rebind that alias instead of leaving a stale offline duplicate behind.
    const prior = previous.find(item => !consumed.has(item) && (item.id === device.id
      || item.transports.some(oldTransport => device.transports.some(transport => transport.serial === oldTransport.serial))));
    if (!prior) return device;
    consumed.add(prior);
    const transports = new Map(device.transports.map(transport => [transport.serial, transport]));
    for (const transport of prior.transports) {
      if (!transports.has(transport.serial)) transports.set(transport.serial, { ...transport, state: 'offline' });
    }
    return { ...device, transports: [...transports.values()] };
  });
  for (const device of previous) {
    if (!consumed.has(device) && !incoming.has(device.id)) merged.push({ ...device, transports: device.transports.map(transport => ({ ...transport, state: 'offline' })) });
  }
  return merged;
}

export function selectDevice(devices: Device[], deviceId: string) {
  return devices.find(device => device.id === deviceId)
    ?? devices.find(device => device.transports.some(transport => transport.state === 'device'))
    ?? devices[0];
}

// Serialize discovery, but make an explicit refresh supersede an older poll.
export function createDeviceDiscovery(read: () => Promise<Device[]>, publish: (devices: Device[]) => void) {
  let pending: Promise<Device[]> | null = null;
  let refreshing: Promise<Device[]> | null = null;
  let revision = 0;
  let disposed = false;

  function start() {
    if (disposed) return Promise.reject(new Error('Device discovery has stopped.'));
    const capturedRevision = revision;
    const request = Promise.resolve().then(read).then(devices => {
      if (!disposed && capturedRevision === revision) publish(devices);
      return devices;
    }).finally(() => { if (pending === request) pending = null; });
    pending = request;
    return request;
  }

  return {
    poll(): Promise<Device[]> { return refreshing ?? pending ?? start(); },
    refresh(): Promise<Device[]> {
      if (refreshing) return refreshing;
      revision += 1;
      const previous = pending;
      const request = (async () => {
        try { await previous; } catch { /* A fresh snapshot can recover a failed poll. */ }
        return start();
      })().finally(() => { if (refreshing === request) refreshing = null; });
      refreshing = request;
      return request;
    },
    dispose() { disposed = true; revision += 1; },
  };
}
