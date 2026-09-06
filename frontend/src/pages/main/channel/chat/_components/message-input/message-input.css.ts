import { style } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  minHeight: '46px',
  display: 'grid',
  gridTemplateColumns: '18px minmax(0, 1fr) auto',
  alignItems: 'end',
  gap: '6px',
  padding: '8px 10px',
  color: vars.color.terminal.foreground,
  background: vars.color.terminal.surface,
  borderTop: `1px solid ${vars.color.terminal.border}`,
});

export const prompt = style({
  alignSelf: 'center',
  color: vars.color.terminal.accentBright,
  fontSize: '13px',
  fontWeight: 800,
});

export const input = style({
  width: '100%',
  minHeight: '28px',
  maxHeight: '104px',
  padding: '6px 8px',
  color: vars.color.terminal.foregroundBright,
  background: vars.color.terminal.background,
  border: `1px solid ${vars.color.terminal.border}`,
  fontSize: '11px',
  lineHeight: '15px',
  overflowY: 'auto',
  resize: 'none',
  whiteSpace: 'pre-wrap',
  wordBreak: 'break-word',

  selectors: {
    '&:focus': {
      borderColor: vars.color.terminal.accent,
    },
    '&::placeholder': {
      color: vars.color.terminal.foregroundMuted,
      opacity: 0.58,
    },
    '&:disabled': {
      opacity: 0.5,
    },
  },
});

export const button = style({
  minHeight: '28px',
  padding: '0 7px',
  color: vars.color.terminal.accentBright,
  border: `1px solid ${vars.color.terminal.border}`,
  fontSize: '9px',
  whiteSpace: 'nowrap',

  selectors: {
    '&:hover': {
      color: vars.color.terminal.background,
      background: vars.color.terminal.accentBright,
    },
    '&:disabled': {
      cursor: 'default',
      opacity: 0.5,
    },
  },
});
