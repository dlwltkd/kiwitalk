import { style } from '@vanilla-extract/css';

export const emoticon = style({
  width: 'min(180px, 45vw)',
  maxHeight: '180px',
  objectFit: 'contain',

  userSelect: 'none',
  WebkitUserDrag: 'none',
});
