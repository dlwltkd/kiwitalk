import { style } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  width: '100%',
  height: '100%',
  minHeight: 0,
  display: 'grid',
  gridTemplateRows: '30px minmax(0, 1fr) 24px',
  color: vars.color.terminal.foreground,
  background: vars.color.terminal.background,
});

export const header = style({
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: '12px',
  padding: '0 10px',
  color: vars.color.terminal.foregroundMuted,
  background: vars.color.terminal.surface,
  borderBottom: `1px solid ${vars.color.terminal.border}`,
  fontSize: '9px',
  textTransform: 'uppercase',

});

export const headerUser = style({
  color: vars.color.terminal.accentBright,
});

export const contentContainer = style({
  minWidth: 0,
  minHeight: 0,
  display: 'grid',
  gridTemplateColumns: 'minmax(220px, 1fr) minmax(320px, 440px)',
  alignItems: 'stretch',

  '@media': {
    'screen and (max-width: 720px)': {
      gridTemplateColumns: 'minmax(0, 1fr)',
      gridTemplateRows: 'auto minmax(0, 1fr)',
    },
  },
});

export const infoContainer = style({
  minWidth: 0,
  display: 'flex',
  flexDirection: 'column',
  padding: 'clamp(24px, 5vw, 72px)',
  background: vars.color.terminal.background,

  '@media': {
    'screen and (max-width: 720px)': {
      padding: '18px 20px 14px',
      borderBottom: `1px solid ${vars.color.terminal.border}`,
    },
  },
});

export const wordmark = style({
  color: vars.color.terminal.foregroundBright,
  fontSize: 'clamp(25px, 4vw, 48px)',
  fontWeight: 800,
  letterSpacing: '-0.08em',

});

export const wordmarkAccent = style({
  color: vars.color.terminal.accentBright,
});

export const description = style({
  marginTop: '12px',
  color: vars.color.terminal.foregroundMuted,
  fontSize: '10px',
  lineHeight: '17px',
  textTransform: 'uppercase',

  '@media': {
    'screen and (max-width: 720px)': {
      display: 'none',
    },
  },
});

export const panel = style({
  minWidth: 0,
  minHeight: 0,
  padding: 'clamp(24px, 5vw, 64px) clamp(20px, 4vw, 48px)',
  background: vars.color.terminal.surface,
  borderLeft: `1px solid ${vars.color.terminal.border}`,

  '@media': {
    'screen and (max-width: 720px)': {
      borderLeft: 0,
    },
  },
});

export const footer = style({
  display: 'flex',
  alignItems: 'center',
  padding: '0 10px',
  color: vars.color.terminal.foregroundMuted,
  background: vars.color.terminal.surface,
  borderTop: `1px solid ${vars.color.terminal.border}`,
  fontSize: '9px',
  textTransform: 'uppercase',
});
