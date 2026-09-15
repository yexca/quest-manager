import type { Device } from './types.ts';

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
