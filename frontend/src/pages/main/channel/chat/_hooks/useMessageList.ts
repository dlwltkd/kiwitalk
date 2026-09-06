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

const PAGE_SIZE = 200;

const groupMessages = (messages: Chatlog[]) => messages.reduce<Chatlog[][]>((groups, message) => {
  const group = groups.at(-1);
  if (group?.at(-1)?.senderId === message.senderId) group.push(message);
  else groups.push([message]);

  return groups;
}, []);

const syncWarning = (result: HistorySyncResult) => {
  if (result.complete) return null;

  if (result.stopReason === 'pageLimit' || result.stopReason === 'timeLimit') {
    return 'More messages remain. Continue loading history.';
  }
  if (result.stopReason === 'unsupportedChannel') {
    return 'History is not available for this room type yet.';
  }

  return 'Some earlier messages were not returned. Showing available history.';
};

export const useMessageList = (channelId: Accessor<string | null>) => {
  const isReady = useReady();
  const event = useChannelEvent();
  const [messages, setMessages] = createSignal<Chatlog[]>([]);
  const [isEnd, setIsEnd] = createSignal(false);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [historyNotice, setHistoryNotice] = createSignal<string | null>(null);
  const [canSyncHistory, setCanSyncHistory] = createSignal(false);
  const [hasNextSyncPage, setHasNextSyncPage] = createSignal(false);
  const [loadVersion, setLoadVersion] = createSignal(0);
  let activeChannelId: string | null = null;
  let generation = 0;
  let initialHistorySyncPending = false;
  let reloadLatestPending = false;
  let paginationCursor: string | undefined;

  const messageGroups = createMemo(() => groupMessages(messages()));

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
    if (!isReady() || !activeChannelId || loading()) return;
    if (isEnd()) {
      if (!hasNextSyncPage()) return;
      initialHistorySyncPending = true;
      reloadLatestPending = true;
      setHasNextSyncPage(false);
      setIsEnd(false);
    }
    setLoadVersion((version) => version + 1);
  };

  const syncMore = () => {
    if (!isReady() || !activeChannelId || loading()) return;
    initialHistorySyncPending = true;
    reloadLatestPending = true;
    setIsEnd(false);
    queueMicrotask(loadMore);
  };

  createEffect(() => {
    const id = channelId();
    if (id === activeChannelId) return;

    activeChannelId = id;
    generation += 1;
    initialHistorySyncPending = Boolean(id);
    reloadLatestPending = false;
    paginationCursor = undefined;
    setMessages([]);
    setIsEnd(false);
    setLoading(false);
    setError(null);
    setHistoryNotice(null);
    setCanSyncHistory(false);
    setHasNextSyncPage(false);

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
    const oldestLogId = shouldReloadLatest ? undefined : paginationCursor;
    const shouldSyncHistory = initialHistorySyncPending;
    setLoading(true);
    setError(null);

    try {
      const loaded = await loadChat(id, PAGE_SIZE, oldestLogId, true);
      if (requestGeneration !== generation || id !== activeChannelId) return;

      setMessages((current) => mergeLatestMessages(current, loaded));
      paginationCursor = loaded.at(-1)?.logId ?? (shouldReloadLatest ? undefined : paginationCursor);
      reloadLatestPending = false;
      setIsEnd(loaded.length < PAGE_SIZE);

      let syncFailed = false;
      if (shouldSyncHistory) {
        initialHistorySyncPending = false;
        let result: HistorySyncResult | undefined;
        try {
          result = await syncChannelHistory(id);
        } catch {
          syncFailed = true;
        }

        if (requestGeneration !== generation || id !== activeChannelId) return;
        setHistoryNotice(result ? syncWarning(result) : 'History could not be loaded. You can retry.');
        setCanSyncHistory(syncFailed || Boolean(result && !result.complete && result.stopReason !== 'unsupportedChannel'));
        setHasNextSyncPage(Boolean(result && result.fetchedCount > 0 && (
          result.stopReason === 'pageLimit' || result.stopReason === 'timeLimit'
        )));

        const refreshed = await loadChat(id, PAGE_SIZE, undefined, true);
        if (requestGeneration !== generation || id !== activeChannelId) return;

        setMessages((current) => mergeLatestMessages(current, refreshed));
        paginationCursor = refreshed.at(-1)?.logId;
        setIsEnd(refreshed.length < PAGE_SIZE);
      }
    } catch {
      if (requestGeneration === generation) {
        setError('Messages could not be loaded. You can retry.');
        setCanSyncHistory(true);
      }
    } finally {
      if (requestGeneration === generation) setLoading(false);
    }
  }, { defer: true }));

  createEffect(on(event, (incoming) => {
    if (!incoming || incoming.channelId !== activeChannelId) return;

    if (incoming.type === 'Chat') {
      setMessages((current) => mergeLatestMessages(current, [incoming.content.chat]));
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
    syncMore,
    canSyncHistory,
    isEnd: () => isEnd() && !hasNextSyncPage(),
    loading,
    error: () => error() ?? historyNotice(),
  };
};
