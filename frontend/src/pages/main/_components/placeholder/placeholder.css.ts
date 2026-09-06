import { style } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  width: '100%',
  height: '100%',
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  justifyContent: 'center',
  gap: '8px',
  padding: '24px',
  color: vars.color.terminal.foregroundMuted,
  background: vars.color.terminal.background,
  textAlign: 'center',
});

export const command = style({
  color: vars.color.terminal.accent,
  fontSize: '11px',
});

export const message = style({
  color: vars.color.terminal.foregroundBright,
  fontSize: '12px',
  fontWeight: 700,
});

export const hint = style({
  maxWidth: '54ch',
  fontSize: '9px',
  lineHeight: '14px',
});
