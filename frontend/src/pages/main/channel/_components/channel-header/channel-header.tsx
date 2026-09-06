import { Show } from 'solid-js';

import * as styles from './channel-header.css';

export type ChannelHeaderProps = {
  profile?: string;
  name?: string;
  members?: number;
  silent?: boolean;
  loading?: boolean;
  onBack?: () => void;
};

export const ChannelHeader = (props: ChannelHeaderProps) => (
  <header class={styles.container}>
    <button class={styles.back} type="button" onClick={props.onBack} aria-label="Back to channels">
      ‹
    </button>
    <div class={styles.identity}>
      <span class={styles.prompt}>#</span>
      <span class={styles.name}>{props.name || (props.loading ? 'loading...' : 'unnamed')}</span>
      <Show when={(props.members ?? 0) > 0}>
        <span class={styles.members}>{props.members} users</span>
      </Show>
    </div>
    <div class={styles.flags}>
      <Show when={props.silent}>
        <span>muted</span>
      </Show>
      <span class={styles.live}>● live</span>
    </div>
  </header>
);
