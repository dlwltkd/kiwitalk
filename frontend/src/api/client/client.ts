import { invoke } from '@tauri-apps/api/core';

import { ChannelListItem, ClientStatus, KiwiTalkEvent } from '../_types';

export function created(): Promise<boolean> {
  return invoke('plugin:client|created');
}

export function create(status: ClientStatus): Promise<number> {
  return invoke('plugin:client|create', { status });
}

export function reconnect(status: ClientStatus): Promise<number> {
  return invoke('plugin:client|reconnect', { status });
}

export function destroy(): Promise<void> {
  return invoke('plugin:client|destroy');
}

export function nextEvent(): Promise<KiwiTalkEvent | null> {
  return invoke('plugin:client|next_event');
}

export function getChannelList(): Promise<[string, ChannelListItem][]> {
  return invoke('plugin:client|channel_list');
}

export async function* createMainEventStream(): AsyncGenerator<KiwiTalkEvent> {
  let event: KiwiTalkEvent | null;

  while ((event = await nextEvent())) {
    yield event;
  }
}
