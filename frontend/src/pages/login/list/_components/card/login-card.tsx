import * as styles from './login-card.css';

export type LoginCardProps = {
  selected?: boolean;
  onClick?: () => void;
};

export const LoginCard = (props: LoginCardProps) => (
  <button
    aria-pressed={props.selected ?? false}
    class={styles.container[props.selected ? 'selected' : 'idle']}
    type="button"
    onClick={props.onClick}
  >
    <span class={styles.index}>01</span>
    <span class={styles.textContainer}>
      <span class={styles.name}>saved account</span>
      <span class={styles.email}>local profile available // identity hidden</span>
    </span>
    <span class={styles.state}>{props.selected ? '[selected]' : '[enter]'}</span>
  </button>
);
