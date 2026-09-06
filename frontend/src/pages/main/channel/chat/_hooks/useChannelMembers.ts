import { Accessor, createEffect, createSignal, on } from 'solid-js';

import { Channel, NormalChannelUser } from '@/api/client';
import { useChannelEvent } from '@/pages/main/_hooks';

export const useChannelMembers = (
  channelId: Accessor<string | null>,
  channel: Accessor<Channel | null>,
) => {
  const event = useChannelEvent();

  const [members, setMembers] = createSignal<Record<string, NormalChannelUser>>({});

  createEffect(on(channel, (channel) => {
    if (channel?.kind === 'normal') {
      setMembers(Object.fromEntries(channel.content.users));
    } else {
      setMembers({});
    }
  }));

  createEffect(on(event, (e) => {
    if (!e || e.channelId !== channelId()) return;

    if (e.type === 'Chat') {
      const { senderId: userId, logId } = e.content;

      setMembers((members) => {
        const result = { ...members };

        if (result[userId]) result[userId].watermark = logId;

        return result;
      });
    }

    if (e.type === 'ChatRead') {
      const { logId, userId } = e.content;

      setMembers((members) => {
        const result = { ...members };

        if (result[userId]) result[userId].watermark = logId;

        return result;
      });
    }
  }));

  return members;
};
