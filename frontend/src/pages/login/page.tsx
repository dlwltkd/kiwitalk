import { createEffect, createMemo, createResource, createSignal } from 'solid-js';
import { Outlet, useLocation, useMatch, useNavigate } from '@solidjs/router';
import { useTransContext } from '@jellybrick/solid-i18next';

import { defaultLoginForm } from '@/api';
import { LoginStepper } from './_components/login-stepper';

import * as styles from './page.css';

type LoginRouteState = {
  error?: unknown;
};

export const LoginBasePage = () => {
  const [t] = useTransContext();
  const location = useLocation<LoginRouteState>();
  const navigate = useNavigate();
  const isBasePage = useMatch(() => '/login');

  const [loginData] = createResource(async () => defaultLoginForm());
  const [enableBack, setEnableBack] = createSignal(true);

  createEffect(() => {
    if (!isBasePage()) return;

    if (loginData.state === 'ready') {
      if (loginData().email) {
        const error = location.state?.error;
        const sessionError = error instanceof Error
          ? error.message
          : typeof error === 'string'
            ? error
            : undefined;
        navigate('list', {
          replace: true,
          state: sessionError ? { sessionError } : undefined,
        });
      }
      else {
        setEnableBack(false);
        navigate('login', { replace: true });
      }
    }
  });

  const step = () => {
    const route = location.pathname.match(/login\/([^/]+)$/)?.[1];
    return route === 'list' ? 'login' : route;
  };
  const steps = createMemo(() => [
    {
      id: 'login',
      title: t('login.title'),
    },
    {
      id: 'device-register',
      title: t('login.register_title'),
    },
    {
      id: 'end',
      title: t('login.end_title'),
    },
  ]);

  return (
    <main class={styles.container}>
      <header class={styles.header}>
        <span><strong class={styles.headerUser}>auth</strong>@kiwitalk:~$ session --android</span>
        <span>encrypted transport</span>
      </header>
      <section class={styles.contentContainer}>
        <div class={styles.infoContainer}>
          <div class={styles.wordmark}>kiwi<span class={styles.wordmarkAccent}>//talk</span></div>
          <p class={styles.description}>
            native kakao client<br />
            loco protocol / android subdevice
          </p>
          <LoginStepper
            enableBack={enableBack()}
            steps={steps()}
            step={step()}
            onBack={() => navigate(-1)}
          />
        </div>
        <div class={styles.panel}>
          <Outlet />
        </div>
      </section>
      <footer class={styles.footer}>password memory: native process // password persistence: disabled</footer>
    </main>
  );
};
