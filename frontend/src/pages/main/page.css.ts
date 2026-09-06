import { style, styleVariants } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  width: '100%',
  height: '100%',
  minHeight: 0,
  display: 'grid',
  gridTemplateColumns: 'auto minmax(0, 1fr)',
  gridTemplateRows: 'minmax(0, 1fr) 24px',
  color: vars.color.terminal.foreground,
  background: vars.color.terminal.background,
});

export const workspace = style({
  minWidth: 0,
  minHeight: 0,
  overflow: 'hidden',
});

export const statusLine = style({
  minWidth: 0,
  display: 'flex',
  alignItems: 'center',
  gap: '12px',
  padding: '0 10px',
  color: vars.color.terminal.foregroundMuted,
  background: vars.color.terminal.surface,
  borderTop: `1px solid ${vars.color.terminal.border}`,
  fontSize: '10px',
  lineHeight: 1,
  textTransform: 'uppercase',
  whiteSpace: 'nowrap',
  overflow: 'hidden',
});

export const connection = styleVariants({
  ready: { color: vars.color.terminal.accentBright },
  pending: { color: vars.color.yellow400 },
});

export const shortcuts = style({
  minWidth: 0,
  marginLeft: 'auto',
  overflow: 'hidden',
  textOverflow: 'ellipsis',

  '@media': {
    'screen and (max-width: 820px)': {
      display: 'none',
    },
  },
});
