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

export const title = style({
  color: vars.color.terminal.accentBright,
  fontSize: '22px',
  fontWeight: 800,
  lineHeight: '30px',
  whiteSpace: 'pre-line',
});

export const subtitle = style({
  marginBottom: '14px',
  color: vars.color.terminal.foregroundMuted,
  fontSize: '10px',
  lineHeight: '16px',
});
