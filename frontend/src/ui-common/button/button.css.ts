import { style, styleVariants } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const baseButton = style({
  minHeight: '30px',
  display: 'inline-flex',
  alignItems: 'center',
  justifyContent: 'center',
  gap: '7px',
  padding: '5px 10px',
  border: '1px solid transparent',
  fontSize: '10px',
  fontWeight: 700,
  lineHeight: '18px',
  textTransform: 'uppercase',
  transition: `color ${vars.easing.fill}, background ${vars.easing.background}, border-color ${vars.easing.background}`,

  selectors: {
    '&:disabled': {
      cursor: 'default',
      opacity: 0.45,
    },
  },
});

export const button = styleVariants({
  primary: [baseButton, {
    color: vars.color.terminal.background,
    background: vars.color.terminal.accentBright,
    borderColor: vars.color.terminal.accentBright,
    selectors: {
      '&:hover:not(:disabled)': {
        background: vars.color.terminal.foregroundBright,
        borderColor: vars.color.terminal.foregroundBright,
      },
    },
  }],
  secondary: [baseButton, {
    color: vars.color.terminal.foreground,
    background: vars.color.terminal.surfaceRaised,
    borderColor: vars.color.terminal.border,
    selectors: {
      '&:hover:not(:disabled)': {
        color: vars.color.terminal.foregroundBright,
        borderColor: vars.color.terminal.accent,
      },
    },
  }],
  text: [baseButton, {
    color: vars.color.terminal.foregroundMuted,
    background: 'transparent',
    selectors: {
      '&:hover:not(:disabled)': {
        color: vars.color.terminal.accentBright,
        background: vars.color.terminal.surfaceHover,
      },
    },
  }],
  glass: [baseButton, {
    color: vars.color.terminal.foreground,
    background: vars.color.terminal.surface,
    borderColor: vars.color.terminal.border,
    selectors: {
      '&:hover:not(:disabled)': {
        color: vars.color.terminal.accentBright,
        background: vars.color.terminal.surfaceHover,
      },
    },
  }],
});
