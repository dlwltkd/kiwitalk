import { style, styleVariants } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const sidebar = style({
  gridRow: '1 / -1',
  width: '112px',
  minHeight: 0,
  display: 'flex',
  flexDirection: 'column',
  justifyContent: 'space-between',
  padding: '10px 8px 8px',
  color: vars.color.terminal.foregroundMuted,
  background: vars.color.terminal.surface,
  borderRight: `1px solid ${vars.color.terminal.border}`,

  '@media': {
    'screen and (max-width: 720px)': {
      width: '46px',
      paddingInline: '5px',
    },
  },
});

export const brand = style({
  height: '32px',
  display: 'flex',
  alignItems: 'baseline',
  paddingInline: '7px',
  color: vars.color.terminal.foregroundBright,
  fontSize: '14px',
  fontWeight: 800,
  letterSpacing: '-0.04em',
  textTransform: 'lowercase',
});

export const brandMark = style({});

export const brandCursor = style({
  color: vars.color.terminal.accentBright,
});

export const navigation = style({
  display: 'flex',
  flexDirection: 'column',
  gap: '3px',
});

const itemBase = style({
  position: 'relative',
  width: '100%',
  minHeight: '32px',
  display: 'grid',
  gridTemplateColumns: '22px minmax(0, 1fr) auto',
  alignItems: 'center',
  gap: '4px',
  padding: '5px 7px',
  borderLeft: '2px solid transparent',
  color: vars.color.terminal.foregroundMuted,
  fontSize: '11px',
  transition: `color ${vars.easing.fill}, background ${vars.easing.background}`,

  '@media': {
    'screen and (max-width: 720px)': {
      gridTemplateColumns: '1fr',
      justifyItems: 'center',
      paddingInline: '3px',
    },
  },
});

export const item = styleVariants({
  active: [itemBase, {
    color: vars.color.terminal.foregroundBright,
    background: vars.color.terminal.selection,
    borderLeftColor: vars.color.terminal.accentBright,
  }],
  inactive: [itemBase, {
    selectors: {
      '&:hover': {
        color: vars.color.terminal.foreground,
        background: vars.color.terminal.surfaceHover,
      },
    },
  }],
});

export const utility = style([itemBase, {
  selectors: {
    '&:hover': {
      color: vars.color.terminal.foreground,
      background: vars.color.terminal.surfaceHover,
    },
  },
}]);

export const shortcut = style({
  color: vars.color.terminal.accent,
  fontSize: '10px',
  fontVariantNumeric: 'tabular-nums',
});

export const label = style({
  minWidth: 0,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',

  '@media': {
    'screen and (max-width: 720px)': {
      display: 'none',
    },
  },
});

export const badge = style({
  minWidth: '17px',
  padding: '1px 3px',
  color: vars.color.terminal.background,
  background: vars.color.terminal.accentBright,
  fontSize: '9px',
  lineHeight: '13px',
  textAlign: 'center',
  fontVariantNumeric: 'tabular-nums',

  '@media': {
    'screen and (max-width: 720px)': {
      position: 'absolute',
      top: '1px',
      right: '1px',
      minWidth: '12px',
      padding: 0,
      fontSize: '8px',
      lineHeight: '12px',
    },
  },
});
