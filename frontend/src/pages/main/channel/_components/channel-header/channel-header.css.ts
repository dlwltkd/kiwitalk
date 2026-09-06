import { style } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  minWidth: 0,
  height: '44px',
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  padding: '0 12px',
  color: vars.color.terminal.foreground,
  background: vars.color.terminal.surface,
  borderBottom: `1px solid ${vars.color.terminal.border}`,
});

export const back = style({
  display: 'none',
  width: '24px',
  height: '24px',
  placeItems: 'center',
  color: vars.color.terminal.accentBright,
  border: `1px solid ${vars.color.terminal.border}`,

  '@media': {
    'screen and (max-width: 720px)': {
      display: 'grid',
    },
  },
});

export const identity = style({
  minWidth: 0,
  display: 'flex',
  alignItems: 'baseline',
  gap: '7px',
});

export const prompt = style({
  color: vars.color.terminal.accentBright,
  fontSize: '14px',
  fontWeight: 800,
});

export const name = style({
  minWidth: 0,
  overflow: 'hidden',
  color: vars.color.terminal.foregroundBright,
  fontSize: '12px',
  fontWeight: 700,
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});

export const members = style({
  flexShrink: 0,
  color: vars.color.terminal.foregroundMuted,
  fontSize: '9px',
});

export const flags = style({
  marginLeft: 'auto',
  display: 'flex',
  alignItems: 'center',
  gap: '9px',
  color: vars.color.terminal.foregroundMuted,
  fontSize: '9px',
  textTransform: 'uppercase',
});

export const live = style({
  color: vars.color.terminal.accentBright,
});
