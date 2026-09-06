import { Accessor, createEffect, createResource, createSignal, on } from 'solid-js';
import { getChannelList, KiwiTalkEvent } from '@/api';

export const useSidebar = (
  isReady: Accessor<boolean>,
  event: Accessor<KiwiTalkEvent | null>,
) => {
  // FIXME create @/features/config and migrate to useConfiguration
  const [isNotificationActive, setIsNotificationActive] = createSignal(false);

  const [badges, { refetch }] = createResource(isReady, async (isReady) => {
    if (!isReady) {
      return {
        chat: '...',
        open: '...',
      };
    }

    let chatBadge = 0;
    let openChatBadge = 0;

    for (const [, item] of await getChannelList()) {
      // TODO: add open chat badge
      chatBadge += item.unreadCount;
      openChatBadge += 0;
    }

    return {
      chat: chatBadge,
      open: openChatBadge,
    };
  });

  createEffect(on(event, (incoming) => {
    if (isReady() && incoming?.type === 'Channel') void refetch();
  }));

  return {
    badges: () => badges(),

    notificationActive: () => isNotificationActive(),
    setNotificationActive: (active: boolean) => setIsNotificationActive(active),
  };
};
