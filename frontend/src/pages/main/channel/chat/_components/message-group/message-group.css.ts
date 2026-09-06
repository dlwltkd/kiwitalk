import { style, styleVariants } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  width: '100%',
  padding: '7px 10px 9px',
  borderBottom: `1px solid ${vars.color.terminal.border}`,

  selectors: {
    '&:hover': {
      background: vars.color.terminal.surface,
    },
  },
});

const senderBase = style({
  display: 'flex',
  alignItems: 'baseline',
  gap: '7px',
  marginBottom: '4px',
  fontSize: '9px',
  fontWeight: 700,
  textTransform: 'uppercase',
});

export const sender = styleVariants({
  other: [senderBase, { color: vars.color.terminal.accent }],
  mine: [senderBase, { color: vars.color.terminal.accentBright }],
});

export const senderName = style({
  minWidth: 0,
  overflow: 'hidden',
  color: vars.color.terminal.foregroundBright,
  fontSize: '10px',
  textOverflow: 'ellipsis',
  textTransform: 'none',
  whiteSpace: 'nowrap',
});

export const messageContainer = style({
  display: 'flex',
  flexDirection: 'column',
  gap: '2px',
});
