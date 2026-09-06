import { style, styleVariants } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

const channelBase = style({
  width: '100%',
  minHeight: '50px',
  display: 'grid',
  gridTemplateColumns: '28px minmax(0, 1fr) 40px',
  alignItems: 'center',
  gap: '6px',
  padding: '6px 8px',
  color: vars.color.terminal.foreground,
  borderLeft: '2px solid transparent',
  textAlign: 'left',
  transition: `color ${vars.easing.fill}, background ${vars.easing.background}`,
});

export const channel = styleVariants({
  active: [channelBase, {
    color: vars.color.terminal.foregroundBright,
    background: vars.color.terminal.selection,
    borderLeftColor: vars.color.terminal.accentBright,
  }],
  cursor: [channelBase, {
    background: vars.color.terminal.surfaceHover,
    borderLeftColor: vars.color.terminal.accent,
  }],
  inactive: [channelBase, {
    selectors: {
      '&:hover': {
        background: vars.color.terminal.surfaceHover,
      },
    },
  }],
});

export const index = style({
  alignSelf: 'start',
  paddingTop: '2px',
  color: vars.color.terminal.accent,
  fontSize: '9px',
  fontVariantNumeric: 'tabular-nums',
});

export const content = style({
  minWidth: 0,
  display: 'flex',
  flexDirection: 'column',
  gap: '4px',
});

export const heading = style({
  minWidth: 0,
  display: 'flex',
  alignItems: 'baseline',
  gap: '5px',
});

export const name = style({
  minWidth: 0,
  overflow: 'hidden',
  color: 'inherit',
  fontSize: '11px',
  fontWeight: 700,
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});

export const members = style({
  flexShrink: 0,
  color: vars.color.terminal.foregroundMuted,
  fontSize: '9px',
});

export const muted = style({
  flexShrink: 0,
  color: vars.color.terminal.borderStrong,
  fontSize: '8px',
  textTransform: 'uppercase',
});

export const preview = style({
  minWidth: 0,
  overflow: 'hidden',
  color: vars.color.terminal.foregroundMuted,
  fontSize: '10px',
  lineHeight: '13px',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});

export const meta = style({
  alignSelf: 'stretch',
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'flex-end',
  justifyContent: 'space-between',
  gap: '3px',
});

export const time = style({
  color: vars.color.terminal.foregroundMuted,
  fontSize: '8px',
  fontVariantNumeric: 'tabular-nums',
});

export const unread = style({
  minWidth: '17px',
  padding: '1px 3px',
  color: vars.color.terminal.background,
  background: vars.color.terminal.accentBright,
  fontSize: '9px',
  fontWeight: 700,
  lineHeight: '13px',
  textAlign: 'center',
  fontVariantNumeric: 'tabular-nums',
});
