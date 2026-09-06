import { invoke } from '@tauri-apps/api/core';
import { GlobalConfig } from './_types';

export function loadConfig(): Promise<GlobalConfig> {
  return invoke<GlobalConfig>('plugin:configuration|load');
}

export function saveConfig(configuration: GlobalConfig): Promise<void> {
  return invoke('plugin:configuration|save', { configuration });
}
