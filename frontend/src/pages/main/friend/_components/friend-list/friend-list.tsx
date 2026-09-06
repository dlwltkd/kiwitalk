import { For, Show } from 'solid-js';

import { FriendProfile, LogonProfile } from '@/api';
import { FriendItem } from '../friend-item';

import * as styles from './friend-list.css';

export type FriendListProps = {
  me?: LogonProfile;
  all?: FriendProfile[];
  pinned?: FriendProfile[];
  nearBirthday?: FriendProfile[];
};

export const FriendList = (props: FriendListProps) => (
  <section class={styles.container} aria-label="People">
    <header class={styles.header}>
      <span><strong class={styles.prompt}>~</strong>/people</span>
      <span class={styles.count}>[{props.all?.length ?? 0}]</span>
    </header>
    <div class={styles.columns}>
      <span>id</span>
      <span>identity / status</span>
    </div>
    <div class={styles.scrollArea}>
      <Show when={props.me} keyed>
        {(profile) => (
          <section>
            <div class={styles.section}>self</div>
            <ul>
              <FriendItem
                index={0}
                name={profile.nickname}
                description={profile.profile.statusMessage}
              />
            </ul>
          </section>
        )}
      </Show>
      <section>
        <div class={styles.section}>contacts</div>
        <ul>
          <For each={props.all}>
            {(friend, index) => (
              <FriendItem
                index={index() + 1}
                name={friend.nickname}
                description={friend.statusMessage}
              />
            )}
          </For>
        </ul>
        <Show when={(props.all?.length ?? 0) === 0}>
          <div class={styles.empty}>no contacts available from the current profile endpoint</div>
        </Show>
      </section>
    </div>
  </section>
);
