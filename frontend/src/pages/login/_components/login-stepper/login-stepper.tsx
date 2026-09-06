import { For, JSX, Show, mergeProps } from 'solid-js';
import { useTransContext } from '@jellybrick/solid-i18next';

import { Button } from '@/ui-common/button';
import * as styles from './login-stepper.css';

type Step = {
  id: string;
  title: string;
  icon?: JSX.Element;
};

export type LoginStepperProps = {
  enableBack?: boolean;
  steps?: Step[];
  step?: string;
  onBack?: () => void;
};

export const LoginStepper = (props: LoginStepperProps) => {
  const local = mergeProps({ steps: [] }, props);
  const [t] = useTransContext();
  const activeIndex = () => local.steps.findIndex((step) => step.id === props.step);

  return (
    <div class={styles.container}>
      <div class={styles.label}>auth flow</div>
      <ol class={styles.steps}>
        <For each={local.steps}>
          {(step, index) => {
            const variant = () => index() === activeIndex()
              ? 'active'
              : index() < activeIndex() ? 'complete' : 'pending';

            return (
              <li class={styles.step[variant()]}>
                <span>{String(index() + 1).padStart(2, '0')}</span>
                <span>{step.title}</span>
              </li>
            );
          }}
        </For>
      </ol>
      <Show when={props.enableBack}>
        <Button variant="text" onClick={local.onBack} class={styles.back}>
          ← {t('common.prev')}
        </Button>
      </Show>
    </div>
  );
};
