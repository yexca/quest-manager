import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from './api';
import { createDeviceDiscovery, mergeDeviceSnapshots } from './deviceState';
import type { Device } from './types';

export function useDeviceDiscovery(refreshToken: number, fail: (error: unknown) => void) {
  const [devices, setDevices] = useState<Device[]>([]);
  const [loadingDevices, setLoadingDevices] = useState(true);
  const discovery = useRef<ReturnType<typeof createDeviceDiscovery> | null>(null);
  const knownDevices = useRef<Device[]>([]);
  const visibleRequest = useRef(0);
  const loadDevices = useCallback((quiet = false): Promise<Device[]> => {
    const current = discovery.current;
    if (!current) return Promise.resolve([]);
    const request = quiet ? null : ++visibleRequest.current;
    if (!quiet) setLoadingDevices(true);
    return current.poll().finally(() => { if (discovery.current === current && visibleRequest.current === request) setLoadingDevices(false); });
  }, []);
  const refreshDevices = useCallback(() => {
    const current = discovery.current;
    if (!current) return Promise.resolve([]);
    const request = ++visibleRequest.current;
    setLoadingDevices(true);
    return current.refresh().finally(() => { if (discovery.current === current && visibleRequest.current === request) setLoadingDevices(false); });
  }, []);
  useEffect(() => {
    knownDevices.current = [];
    const current = createDeviceDiscovery(api.devices, snapshot => {
      knownDevices.current = mergeDeviceSnapshots(knownDevices.current, snapshot);
      setDevices(knownDevices.current);
    });
    discovery.current = current;
    const report = (error: unknown) => { if (discovery.current === current) fail(error); };
    void loadDevices().catch(report);
    const timer = setInterval(() => { void loadDevices(true).catch(report); }, 3000);
    return () => { current.dispose(); discovery.current = null; clearInterval(timer); };
  }, [loadDevices, fail]);
  useEffect(() => {
    if (refreshToken === 0) return;
    const current = discovery.current;
    void refreshDevices().catch(error => { if (discovery.current === current) fail(error); });
  }, [refreshToken, refreshDevices, fail]);
  return { devices, loadingDevices, refreshDevices };
}
