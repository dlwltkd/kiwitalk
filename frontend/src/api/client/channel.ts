import { invoke } from '@tauri-apps/api/core';

export type Chatlog = {
  /** bigint */
  logId: string;
  /** bigint */
  prevLogId?: string;

  senderId: string;
  sendAt: number;

  chatType: number;

  content?: string;
  attachment?: string;
  supplement?: string;

  referer?: number;
}

export type ChannelMeta = {
  type: number,
  revision: string,
  authorId: string,
  updated_at: number,
  content: string,
};

export type ArchiveEntry = {
  date: string;
  time: string | null;
  sender: string | null;
  senderId: string | null;
  content: string;
  sendAt: number | null;
};

export type ChatArchive = {
  sourceName: string;
  savedAt: string;
  utcOffsetMinutes: number;
  messageCount: number;
  matchedCount: number;
  entries: ArchiveEntry[];
};

export function loadChatArchive(id: string): Promise<ChatArchive | null> {
  return invoke('plugin:client|channel_load_archive', { id });
}

export function importChatArchive(id: string, sourceName: string, text: string, utcOffsetMinutes: number): Promise<ChatArchive> {
  return invoke('plugin:client|channel_import_archive', { id, sourceName, text, utcOffsetMinutes });
}

export type ChannelUser = {
  nickname: string;

  profileUrl: string;
  fullProfileUrl: string;
  originalProfileUrl: string;

  /** bigint */
  watermark: string;
}

export type NormalChannelUser = {
  countryIso: string;
  statusMessage: string;
  accountId: string;
  linkedServices: string;
  suspended: boolean;
} & ChannelUser;
export type OpenChannelUser = {
  userType: number;
  accountId: string;
  countryIso?: string;
  serviceUserType?: number;
  suspended: boolean;
  suspicion: string;
  openMemberType: number;
  profileType: number;
  profileLinkId: string;
  openToken: number;
} & ChannelUser;

type NormalChannelKind = {
  kind: 'normal',
  content: {
    users: [string, NormalChannelUser][],
  }
}
type OpenChannelKind = {
  kind: 'open',
  content: {
    users: [string, OpenChannelUser][],
    metas: ChannelMeta[],
    openToken: number,
  }
}

export type Channel = NormalChannelKind | OpenChannelKind;

export async function loadChannel(id: string): Promise<Channel> {
  return invoke('plugin:client|load_channel', { id });
}

export async function setChannelActive(id: string, active: boolean): Promise<void> {
  await invoke('plugin:client|channel_set_active', { id, active });
}

export async function sendText(id: string, text: string): Promise<Chatlog> {
  return await invoke('plugin:client|channel_send_text', { id, text });
}

export async function normalChannelReadChat(id: string, logId: string) {
  await invoke('plugin:client|normal_channel_read_chat', { id, logId });
}

export type HistorySyncStopReason =
  | 'upToDate'
  | 'serverComplete'
  | 'reachedTarget'
  | 'emptyBatch'
  | 'noProgress'
  | 'historyGap'
  | 'pageLimit'
  | 'timeLimit'
  | 'unsupportedChannel';

export type HistorySyncResult = {
  fetchedCount: number;
  pageCount: number;
  cachedCount: number;
  gapCount: number;
  complete: boolean;
  stopReason: HistorySyncStopReason;
};

export function syncChannelHistory(id: string): Promise<HistorySyncResult> {
  return invoke('plugin:client|channel_sync_history', { id });
}

export async function loadChat(
  id: string,
  count: number,
  fromLogId?: string,
  exclusive: boolean = false,
): Promise<Chatlog[]> {
  return await invoke(
    'plugin:client|channel_load_chat',
    { id, count, exclusive, fromLogId },
  );
}
