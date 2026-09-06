import { invoke } from '@tauri-apps/api/core';

export function getDeviceLocale(): Promise<string> {
  return invoke<string>('plugin:system|get_device_locale');
}

export function getDeviceName(): Promise<string> {
  return invoke<string>('plugin:system|get_device_name');
}
