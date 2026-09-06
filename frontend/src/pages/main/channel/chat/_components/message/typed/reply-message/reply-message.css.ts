import { classes, vars } from '@/features/theme';
import { style, styleVariants } from '@vanilla-extract/css';

export const container = style({
  width: '100%',
  height: 'fit-content',

  display: 'flex',
  flexDirection: 'column',
  justifyContent: 'flex-start',
  alignItems: 'stretch',
  gap: '5px',
});

export const replyContainer = style({
  position: 'relative',

  display: 'flex',
  flexDirection: 'column',
  justifyContent: 'center',
  alignItems: 'flex-start',
  gap: '4px',

  padding: '3px 6px 3px 12px',
  margin: 0,
  borderRadius: vars.radius.small,
  cursor: 'pointer',

  transition: `background-color ${vars.easing.background}`,

  selectors: {
    '&:hover': {
      backgroundColor: vars.color.terminal.surfaceHover,
    },
    '&:active': {
      backgroundColor: vars.color.terminal.surfaceHover,
    },
  },
});
export const replyText = styleVariants({
  sender: [classes.typography.body, {
    fontWeight: 700,
    color: vars.color.terminal.accent,
  }],
  content: [classes.typography.body, {
    color: vars.color.terminal.foregroundMuted,
  }],
});

export const replyDivider = style({
  position: 'absolute',
  top: '3px',
  bottom: '3px',
  left: '3px',

  width: '2px',

  backgroundColor: vars.color.terminal.accent,
  borderRadius: vars.radius.full,
});
