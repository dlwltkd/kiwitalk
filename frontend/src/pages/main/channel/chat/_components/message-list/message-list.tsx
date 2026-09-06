import {
  JSX,
  Show,
  createEffect,
  createMemo,
  createRenderEffect,
  createSignal,
  on,
  untrack,
} from 'solid-js';

import { ChannelUser, Chatlog } from '@/api/client';
import { VirtualList, VirtualListRef } from '@/ui-common/virtual-list';
import { MessageGroup } from '../message-group';

import * as styles from './message-list.css';

export type MessageListProps = {
  scroller?: (ref: VirtualListRef) => void;
  channelId: string;
  messageGroups: Chatlog[][];
  members: Record<string, ChannelUser>;
  logonId?: string;
  isEnd?: boolean;
  loading?: boolean;
  error?: string | null;
  onLoadMore?: () => void;
};

export const MessageList = (props: MessageListProps) => {
  const [stickToBottom, setStickToBottom] = createSignal(true);
  const members = createMemo(() => Object.values(props.members));

  createRenderEffect(on(() => props.channelId, () => setStickToBottom(true)));
  createEffect(on(() => props.messageGroups.length, (length) => {
    if (length > 0 && stickToBottom()) {
      requestAnimationFrame(() => setStickToBottom(false));
    }
  }, { defer: false }));

  const onScroll: JSX.EventHandlerUnion<HTMLUListElement, Event> = (event) => {
    if (!props.isEnd && !props.loading && event.currentTarget.scrollTop <= 24) {
      props.onLoadMore?.();
    }
  };

  return (
    <div class={styles.container}>
      <Show when={props.error}>
        <div class={styles.notice.error}>! {props.error}</div>
      </Show>
      <Show when={props.loading}>
        <div class={styles.notice.loading}>… loading transcript</div>
      </Show>
      <Show when={props.isEnd && props.messageGroups.length > 0}>
        <div class={styles.notice.end}>— beginning of local history —</div>
      </Show>
      <Show when={!props.loading && !props.error && props.messageGroups.length === 0}>
        <div class={styles.empty}>
          <span class={styles.emptyCommand}>$ history --local</span>
          <span>no messages are stored for this channel yet</span>
        </div>
      </Show>

      <VirtualList
        reverse
        component="ul"
        ref={props.scroller}
        items={props.messageGroups}
        class={styles.virtualList.outer}
        innerClass={styles.virtualList.inner}
        topMargin={8}
        bottomMargin={8}
        estimatedItemHeight={72}
        alignToBottom={stickToBottom()}
        onScroll={onScroll}
      >
        {(group) => {
          const senderId = untrack(() => group![0].senderId);
          const mine = senderId === props.logonId;

          return (
            <MessageGroup
              sender={props.members[senderId]?.nickname ?? (mine ? 'you' : 'peer')}
              isMine={mine}
              messages={group!}
              members={members()}
            />
          );
        }}
      </VirtualList>
    </div>
  );
};
