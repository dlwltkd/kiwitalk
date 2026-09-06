import { useTransContext } from '@jellybrick/solid-i18next';

import { Button } from '@/ui-common/button';
import { Input } from '@/ui-common/input';

import * as styles from './page.css';

import IconUser from '@/assets/icons/user.svg';
import IconKey from '../_assets/icons/key.svg';
import { createEffect, createSignal } from 'solid-js';
import { loginWithResult } from '@/api';
import { useNavigate, useRouteData } from '@solidjs/router';
import { ErrorTip } from '../_components/error-tip';

export const LoginContentPage = () => {
  const [t] = useTransContext();
  const navigate = useNavigate();
  const refreshLoginState = useRouteData<() => () => void>();

  let loginInput: HTMLInputElement | null = null;
  let passwordInput: HTMLInputElement | null = null;

  const [error, setError] = createSignal<string | null>(null);
  const [submitting, setSubmitting] = createSignal(false);

  createEffect((prevTimeout: number | undefined) => {
    if (typeof error() === 'string') {
      if (prevTimeout !== undefined) clearTimeout(prevTimeout);

      const timeout = window.setTimeout(() => {
        setError(null);
      }, 5000);

      return timeout;
    }
  });

  const onLogin = async (event: Event) => {
    event.preventDefault();

    if (!loginInput?.value || !passwordInput?.value) {
      setError(t(`login.reason.required_id_password`));
      return;
    }

    if (submitting()) return;
    setSubmitting(true);
    const form = {
      email: loginInput.value,
      password: passwordInput.value,
      saveEmail: true, // input.saveId,
      autoLogin: false, // input.autoLogin,
    };
    passwordInput.value = '';
    try {
      const result = await loginWithResult(form);

      if (result.type === 'Success') {
        refreshLoginState();
        navigate('/');
      } else if (result.type === 'NeedRegister') {
        navigate('../device-register', {
          state: { challenge: result.challenge },
        });
      } else {
        setError(t(result.key));
        setSubmitting(false);
      }
    } catch {
      setError(t('login.reason.auto_login_failed.general'));
      setSubmitting(false);
    }
  };

  return (
    <form class={styles.loginForm} onSubmit={onLogin}>
      <div class={styles.heading}>
        <span class={styles.command}>$ auth --account</span>
        <span class={styles.caption}>credentials remain in the native process</span>
      </div>
      <ErrorTip message={error()} />
      <Input
        ref={(element) => loginInput = element}
        icon={<IconUser />}
        placeholder={t('login.id_placeholder')}
      />
      <Input
        ref={(element) => passwordInput = element}
        type={'password'}
        icon={<IconKey />}
        placeholder={t('login.password_placeholder')}
      />
      <Button disabled={submitting()}>
        {submitting() ? 'authenticating...' : t('login.login')}
      </Button>
    </form>
  );
};
