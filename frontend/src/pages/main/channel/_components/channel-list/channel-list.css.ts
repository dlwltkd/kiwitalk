import { style } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  width: '100%',
  height: '100%',
  minHeight: 0,
  display: 'grid',
  gridTemplateRows: 'auto 24px minmax(0, 1fr)',
  background: vars.color.terminal.surface,
});

export const header = style({
  display: 'flex',
  flexDirection: 'column',
  gap: '9px',
  padding: '12px 10px 10px',
  borderBottom: `1px solid ${vars.color.terminal.border}`,
});

export const pathLine = style({
  display: 'flex',
  alignItems: 'center',
  gap: '2px',
  color: vars.color.terminal.foregroundBright,
  fontSize: '13px',
  fontWeight: 700,
});

export const prompt = style({
  color: vars.color.terminal.accentBright,
});

export const count = style({
  marginLeft: 'auto',
  color: vars.color.terminal.foregroundMuted,
  fontSize: '10px',
  fontWeight: 400,
  fontVariantNumeric: 'tabular-nums',
});

export const search = style({
  height: '28px',
  display: 'grid',
  gridTemplateColumns: '18px minmax(0, 1fr) auto',
  alignItems: 'center',
  padding: '0 7px',
  color: vars.color.terminal.foreground,
  background: vars.color.terminal.background,
  border: `1px solid ${vars.color.terminal.border}`,
  fontSize: '11px',

  selectors: {
    '&:focus-within': {
      borderColor: vars.color.terminal.accent,
    },
  },
});

export const searchInput = style({
  minWidth: 0,

  selectors: {
    '&::placeholder': {
      color: vars.color.terminal.foregroundMuted,
      opacity: 0.62,
    },
  },
});

export const searchClear = style({
  color: vars.color.terminal.foregroundMuted,
  fontSize: '9px',
  textTransform: 'uppercase',
});

export const searchPrompt = style({
  color: vars.color.terminal.accentBright,
  fontWeight: 700,
});

export const columns = style({
  display: 'grid',
  gridTemplateColumns: '28px minmax(0, 1fr) 40px',
  alignItems: 'center',
  gap: '6px',
  padding: '0 8px',
  color: vars.color.terminal.foregroundMuted,
  borderBottom: `1px solid ${vars.color.terminal.border}`,
  fontSize: '9px',
  textTransform: 'uppercase',
});

export const scrollArea = style({
  minHeight: 0,
  overflowY: 'auto',
  overflowX: 'hidden',
});

export const list = style({
  display: 'flex',
  flexDirection: 'column',
  padding: '4px 0',
});

export const empty = style({
  padding: '18px 10px',
  color: vars.color.terminal.foregroundMuted,
  fontSize: '11px',
});

export const error = style({
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: '8px',
  padding: '8px 10px',
  color: vars.color.red400,
  background: vars.color.terminal.background,
  borderBottom: `1px solid ${vars.color.terminal.border}`,
  fontSize: '10px',
});

export const retry = style({
  flexShrink: 0,
  color: vars.color.terminal.accentBright,
  fontSize: '9px',

  selectors: {
    '&:hover': {
      color: vars.color.terminal.foregroundBright,
    },
  },
});
