import { style, styleVariants } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

const base = style({
  width: '100%',
  minHeight: '54px',
  display: 'grid',
  gridTemplateColumns: '26px minmax(0, 1fr) auto',
  alignItems: 'center',
  gap: '8px',
  padding: '8px 10px',
  color: vars.color.terminal.foreground,
  background: vars.color.terminal.background,
  border: `1px solid ${vars.color.terminal.border}`,
  borderLeft: '2px solid transparent',
  textAlign: 'left',
});

export const container = styleVariants({
  selected: [base, {
    background: vars.color.terminal.selection,
    borderLeftColor: vars.color.terminal.accentBright,
  }],
  idle: [base, {
    selectors: {
      '&:hover': {
        background: vars.color.terminal.surfaceHover,
        borderLeftColor: vars.color.terminal.accent,
      },
    },
  }],
});

export const index = style({
  color: vars.color.terminal.accent,
  fontSize: '9px',
});

export const textContainer = style({
  minWidth: 0,
  display: 'flex',
  flexDirection: 'column',
  gap: '3px',
});

export const name = style({
  color: vars.color.terminal.foregroundBright,
  fontSize: '11px',
  fontWeight: 700,
  textTransform: 'uppercase',
});

export const email = style({
  overflow: 'hidden',
  color: vars.color.terminal.foregroundMuted,
  fontSize: '9px',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});

export const state = style({
  color: vars.color.terminal.accentBright,
  fontSize: '8px',
  textTransform: 'uppercase',
});
