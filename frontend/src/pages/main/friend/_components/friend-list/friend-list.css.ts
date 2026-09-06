import { style } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  width: '100%',
  height: '100%',
  minHeight: 0,
  display: 'grid',
  gridTemplateRows: '52px 24px minmax(0, 1fr)',
  background: vars.color.terminal.surface,
});

export const header = style({
  display: 'flex',
  alignItems: 'center',
  padding: '0 10px',
  color: vars.color.terminal.foregroundBright,
  borderBottom: `1px solid ${vars.color.terminal.border}`,
  fontSize: '13px',
  fontWeight: 700,

});

export const prompt = style({
  color: vars.color.terminal.accentBright,
});

export const count = style({
  marginLeft: 'auto',
  color: vars.color.terminal.foregroundMuted,
  fontSize: '9px',
  fontWeight: 400,
});

export const columns = style({
  display: 'grid',
  gridTemplateColumns: '28px minmax(0, 1fr)',
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
});

export const section = style({
  padding: '8px 8px 3px',
  color: vars.color.terminal.foregroundMuted,
  fontSize: '8px',
  textTransform: 'uppercase',
});

export const empty = style({
  padding: '10px 8px',
  color: vars.color.terminal.foregroundMuted,
  fontSize: '9px',
  lineHeight: '14px',
});
