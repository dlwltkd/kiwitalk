import { style } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  width: '100%',
  height: '100%',
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  justifyContent: 'center',
  padding: '24px',
  color: vars.color.terminal.foregroundMuted,
  background: vars.color.terminal.background,
  textAlign: 'center',
});

export const mark = style({
  marginBottom: '14px',
  color: vars.color.terminal.accent,
  fontSize: '15px',
  lineHeight: '15px',
});

export const title = style({
  marginBottom: '6px',
  color: vars.color.terminal.foreground,
  fontSize: '12px',
  fontWeight: 700,
});

export const subtitle = style({
  maxWidth: '44ch',
  fontSize: '10px',
  lineHeight: '15px',
});
