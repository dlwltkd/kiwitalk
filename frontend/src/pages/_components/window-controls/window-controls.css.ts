import { style, styleVariants } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  minHeight: '30px',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  paddingLeft: '10px',
  color: vars.color.terminal.foregroundMuted,
  background: vars.color.terminal.surface,
  borderBottom: `1px solid ${vars.color.terminal.border}`,
  fontSize: '11px',
  lineHeight: 1,
  letterSpacing: '0.04em',
  zIndex: vars.layer.windowFrame,
  userSelect: 'none',
});

export const identity = style({
  minWidth: 0,
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
});

export const prompt = style({
  color: vars.color.terminal.accentBright,
  fontWeight: 700,
});

export const title = style({
  color: vars.color.terminal.foregroundBright,
  fontWeight: 700,
  textTransform: 'uppercase',
});

export const protocol = style({
  color: vars.color.terminal.foregroundMuted,
  textTransform: 'uppercase',

  '@media': {
    'screen and (max-width: 520px)': {
      display: 'none',
    },
  },
});

export const buttons = style({
  alignSelf: 'stretch',
  display: 'flex',
  alignItems: 'stretch',
});

const buttonBase = style({
  width: '34px',
  display: 'grid',
  placeItems: 'center',
  color: vars.color.terminal.foregroundMuted,
  fontSize: '13px',
  transition: `color ${vars.easing.fill}, background ${vars.easing.background}`,

  selectors: {
    '&:hover': {
      color: vars.color.terminal.foregroundBright,
      background: vars.color.terminal.surfaceHover,
    },
  },
});

export const buttonMinMax = styleVariants({
  active: [buttonBase],
  inactive: [buttonBase, { opacity: 0.55 }],
});

export const buttonClose = styleVariants({
  active: [buttonBase, {
    selectors: {
      '&:hover': {
        color: vars.color.terminal.background,
        background: vars.color.red400,
      },
    },
  }],
  inactive: [buttonBase, { opacity: 0.55 }],
});
