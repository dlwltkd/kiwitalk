import {
  Accessor,
  createEffect,
  createMemo,
  createSignal,
  on,
  onCleanup,
} from 'solid-js';

import {
  Chatlog,
  HistorySyncResult,
  loadChat,
  syncChannelHistory,
} from '@/api/client';
import { useChannelEvent, useReady } from '@/pages/main/_hooks';

const groupMessages = (messages: Chatlog[]) => messages.reduce<Chatlog[][]>((groups, message) => {
  const group = groups.at(-1);
  if (group?.at(-1)?.senderId === message.senderId) group.push(message);
  else groups.push([message]);

  return groups;
}, []);

const syncWarning = (result: HistorySyncResult) => {
  if (result.complete) return null;

  if (result.stopReason === 'pageLimit' || result.stopReason === 'timeLimit') {
    return 'history sync paused at its safety limit; reopen this chat to continue';
  }
  if (result.stopReason === 'unsupportedChannel') {
    return 'history sync is not available for this room type yet';
  }

  return 'remote history synchronization stalled; showing cached messages';
};

export const useMessageList = (channelId: Accessor<string | null>) => {
  const isReady = useReady();
  const event = useChannelEvent();
  const [messages, setMessages] = createSignal<Chatlog[]>([]);
  const [isEnd, setIsEnd] = createSignal(false);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [loadVersion, setLoadVersion] = createSignal(0);
  let activeChannelId: string | null = null;
  let generation = 0;
  let initialHistorySyncPending = false;
  let reloadLatestPending = false;

  const messageGroups = createMemo(() => groupMessages(messages()));

  const mergeMessages = (current: Chatlog[], incoming: Chatlog[], position: 'start' | 'end') => {
    const known = new Set(current.map((message) => message.logId));
    const unique = incoming.filter((message) => !known.has(message.logId));

    return position === 'start' ? [...unique, ...current] : [...current, ...unique];
  };

  const mergeLatestMessages = (current: Chatlog[], incoming: Chatlog[]) => {
    const byLogId = new Map(current.map((message) => [message.logId, message]));
    for (const message of incoming) byLogId.set(message.logId, message);

    return [...byLogId.values()].sort((left, right) => {
      const leftId = BigInt(left.logId);
      const rightId = BigInt(right.logId);

      return leftId === rightId ? 0 : leftId > rightId ? -1 : 1;
    });
  };

  const loadMore = () => {
    if (!isReady() || !activeChannelId || loading() || isEnd()) return;
    setLoadVersion((version) => version + 1);
  };

  createEffect(() => {
    const id = channelId();
    if (id === activeChannelId) return;

    activeChannelId = id;
    generation += 1;
    initialHistorySyncPending = Boolean(id);
    reloadLatestPending = false;
    setMessages([]);
    setIsEnd(false);
    setLoading(false);
    setError(null);

    if (id && isReady()) queueMicrotask(loadMore);
  });

  createEffect(on(isReady, (ready) => {
    generation += 1;
    setLoading(false);
    if (!ready || !activeChannelId) return;

    initialHistorySyncPending = true;
    reloadLatestPending = true;
    setIsEnd(false);
    queueMicrotask(loadMore);
  }, { defer: true }));

  createEffect(on(loadVersion, async () => {
    const id = activeChannelId;
    if (!id || loading() || isEnd()) return;

    const requestGeneration = ++generation;
    const shouldReloadLatest = reloadLatestPending;
    const oldestLogId = shouldReloadLatest ? undefined : messages().at(-1)?.logId;
    const shouldSyncHistory = initialHistorySyncPending;
    setLoading(true);
    setError(null);

    try {
      let syncFailed = false;
      let syncNotice: string | null = null;
      if (shouldSyncHistory) {
        try {
          syncNotice = syncWarning(await syncChannelHistory(id));
        } catch {
          syncFailed = true;
        }

        if (requestGeneration !== generation || id !== activeChannelId) return;
        initialHistorySyncPending = false;
      }

      const loaded = await loadChat(id, 200, oldestLogId, true);
      if (requestGeneration !== generation || id !== activeChannelId) return;

      if (shouldReloadLatest) {
        setMessages((current) => mergeLatestMessages(current, loaded));
        reloadLatestPending = false;
      } else if (loaded.length === 0) {
        setIsEnd(true);
      } else {
        setMessages((current) => mergeMessages(current, loaded, 'end'));
      }

      if (syncFailed) setError('remote history could not be synchronized; showing local transcript');
      else if (syncNotice) setError(syncNotice);
    } catch {
      if (requestGeneration === generation) setError('local transcript could not be loaded');
    } finally {
      if (requestGeneration === generation) setLoading(false);
    }
  }, { defer: true }));

  createEffect(on(event, (incoming) => {
    if (!incoming || incoming.channelId !== activeChannelId) return;

    if (incoming.type === 'Chat') {
      setMessages((current) => mergeMessages(current, [incoming.content], 'start'));
    } else if (incoming.type === 'ChatDeleted') {
      setMessages((current) => current.filter((message) => message.logId !== incoming.content.logId));
    }
  }));

  onCleanup(() => {
    generation += 1;
    activeChannelId = null;
    initialHistorySyncPending = false;
    reloadLatestPending = false;
  });

  return {
    messageGroups,
    loadMore,
    isEnd,
    loading,
    error,
  };
};
