import { JSX, Show } from 'solid-js';

import * as styles from './message.css';

export type MessageProps = {
  unread?: number;
  time?: number;
  isMine?: boolean;
  isBubble?: boolean;
  isConnected?: boolean;
  children?: JSX.Element;
};

export const Message = (props: MessageProps) => {
  const variant = () => props.isMine ? 'mine' : 'other';
  const time = () => typeof props.time === 'number'
    ? new Date(props.time * 1000).toLocaleTimeString([], {
      hour: '2-digit',
      minute: '2-digit',
    })
    : '--:--';

  return (
    <li class={styles.container}>
      <time class={styles.time}>{time()}</time>
      <span class={styles.marker[variant()]}>{props.isMine ? '›' : '│'}</span>
      <div class={styles.content}>{props.children}</div>
      <Show when={(props.unread ?? 0) > 0}>
        <span class={styles.unread}>~{props.unread}</span>
      </Show>
    </li>
  );
};
