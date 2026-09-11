import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from './api';
import type { Device } from './types';

export function useDeviceDiscovery(refreshToken: number, fail: (error: unknown) => void) {
  const [devices, setDevices] = useState<Device[]>([]);
  const [loadingDevices, setLoadingDevices] = useState(true);
  const polling = useRef(false);
  const mounted = useRef(false);
  const loadDevices = useCallback(async (quiet = false) => {
    if (polling.current) return;
    polling.current = true;
    if (!quiet) setLoadingDevices(true);
    try { const result = await api.devices(); if (mounted.current) setDevices(result); }
    catch (error) { if (mounted.current) fail(error); }
    finally { polling.current = false; if (mounted.current) setLoadingDevices(false); }
  }, [fail]);
  useEffect(() => {
    mounted.current = true;
    void loadDevices();
    const timer = setInterval(() => { void loadDevices(true); }, 15000);
    return () => { mounted.current = false; clearInterval(timer); };
  }, [refreshToken, loadDevices]);
  return { devices, loadingDevices };
}
