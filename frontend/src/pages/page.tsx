import { createResource, createEffect, on } from 'solid-js';
import { Navigate, Route, useNavigate } from '@solidjs/router';
import { useTransContext } from '@jellybrick/solid-i18next';

import { autoLogin, logon } from '@/api/api';
import { useConfig } from '@/features/config';

import { LoginBasePage } from './login';
import { MainPage } from './main';
import { ChannelListPage } from './main/channel';
import { LoginContentPage } from './login/login';
import { LoginListPage } from './login/list';
import { DeviceRegisterPage } from './login/device-register/page';
import { LoginEndPage } from './login/end';
import { ChatPage } from './main/channel/chat';
import { FriendListPage } from './main/friend';
import { TerminalPlaceholder } from './main/_components/placeholder';

export const App = () => {
  const [t, { changeLanguage }] = useTransContext();
  const [config] = useConfig();
  const navigate = useNavigate();

  const [isLogin, { refetch }] = createResource(async () => {
    if (await logon()) return true;

    let response;
    try {
      response = await autoLogin();
    } catch (error) {
      console.error(error);
      throw new Error(t('login.reason.auto_login_failed.general'));
    }

    if (response.type === 'Success') return response.content;
    throw new Error(t(`login.status.login.${response.content}`));
  });

  createEffect(on(config, (config) => {
    if (!config) return;

    if (config.global.locale.type === 'Auto') {
      changeLanguage(config.deviceLocale);
    } else {
      changeLanguage(config.global.locale.value);
    }
  }));

  createEffect(() => {
    if (isLogin.state === 'ready') {
      if (isLogin()) navigate('/main');
      else navigate('/login');
    } else if (isLogin.state === 'errored') {
      navigate('/login', { state: { error: isLogin.error } });
    }
  });

  return (
    <>
      <Route path={'/main'} component={MainPage}>
        <Route path={'/'} element={<Navigate href={'/main/chat'} />} />
        <Route path={'/chat'}>
          <Route path={'/'} component={ChannelListPage}>
            <Route path={'/:channelId?'} component={ChatPage} />
          </Route>
        </Route>
        <Route path={'/friends'}>
          <Route path={'/'} component={FriendListPage}>
            <Route path={'/'} component={ChatPage} />
          </Route>
        </Route>
        <Route path={'/*'} component={TerminalPlaceholder} />
      </Route>
      <Route path={'/login'} component={LoginBasePage}>
        <Route path={'/'} component={LoginBasePage} />
        <Route
          path={'/list'}
          component={LoginListPage}
          data={() => refetch}
        />
        <Route
          path={'/login'}
          component={LoginContentPage}
          data={() => refetch}
        />
        <Route
          path={'/device-register'}
          component={DeviceRegisterPage}
          data={() => refetch}
        />
        <Route path={'/end'} component={LoginEndPage} />
      </Route>
    </>
  );
};
