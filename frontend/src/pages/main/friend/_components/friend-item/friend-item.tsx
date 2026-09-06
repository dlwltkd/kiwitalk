import * as styles from './friend-item.css';

export type FriendItemProps = {
  profile?: string;
  name: string;
  description?: string;
  collapsed?: boolean;
  index?: number;
};

export const FriendItem = (props: FriendItemProps) => (
  <li class={styles.container}>
    <span class={styles.index}>{String(props.index ?? 0).padStart(2, '0')}</span>
    <span class={styles.textContainer}>
      <span class={styles.title}>{props.name || 'unknown'}</span>
      <span class={styles.description}>{props.description || '[no status]'}</span>
    </span>
  </li>
);
