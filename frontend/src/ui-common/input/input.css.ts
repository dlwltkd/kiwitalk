import { style } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  width: '100%',
  minHeight: '34px',
  display: 'grid',
  gridTemplateColumns: '20px minmax(0, 1fr)',
  alignItems: 'center',
  gap: '7px',
  padding: '5px 8px',
  color: vars.color.terminal.foregroundMuted,
  background: vars.color.terminal.background,
  border: `1px solid ${vars.color.terminal.border}`,
  transition: `border-color ${vars.easing.background}`,

  selectors: {
    '&:focus-within': {
      borderColor: vars.color.terminal.accent,
    },
  },
});

export const input = style({
  width: '100%',
  minWidth: 0,
  color: vars.color.terminal.foregroundBright,
  fontSize: '11px',
  lineHeight: '20px',

  selectors: {
    '&::placeholder': {
      color: vars.color.terminal.foregroundMuted,
      opacity: 0.62,
    },
  },
});

export const iconWrapper = style({
  width: '16px',
  height: '16px',
  display: 'grid',
  placeItems: 'center',
  color: vars.color.terminal.accent,
  fontSize: '14px',
});
