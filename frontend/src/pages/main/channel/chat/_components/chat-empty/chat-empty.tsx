import * as styles from './chat-empty.css';

export const ChatEmpty = () => (
  <div class={styles.container}>
    <pre class={styles.mark} aria-hidden="true">{'  ┌─┐\n  │·│\n  └─┘'}</pre>
    <p class={styles.title}>no channel selected</p>
    <p class={styles.subtitle}>use j/k to move, then enter to open a transcript</p>
  </div>
);
