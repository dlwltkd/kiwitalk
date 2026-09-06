import { Show } from 'solid-js';

import * as styles from './sidebar.css';

type SidebarButtonProps = {
  active: boolean;
  badge?: string | number;
  label: string;
  shortcut: string;
  onClick: () => void;
};

const SidebarButton = (props: SidebarButtonProps) => (
  <button
    aria-current={props.active ? 'page' : undefined}
    class={styles.item[props.active ? 'active' : 'inactive']}
    onClick={props.onClick}
    type="button"
  >
    <span class={styles.shortcut}>{props.shortcut}</span>
    <span class={styles.label}>{props.label}</span>
    <Show when={props.badge !== undefined && props.badge !== 0}>
      <span class={styles.badge}>{props.badge}</span>
    </Show>
  </button>
);

export type SidebarProps = {
  collapsed?: boolean;
  activePath: string;
  setActivePath: (path: string) => void;
  chatBadges?: string | number;
  notificationActive?: boolean;
  onNotificationActive?: (active: boolean) => void;
};

export const Sidebar = (props: SidebarProps) => (
  <aside class={styles.sidebar}>
    <div class={styles.brand} aria-label="KiwiTalk">
      <span class={styles.brandMark}>kt</span>
      <span class={styles.brandCursor}>_</span>
    </div>

    <nav class={styles.navigation} aria-label="Primary">
      <SidebarButton
        active={props.activePath === 'chat'}
        badge={props.chatBadges}
        label="chats"
        shortcut="01"
        onClick={() => props.setActivePath('chat')}
      />
      <SidebarButton
        active={props.activePath === 'friends'}
        label="people"
        shortcut="02"
        onClick={() => props.setActivePath('friends')}
      />
    </nav>

    <div class={styles.navigation}>
      <button
        aria-pressed={props.notificationActive ?? false}
        class={styles.utility}
        onClick={() => props.onNotificationActive?.(!props.notificationActive)}
        type="button"
      >
        <span class={styles.shortcut}>nt</span>
        <span class={styles.label}>
          {props.notificationActive ? 'notify:on' : 'notify:off'}
        </span>
      </button>
      <SidebarButton
        active={props.activePath === 'settings'}
        label="config"
        shortcut="03"
        onClick={() => props.setActivePath('settings')}
      />
    </div>
  </aside>
);
