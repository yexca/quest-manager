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
  if (deviceId) return devices.find(device => device.id === deviceId);
  const known = devices.filter(device => knownIds.has(device.id));
  return known.find(device => selectTransport(device)) ?? known[0];
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
  const merged = next.map(device => {
    const prior = previous.find(item => item.id === device.id);
    if (!prior) return device;
    const transports = new Map(device.transports.map(transport => [transport.serial, transport]));
    for (const transport of prior.transports) {
      if (!transports.has(transport.serial)) transports.set(transport.serial, { ...transport, state: 'offline' });
    }
    return { ...device, transports: [...transports.values()] };
  });
  for (const device of previous) {
    if (!incoming.has(device.id)) merged.push({ ...device, transports: device.transports.map(transport => ({ ...transport, state: 'offline' })) });
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
