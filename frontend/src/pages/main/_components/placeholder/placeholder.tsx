import * as styles from './placeholder.css';

export const TerminalPlaceholder = () => (
  <section class={styles.container}>
    <span class={styles.command}>$ module --status</span>
    <span class={styles.message}>not implemented in this native milestone</span>
    <span class={styles.hint}>channels, transcript history, and text messaging are available now</span>
  </section>
);
