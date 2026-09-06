import { style } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  width: '100%',
  minHeight: '44px',
  display: 'grid',
  gridTemplateColumns: '28px minmax(0, 1fr)',
  alignItems: 'center',
  gap: '6px',
  padding: '5px 8px',
  borderLeft: '2px solid transparent',

  selectors: {
    '&:hover': {
      background: vars.color.terminal.surfaceHover,
      borderLeftColor: vars.color.terminal.accent,
    },
  },
});

export const index = style({
  alignSelf: 'start',
  paddingTop: '2px',
  color: vars.color.terminal.accent,
  fontSize: '9px',
});

export const textContainer = style({
  minWidth: 0,
  display: 'flex',
  flexDirection: 'column',
  gap: '3px',
});

export const title = style({
  overflow: 'hidden',
  color: vars.color.terminal.foregroundBright,
  fontSize: '11px',
  fontWeight: 700,
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});

export const description = style({
  overflow: 'hidden',
  color: vars.color.terminal.foregroundMuted,
  fontSize: '9px',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});
