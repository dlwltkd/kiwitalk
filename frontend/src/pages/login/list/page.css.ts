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
  fontSize: '15px',
  fontWeight: 700,
});

export const caption = style({
  color: vars.color.terminal.foregroundMuted,
  fontSize: '9px',
  lineHeight: '14px',
  textTransform: 'uppercase',
});

export const tool = style({
  display: 'flex',
  flexWrap: 'wrap',
  justifyContent: 'flex-end',
  alignItems: 'center',
  gap: '6px',
  marginTop: '4px',
});
