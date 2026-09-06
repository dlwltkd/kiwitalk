import { useLocation, useNavigate, useRouteData } from '@solidjs/router';
import { useTransContext } from '@jellybrick/solid-i18next';
import { createSignal, onCleanup, onMount, Show } from 'solid-js';

import {
  cancelAndroidRegistration,
  pollAndroidRegistration,
  VerificationChallenge,
} from '@/api';
import { Button } from '@/ui-common/button';

import { ErrorTip } from '../_components/error-tip';
import * as styles from './page.css';

type RegistrationRouteState = {
  challenge?: VerificationChallenge;
};

export const DeviceRegisterPage = () => {
  const [t] = useTransContext();
  const navigate = useNavigate();
  const location = useLocation<RegistrationRouteState>();
  const refreshLoginState = useRouteData<() => () => void>();
  const challenge = location.state?.challenge;

  const [remainingSeconds, setRemainingSeconds] = createSignal(
    challenge?.remainingSeconds ?? 0,
  );
  const [error, setError] = createSignal<string | null>(null);
  let pollTimer: number | undefined;
  let countdownTimer: number | undefined;
  let disposed = false;
  let completed = false;

  const clearTimers = () => {
    if (pollTimer !== undefined) window.clearTimeout(pollTimer);
    if (countdownTimer !== undefined) window.clearInterval(countdownTimer);
  };

  const schedulePoll = (seconds: number) => {
    if (disposed) return;
    pollTimer = window.setTimeout(() => void poll(), Math.max(1, seconds) * 1000);
  };

  const poll = async () => {
    if (disposed || !challenge) return;

    try {
      const response = await pollAndroidRegistration(challenge.id);
      if (disposed) return;
      if (response.type === 'Failure') {
        setError(t(`login.status.device_register.${response.content}`));
        return;
      }

      switch (response.content.type) {
        case 'Pending':
          setRemainingSeconds(response.content.content.remainingSeconds);
          schedulePoll(response.content.content.nextRequestIntervalSeconds);
          break;
        case 'Authenticated':
          completed = true;
          clearTimers();
          refreshLoginState();
          navigate('/login/end', { replace: true });
          break;
        case 'Expired':
          clearTimers();
          setRemainingSeconds(0);
          setError(t('login.registration_expired'));
          break;
        case 'Stale':
          clearTimers();
          setError(t('login.registration_stale'));
          break;
      }
    } catch {
      if (disposed) return;
      setError(t('login.registration_network_error'));
      if (remainingSeconds() > 0) schedulePoll(2);
    }
  };

  const cancel = async () => {
    if (!challenge) {
      navigate('/login/login', { replace: true });
      return;
    }

    completed = true;
    clearTimers();
    await cancelAndroidRegistration(challenge.id).catch(() => undefined);
    navigate('/login/login', { replace: true });
  };

  onMount(() => {
    if (!challenge) {
      setError(t('login.registration_stale'));
      return;
    }

    countdownTimer = window.setInterval(() => {
      setRemainingSeconds((remaining) => Math.max(0, remaining - 1));
    }, 1000);
    void poll();
  });

  onCleanup(() => {
    disposed = true;
    clearTimers();
    if (challenge && !completed) {
      void cancelAndroidRegistration(challenge.id).catch(() => undefined);
    }
  });

  return (
    <div class={styles.container}>
      <div class={styles.heading}>
        <span class={styles.command}>$ device register --android</span>
        <span class={styles.caption}>enter this code in the KakaoTalk mobile app</span>
      </div>
      <ErrorTip message={error()} />
      <Show when={challenge} keyed>
        {(activeChallenge) => (
          <>
            <p class={styles.instruction}>
              {t('login.registration_instruction')}
            </p>
            <div
              class={styles.passcode}
              aria-label={t('login.registration_code_label')}
            >
              {activeChallenge.passcode}
            </div>
            <p class={styles.status}>
              {t('login.registration_waiting', { seconds: remainingSeconds() })}
            </p>
          </>
        )}
      </Show>
      <div class={styles.tool}>
        <Button type='button' variant='text' onClick={() => void cancel()}>
          {t('common.cancel')}
        </Button>
      </div>
    </div>
  );
};
