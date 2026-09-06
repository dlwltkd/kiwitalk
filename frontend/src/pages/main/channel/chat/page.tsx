import {
  Show,
  createResource,
  createSignal,
} from 'solid-js';
import { useNavigate, useParams } from '@solidjs/router';
import { useTransContext } from '@jellybrick/solid-i18next';

import { getChannelList, meProfile } from '@/api';
import { sendText } from '@/api/client';
import { VirtualListRef } from '@/ui-common/virtual-list';
import { useReady } from '@/pages/main/_hooks';
import { ChannelHeader } from '../_components/channel-header';
import { ChatEmpty } from './_components/chat-empty';
import { MessageInput } from './_components/message-input';
import { MessageList } from './_components/message-list';
import { useChannel, useChannelMembers, useMessageList } from './_hooks';

import * as styles from './page.css';

type ActiveChatProps = {
  channelId: string;
};

const ActiveChat = (props: ActiveChatProps) => {
  const isReady = useReady();
  const [t] = useTransContext();
  const navigate = useNavigate();
  const channelId = () => props.channelId;
  const channelState = useChannel(channelId);
  const channel = channelState.channel;
  const members = useChannelMembers(channelId, channel);
  const transcript = useMessageList(channelId);
  const [sendError, setSendError] = createSignal<string | null>(null);
  const [observedSelfId, setObservedSelfId] = createSignal<string | undefined>();
  const [scroller, setScroller] = createSignal<VirtualListRef | null>(null);

  const [me] = createResource(isReady, async (ready) => ready ? meProfile() : null);
  const [channelInfo] = createResource(
    () => [isReady(), props.channelId] as const,
    async ([ready, id]) => {
      if (!ready) return null;

      return Object.fromEntries(await getChannelList())[id] ?? null;
    },
  );

  const scrollToBottom = () => {
    const element = scroller()?.element;
    if (!element) return;

    requestAnimationFrame(() => element.scrollTo({
      top: element.scrollHeight,
      behavior: 'smooth',
    }));
  };

  const selfId = () => me()?.profile.id ?? observedSelfId();

  const onSubmit = async (text: string) => {
    setSendError(null);
    try {
      const result = await sendText(props.channelId, text);
      setObservedSelfId(result.senderId);

      scrollToBottom();
    } catch (error) {
      setSendError('message was not sent; your draft is still here');
      throw error;
    }
  };

  return (
    <section class={styles.container}>
      <ChannelHeader
        name={channelInfo()?.name?.trim() || 'untitled room'}
        members={channelInfo()?.userCount ?? 0}
        loading={channelInfo.loading}
        onBack={() => navigate('/main/chat')}
      />

      <Show when={channelState.error()}>
        <div class={styles.channelError}>! {channelState.error()}</div>
      </Show>

      <MessageList
        scroller={setScroller}
        channelId={props.channelId}
        logonId={selfId()}
        messageGroups={transcript.messageGroups()}
        members={members()}
        isEnd={transcript.isEnd()}
        loading={transcript.loading()}
        error={transcript.error()}
        onLoadMore={transcript.loadMore}
      />

      <Show when={sendError()}>
        <div class={styles.commandError}>! {sendError()}</div>
      </Show>
      <MessageInput
        placeholder={t('main.chat.placeholder')}
        disabled={!isReady()}
        onSubmit={onSubmit}
      />
    </section>
  );
};

export const ChatPage = () => {
  const params = useParams();

  return (
    <Show when={params.channelId} keyed fallback={<ChatEmpty />}>
      {(channelId) => <ActiveChat channelId={channelId} />}
    </Show>
  );
};
