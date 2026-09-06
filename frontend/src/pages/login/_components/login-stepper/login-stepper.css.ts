import { style, styleVariants } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  maxWidth: '420px',
  display: 'flex',
  flexDirection: 'column',
  gap: '12px',
  marginTop: 'auto',
});

export const label = style({
  color: vars.color.terminal.foregroundMuted,
  fontSize: '9px',
  textTransform: 'uppercase',
});

export const steps = style({
  display: 'flex',
  flexDirection: 'column',
  gap: '3px',
});

const stepBase = style({
  display: 'grid',
  gridTemplateColumns: '28px minmax(0, 1fr)',
  alignItems: 'center',
  gap: '8px',
  minHeight: '28px',
  padding: '4px 7px',
  borderLeft: '2px solid transparent',
  fontSize: '10px',
});

export const step = styleVariants({
  active: [stepBase, {
    color: vars.color.terminal.foregroundBright,
    background: vars.color.terminal.selection,
    borderLeftColor: vars.color.terminal.accentBright,
  }],
  complete: [stepBase, {
    color: vars.color.terminal.accent,
  }],
  pending: [stepBase, {
    color: vars.color.terminal.borderStrong,
  }],
});

export const back = style({
  alignSelf: 'flex-start',
});
