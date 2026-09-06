import { style } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  width: '100%',
  height: '100%',
  display: 'flex',
  flexDirection: 'column',
  justifyContent: 'center',
  alignItems: 'stretch',
  gap: '12px',
});

export const heading = style({
  display: 'flex',
  flexDirection: 'column',
  gap: '5px',
  marginBottom: '6px',
});

export const command = style({
  color: vars.color.terminal.foregroundBright,
  fontSize: '14px',
  fontWeight: 700,
});

export const caption = style({
  color: vars.color.terminal.foregroundMuted,
  fontSize: '9px',
  lineHeight: '14px',
  textTransform: 'uppercase',
});

export const instruction = style({
  color: vars.color.terminal.foreground,
  fontSize: '10px',
  lineHeight: '16px',
});

export const passcode = style({
  padding: '14px 16px',
  color: vars.color.terminal.accentBright,
  background: vars.color.terminal.background,
  border: `1px solid ${vars.color.terminal.accent}`,
  fontSize: 'clamp(28px, 5vw, 42px)',
  fontWeight: 800,
  fontVariantNumeric: 'tabular-nums',
  letterSpacing: '0.16em',
  textAlign: 'center',
  userSelect: 'text',
});

export const status = style({
  color: vars.color.terminal.foregroundMuted,
  fontSize: '9px',
  textAlign: 'right',
});

export const tool = style({
  display: 'flex',
  justifyContent: 'flex-end',
  alignItems: 'center',
});
