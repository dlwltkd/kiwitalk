import { createEffect, createResource, createSignal, on } from 'solid-js';

import { getChannelList, meProfile } from '@/api';
import { useReady, useChannelEvent } from '@/pages/main/_hooks';

import { ChannelListItem } from '../_types';

export const useChannelList = () => {
  const isReady = useReady();
  const event = useChannelEvent();

  const [channelList, setChannelList] = createSignal<ChannelListItem[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [loaded, setLoaded] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  let refreshVersion = 0;

  const [myProfile] = createResource(async () => meProfile());

  const refreshChannelList = async () => {
    const requestVersion = ++refreshVersion;
    const result: ChannelListItem[] = [];

    setLoading(true);
    setError(null);
    try {
      for (const [id, item] of await getChannelList()) {
        const fallbackName = item.displayUsers
          .map((user) => user.nickname.trim())
          .filter(Boolean)
          .join(', ');
        result.push({
          id,
          name: item.name?.trim() || fallbackName || 'untitled room',
          displayUsers: item.displayUsers,
          lastChat: item.lastChat ? {
            ...item.lastChat,
            timestamp: new Date(item.lastChat.timestamp),
          } : undefined,
          userCount: item.userCount,
          unreadCount: item.unreadCount,
          profile: item.profile?.imageUrl ??
          (item.displayUsers.length === 1 ?
            item.displayUsers[0].profileUrl :
            undefined),
          silent: !item.pushAlert,
        });
      }

      if (requestVersion === refreshVersion) {
        setChannelList(result);
        setLoaded(true);
      }
    } catch {
      if (requestVersion === refreshVersion) {
        setError('channel list could not be loaded');
      }
    } finally {
      if (requestVersion === refreshVersion) setLoading(false);
    }
  };

  createEffect(on(event, (e) => {
    if (!e) return;

    const newChannelList = [...channelList()];
    const index = newChannelList.findIndex((item) => item.id === e?.channelId);

    if (index < 0) {
      if (e.type === 'Chat' || e.type === 'Added') {
        void refreshChannelList();
      }
      return;
    }

    if (e.type === 'Left') {
      newChannelList.splice(index, 1);
      setChannelList(newChannelList);
      void refreshChannelList();
      return;
    }

    if (e.type === 'ChatDeleted' || e.type === 'MetaChanged' || e.type === 'Added') {
      void refreshChannelList();
      return;
    }

    const channel = { ...newChannelList[index] };
    newChannelList[index] = channel;

    if (e.type === 'Chat') {
      const chat = e.content.chat;
      if (!e.content.read && chat.senderId !== myProfile()?.profile.id) channel.unreadCount += 1;
      else if (e.content.read) channel.unreadCount = 0;
      channel.lastChat = {
        chatType: chat.chatType,
        content: chat.content,
        attachment: chat.attachment,
        timestamp: new Date(chat.sendAt * 1000),
      };
    }
    if (e.type === 'ChatRead' && e?.content.userId === myProfile()?.profile.id) {
      channel.unreadCount = 0;
    }
    if (e.type === 'UnreadChanged') {
      channel.unreadCount = e.content.unreadCount;
    }
    setChannelList(newChannelList);
  }));

  createResource(isReady, async (ready) => {
    if (!ready) return;

    await refreshChannelList();
  });

  return {
    channels: channelList,
    loading,
    loaded,
    error,
    retry: () => void refreshChannelList(),
  };
};
