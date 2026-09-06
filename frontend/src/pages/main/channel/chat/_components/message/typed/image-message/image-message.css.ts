import { vars } from '@/features/theme';
import { createVar, style } from '@vanilla-extract/css';

export const container = style({
  width: 'min(100%, 420px)',
  maxHeight: '360px',
  display: 'grid',
  gridTemplateColumns: 'repeat(6, minmax(min(40px, 100%), 1fr))',
  gridAutoRows: 'auto',
  gap: '3px',

  borderRadius: vars.radius.small,
  border: `1px solid ${vars.color.terminal.border}`,
  overflow: 'hidden',

  selectors: {
    '&:has(*:only-of-type)': {
      gridTemplateColumns: 'repeat(2, minmax(min(40px, 100%), 1fr))',
    },
  },
});

export const preservedWidth = createVar();
export const preservedHeight = createVar();
export const image = style({
  vars: {
    [preservedWidth]: '0',
    [preservedHeight]: '0',
  },

  width: '100%',
  height: '100%',
  minWidth: preservedWidth,
  minHeight: preservedHeight,

  objectFit: 'cover',
  maxHeight: '360px',
  gridColumn: 'span 2',

  zIndex: vars.layer.base,
  overflow: 'hidden',

  selectors: {
    '[data-grid-type="1"] > &:nth-last-child(1)': {
      gridColumn: 'span 3',
    },
    '[data-grid-type="1"] > &:nth-last-child(2)': {
      gridColumn: 'span 3',
    },
    '[data-grid-type="1"] > &:nth-last-child(3)': {
      gridColumn: 'span 3',
    },
    '[data-grid-type="1"] > &:nth-last-child(4)': {
      gridColumn: 'span 3',
    },

    '[data-grid-type="2"] > &:nth-last-child(1)': {
      gridColumn: 'span 3',
    },
    '[data-grid-type="2"] > &:nth-last-child(2)': {
      gridColumn: 'span 3',
    },
  },
});
