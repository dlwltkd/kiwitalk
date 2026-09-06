import { useTransContext } from '@jellybrick/solid-i18next';

import * as styles from './window-controls.css';

type WindowControlsProps = {
  isActive?: boolean;

  onMinimize?: () => void;
  onMaximize?: () => void;
  onClose?: () => void;
};

export const WindowControls = (props: WindowControlsProps) => {
  const [t] = useTransContext();

  const buttonVariant = () => props.isActive ? 'active' : 'inactive';

  return (
    <div data-tauri-drag-region class={styles.container}>
      <div data-tauri-drag-region class={styles.identity}>
        <span data-tauri-drag-region class={styles.prompt}>❯_</span>
        <span data-tauri-drag-region class={styles.title}>kiwitalk</span>
        <span data-tauri-drag-region class={styles.protocol}>native // loco</span>
      </div>
      <div data-tauri-drag-region class={styles.buttons}>
        <button
          aria-label={t('window-controls.minimize')}
          class={styles.buttonMinMax[buttonVariant()]}
          onClick={props.onMinimize}
          type="button"
        >
          −
        </button>
        <button
          aria-label={t('window-controls.maximize')}
          class={styles.buttonMinMax[buttonVariant()]}
          onClick={props.onMaximize}
          type="button"
        >
          □
        </button>
        <button
          aria-label={t('window-controls.close')}
          class={styles.buttonClose[buttonVariant()]}
          onClick={props.onClose}
          type="button"
        >
          ×
        </button>
      </div>
    </div>
  );
};
