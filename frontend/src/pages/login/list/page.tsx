import { useLocation, useNavigate, useRouteData } from '@solidjs/router';
import { Show, createResource, createSignal } from 'solid-js';
import { useTransContext } from '@jellybrick/solid-i18next';

import { LoginDetailForm, defaultLoginForm, loginWithResult } from '@/api';

import { Button } from '@/ui-common/button';
import { Input } from '@/ui-common/input';
import { LoginCard } from './_components/card';
import { ErrorTip } from '../_components/error-tip';

import * as styles from './page.css';

import IconKey from '../_assets/icons/key.svg';

type LoginListRouteState = {
  sessionError?: string;
};

export const LoginListPage = () => {
  const [t] = useTransContext();
  const navigate = useNavigate();
  const location = useLocation<LoginListRouteState>();
  const refreshLoginState = useRouteData<() => () => void>();

  let passwordInput: HTMLInputElement | null = null;
  const [selectedLoginData, setSelectedLoginData] = createSignal<LoginDetailForm | null>(null);
  const [error, setError] = createSignal<string | null>(
    location.state?.sessionError ?? null,
  );
  const [submitting, setSubmitting] = createSignal(false);

  const [loginData] = createResource(async () => defaultLoginForm());

  const onAddAccount = () => {
    navigate('../login');
  };
  const onToggleLoginData = () => {
    if (selectedLoginData()) setSelectedLoginData(null);
    else setSelectedLoginData(loginData() ?? null);

    passwordInput?.focus();
  };
  const onLogin = async (e: SubmitEvent) => {
    e.preventDefault();

    const data = loginData();

    if (!data || !passwordInput?.value) {
      setError(t(`login.reason.required_id_password`));
      return;
    }

    if (submitting()) return;
    setSubmitting(true);
    const form = {
      email: data.email,
      password: passwordInput.value,
      saveEmail: data.saveEmail,
      autoLogin: data.autoLogin,
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
    <form class={styles.container} onSubmit={onLogin}>
      <div class={styles.heading}>
        <span class={styles.command}>$ auth --saved</span>
        <span class={styles.caption}>select the local account, then enter its password</span>
      </div>
      <ErrorTip message={error()} />
      <LoginCard selected={!!selectedLoginData()} onClick={onToggleLoginData} />
      <Show when={selectedLoginData()} keyed>
        <Input
          ref={(element) => passwordInput = element}
          type={'password'}
          icon={<IconKey />}
          placeholder={t('login.password_placeholder')}
        />
      </Show>
      <div class={styles.tool}>
        <Button variant="text" type="button" onClick={onAddAccount}>
          + {t('login.add_account')}
        </Button>
        <Show when={!!selectedLoginData()}>
          <Button disabled={submitting()}>
            {submitting() ? 'authenticating...' : t('login.login')}
          </Button>
        </Show>
      </div>
    </form>
  );
};
