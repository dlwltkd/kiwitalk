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
  canSyncHistory?: boolean;
  onSyncMore?: () => void;
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
      <div class={styles.historyBar} aria-live="polite">
        <Show when={props.loading} fallback={
          <>
            <Show when={props.error} fallback={
              <Show when={props.isEnd && props.messageGroups.length > 0}>
                <span>All available messages are shown.</span>
              </Show>
            }>
              <span class={styles.historyWarning}>{props.error}</span>
            </Show>
            <Show when={!props.isEnd && props.onLoadMore}>
              <button type="button" class={styles.historyButton} onClick={props.onLoadMore}>Load earlier messages</button>
            </Show>
            <Show when={props.canSyncHistory && props.onSyncMore}>
              <button type="button" class={styles.historyButton} onClick={props.onSyncMore}>Load more history / retry</button>
            </Show>
          </>
        }>
          <span>Loading messages…</span>
        </Show>
      </div>
      <Show when={!props.loading && !props.error && props.messageGroups.length === 0}>
        <div class={styles.empty}>
          <span>No messages are available in this room yet.</span>
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
