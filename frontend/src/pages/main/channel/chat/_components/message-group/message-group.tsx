import { For } from 'solid-js';

import { ChannelUser, Chatlog } from '@/api/client';
import { Message } from '../message/message';

import * as styles from './message-group.css';

export type MessageGroupProps = {
  profile?: string;
  sender?: string;
  isMine: boolean;
  messages: Chatlog[];
  members: ChannelUser[];
};

export const MessageGroup = (props: MessageGroupProps) => {
  const variant = () => props.isMine ? 'mine' : 'other';

  const content = (message: Chatlog) => {
    if (message.content) return message.content;

    if ([2, 27].includes(message.chatType)) return '[image]';
    if ([6, 12, 20, 25].includes(message.chatType)) return '[emoticon]';
    if (message.attachment) return '[attachment]';

    return `[message type ${message.chatType}]`;
  };

  const getUnreadCount = (chat: Chatlog) => {
    try {
      const count = props.members
        .filter((user) => BigInt(user.watermark) < BigInt(chat.logId))
        .length;

      return count > 0 ? count : undefined;
    } catch {
      return undefined;
    }
  };

  return (
    <section class={styles.container}>
      <header class={styles.sender[variant()]}>
        <span>{props.isMine ? 'you' : 'usr'}</span>
        <span class={styles.senderName}>{props.sender || (props.isMine ? 'you' : 'peer')}</span>
      </header>
      <ul class={styles.messageContainer}>
        <For each={props.messages.toReversed()}>
          {(message) => (
            <Message
              isMine={props.isMine}
              time={message.sendAt}
              unread={getUnreadCount(message)}
            >
              {content(message)}
            </Message>
          )}
        </For>
      </ul>
    </section>
  );
};
