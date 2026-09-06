
export type AuthState = 'NeedLogin' | 'Logon';

export type LoginForm = {
  email: string;
  password: string;
  saveEmail: boolean;
  autoLogin: boolean;
};

export type LoginDetailForm = LoginForm & {
  profile: string;
  name: string;
};

export type VerificationChallenge = {
  id: string;
  passcode: string;
  remainingSeconds: number;
};

export type LoginOutcome = { type: 'Authenticated' }
  | { type: 'VerificationRequired', content: VerificationChallenge };

export type RegistrationOutcome = {
  type: 'Pending';
  content: {
    remainingSeconds: number;
    nextRequestIntervalSeconds: number;
  };
} | { type: 'Authenticated' }
  | { type: 'Expired' }
  | { type: 'Stale' };

export type LoginResult = { type: 'Success'; }
  | { type: 'NeedRegister', challenge: VerificationChallenge }
  | { type: 'Error', key: string; forced?: boolean; };

/** @deprecated */
export type LogoutReason = {
  type: 'Kickout';
  reasonId: number;
} | {
  type: 'Error';
  err: unknown
} | {
  type: 'Disconnected';
} | {
  type: 'Logout';
};
