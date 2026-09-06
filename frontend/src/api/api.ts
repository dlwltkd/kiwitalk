import { invoke } from '@tauri-apps/api/core';

import {
  FriendsUpdateResult,
  LoginDetailForm,
  LoginForm,
  LoginOutcome,
  LoginResult,
  LogonProfile,
  UserProfile,
  Response,
  RegistrationOutcome,
} from './_types';

export function defaultLoginForm(): Promise<LoginDetailForm> {
  return invoke('plugin:api|default_login_form');
}

export function logon(): Promise<boolean> {
  return invoke('plugin:api|logon');
}

export function autoLogin(): Promise<Response<boolean>> {
  return invoke('plugin:api|auto_login');
}

export function login(form: LoginForm): Promise<Response<LoginOutcome>> {
  return invoke('plugin:api|login', { form });
}

export function logout(): Promise<boolean> {
  return invoke('plugin:api|logout');
}

export function pollAndroidRegistration(id: string): Promise<Response<RegistrationOutcome>> {
  return invoke('plugin:api|poll_android_registration', { id });
}

export function cancelAndroidRegistration(id: string): Promise<Response<boolean>> {
  return invoke('plugin:api|cancel_android_registration', { id });
}

export function meProfile(): Promise<LogonProfile> {
  return invoke('plugin:api|me_profile');
}

export function friendProfile(id: string): Promise<UserProfile> {
  return invoke('plugin:api|friend_profile', { id });
}

export function updateFriends(friendIds: string[]): Promise<FriendsUpdateResult> {
  return invoke('plugin:api|update_friends', { friendIds });
}

const loginFailureKey = (error: unknown): string => {
  const message = error instanceof Error
    ? error.message
    : typeof error === 'string'
      ? error
      : '';

  if (/database\s+(?:is\s+)?(?:locked|busy)/i.test(message)) {
    return 'login.reason.database_busy';
  }
  if (/timed?\s*out|timeout/i.test(message)) {
    return 'login.reason.network_timeout';
  }
  if (/allowlist|network|request|connect|dns|resolve|socket|tls/i.test(message)) {
    return 'login.reason.network_error';
  }

  return 'login.unknown_error';
};

export const loginWithResult = async (
  loginForm: LoginForm,
): Promise<LoginResult> => {
  if (!loginForm?.email || !loginForm?.password) {
    return {
      type: 'Error',
      key: 'login.reason.required_id_password',
    };
  }

  let key: string;
  let newForced = false;
  try {
    const response = await login({
      email: loginForm.email,
      password: loginForm.password,
      saveEmail: loginForm.saveEmail,
      autoLogin: loginForm.autoLogin,
    });

    if (response.type === 'Success') {
      if (response.content.type === 'Authenticated') return { type: 'Success' };
      return {
        type: 'NeedRegister',
        challenge: response.content.content,
      };
    }

    if (response.content === -101) newForced = true;

    key = `login.status.login.${response.content}`;
  } catch (e) {
    console.error(e);
    key = loginFailureKey(e);
  }

  return {
    type: 'Error',
    key,
    forced: newForced,
  };
};
