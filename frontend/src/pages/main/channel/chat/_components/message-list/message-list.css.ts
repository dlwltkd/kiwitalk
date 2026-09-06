import { style, styleVariants } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  position: 'relative',
  minWidth: 0,
  minHeight: 0,
  overflow: 'hidden',
  display: 'grid',
  gridTemplateRows: 'auto minmax(0, 1fr)',
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

export const historyBar = style({
  display: 'flex',
  flexWrap: 'wrap',
  alignItems: 'center',
  gap: '6px 12px',
  minHeight: '28px',
  padding: '5px 12px',
  borderBottom: `1px solid ${vars.color.terminal.border}`,
  color: vars.color.terminal.foregroundMuted,
  fontSize: '10px',
});

export const historyWarning = style({
  flex: '1 1 240px',
});

export const historyButton = style({
  padding: '4px 7px',
  border: `1px solid ${vars.color.terminal.border}`,
  color: vars.color.terminal.accent,
  background: vars.color.terminal.surface,
  font: 'inherit',
  cursor: 'pointer',
  ':hover': { background: vars.color.terminal.surfaceRaised },
  ':focus-visible': { outline: `1px solid ${vars.color.terminal.accent}`, outlineOffset: '2px' },
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
