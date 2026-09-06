import { style, styleVariants } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  minWidth: 0,
  display: 'grid',
  gridTemplateColumns: '42px 10px minmax(0, 1fr) auto',
  alignItems: 'start',
  gap: '5px',
  color: vars.color.terminal.foreground,
  fontSize: '11px',
  lineHeight: '17px',
});

export const time = style({
  paddingTop: '1px',
  color: vars.color.terminal.foregroundMuted,
  fontSize: '8px',
  fontVariantNumeric: 'tabular-nums',
});

export const marker = styleVariants({
  other: { color: vars.color.terminal.accent },
  mine: { color: vars.color.terminal.accentBright },
});

export const content = style({
  minWidth: 0,
  overflow: 'hidden',
  whiteSpace: 'pre-wrap',
  wordBreak: 'break-word',
});

export const unread = style({
  paddingTop: '1px',
  color: vars.color.terminal.accentBright,
  fontSize: '8px',
  fontVariantNumeric: 'tabular-nums',
});
