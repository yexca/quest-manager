import type { Device, Transport } from './types.ts';

/** Select the active transport for a physical device. USB is the stable default. */
export function selectTransport(device: Device | undefined): Transport | undefined {
  return device?.transports.find(transport => transport.state === 'device' && transport.kind === 'usb')
    ?? device?.transports.find(transport => transport.state === 'device');
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
