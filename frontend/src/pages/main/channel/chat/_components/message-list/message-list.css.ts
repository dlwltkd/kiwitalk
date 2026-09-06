import { style, styleVariants } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  position: 'relative',
  minWidth: 0,
  minHeight: 0,
  overflow: 'hidden',
  background: vars.color.terminal.background,
});

export const virtualList = styleVariants({
  outer: {
    position: 'relative',
    width: '100%',
    height: '100%',
    overflowY: 'auto',
    overflowX: 'hidden',
  },
  inner: {
    position: 'absolute',
    inset: '0 0 auto',
    width: '100%',
  },
});

const noticeBase = style({
  position: 'absolute',
  top: '6px',
  left: '50%',
  zIndex: vars.layer.head,
  padding: '3px 7px',
  background: vars.color.terminal.surfaceRaised,
  border: `1px solid ${vars.color.terminal.border}`,
  fontSize: '9px',
  transform: 'translateX(-50%)',
  whiteSpace: 'nowrap',
});

export const notice = styleVariants({
  loading: [noticeBase, { color: vars.color.terminal.foregroundMuted }],
  error: [noticeBase, { color: vars.color.red400 }],
  end: [noticeBase, { color: vars.color.terminal.foregroundMuted }],
});

export const empty = style({
  position: 'absolute',
  inset: 0,
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  justifyContent: 'center',
  gap: '7px',
  color: vars.color.terminal.foregroundMuted,
  fontSize: '10px',
  pointerEvents: 'none',
});

export const emptyCommand = style({
  color: vars.color.terminal.accent,
});
