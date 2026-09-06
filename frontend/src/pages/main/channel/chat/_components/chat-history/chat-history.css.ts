import { globalStyle, style } from '@vanilla-extract/css';

import { vars } from '@/features/theme';

export const container = style({
  display: 'grid',
  gridTemplateRows: 'auto minmax(0, 1fr)',
  minHeight: 0,
  minWidth: 0,
});

export const toolbar = style({
  display: 'flex',
  alignItems: 'center',
  flexWrap: 'wrap',
  gap: '6px',
  padding: '6px 12px',
  borderBottom: `1px solid ${vars.color.terminal.border}`,
  fontSize: '10px',
  color: vars.color.terminal.foregroundMuted,
});

globalStyle(`${toolbar} button[aria-pressed="true"]`, {
  background: vars.color.terminal.surfaceRaised,
  color: vars.color.terminal.foregroundBright,
});

export const saved = style({
  display: 'grid',
  gridTemplateRows: 'auto minmax(0, 1fr)',
  minHeight: 0,
  minWidth: 0,
});

export const source = style({
  display: 'flex',
  flexDirection: 'column',
  gap: '5px',
  padding: '8px 12px',
  fontSize: '10px',
  overflowWrap: 'anywhere',
  color: vars.color.terminal.foregroundMuted,
  borderBottom: `1px solid ${vars.color.terminal.border}`,
});

export const jumpButtons = style({ display: 'flex', gap: '8px' });

export const entry = style({
  listStyle: 'none',
  padding: '6px 14px 10px',
  borderBottom: `1px solid ${vars.color.terminal.border}`,
});

export const date = style({
  padding: '10px 0',
  textAlign: 'center',
  fontSize: '11px',
  color: vars.color.terminal.foregroundBright,
});

export const sender = style({
  display: 'flex',
  flexWrap: 'wrap',
  gap: '10px',
  marginBottom: '5px',
  color: vars.color.terminal.accent,
  fontSize: '10px',
});

export const content = style({
  whiteSpace: 'pre-wrap',
  overflowWrap: 'anywhere',
  fontSize: '12px',
  lineHeight: '1.6',
});

export const event = style({
  whiteSpace: 'pre-wrap',
  overflowWrap: 'anywhere',
  fontSize: '10px',
  color: vars.color.terminal.foregroundMuted,
});
