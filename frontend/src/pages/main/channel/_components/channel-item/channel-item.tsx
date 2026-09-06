import { Show } from 'solid-js';

import * as styles from './channel-item.css';

export type ChannelItemProps = {
  id: string;
  index: number;
  name: string;
  members: number;
  lastMessage?: string;
  lastMessageTime?: Date;
  profileSrc?: string;
  unreadBadge?: number;
  silent?: boolean;
  selected?: boolean;
  cursor?: boolean;
  onFocus?: () => void;
  onClick?: () => void;
};

const formatTime = (date?: Date) => {
  if (!date) return '--:--';

  const today = new Date();
  if (date.toDateString() === today.toDateString()) {
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  return date.toLocaleDateString([], { month: '2-digit', day: '2-digit' });
};

export const ChannelItem = (props: ChannelItemProps) => {
  const variant = () => props.selected ? 'active' : props.cursor ? 'cursor' : 'inactive';

  return (
    <li role="presentation">
      <button
        id={`channel-${props.id}`}
        aria-current={props.selected ? 'true' : undefined}
        class={styles.channel[variant()]}
        role="option"
        aria-selected={props.selected ?? false}
        tabIndex={props.cursor ? 0 : -1}
        type="button"
        onFocus={props.onFocus}
        onClick={props.onClick}
      >
        <span class={styles.index}>{String(props.index).padStart(2, '0')}</span>
        <span class={styles.content}>
          <span class={styles.heading}>
            <span class={styles.name}>{props.name || '(unnamed)'}</span>
            <Show when={props.members > 1}>
              <span class={styles.members}>+{props.members}</span>
            </Show>
            <Show when={props.silent}>
              <span class={styles.muted}>muted</span>
            </Show>
          </span>
          <span class={styles.preview}>{props.lastMessage || '[no local history]'}</span>
        </span>
        <span class={styles.meta}>
          <span class={styles.time}>{formatTime(props.lastMessageTime)}</span>
          <Show when={(props.unreadBadge ?? 0) > 0}>
            <span class={styles.unread}>{props.unreadBadge}</span>
          </Show>
        </span>
      </button>
    </li>
  );
};
