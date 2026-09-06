import { globalStyle } from '@vanilla-extract/css';

globalStyle('html, body, #root', {
  width: '100%',
  height: '100%',

  padding: 0,
  margin: 0,

  overflow: 'hidden',
});

globalStyle(`*:where(:not(canvas, iframe, img, svg, svg *, symbol *, video))`, {
  all: 'unset',
  display: 'revert',
});

globalStyle('*, *::before, *::after', {
  boxSizing: 'border-box',
  WebkitFontSmoothing: 'greyscale',
  MozOsxFontSmoothing: 'antialiased',
});

globalStyle('table', {
  borderCollapse: 'collapse',
  borderSpacing: 0,
});

globalStyle('canvas, img, picture, svg, video', {
  display: 'block',
  maxWidth: '100%',
});

globalStyle('a', {
  cursor: 'pointer',
});

globalStyle('button', {
  cursor: 'pointer',
  userSelect: 'none',
});

globalStyle(':focus-visible', {
  outline: '1px solid #2dd5b7',
  outlineOffset: '-1px',
});

globalStyle('::selection', {
  color: '#f7e8b2',
  background: '#32473b',
});

globalStyle('*', {
  scrollbarColor: '#53685b #0c1512',
  scrollbarWidth: 'thin',
});

globalStyle('*, *::before, *::after', {
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      animationDuration: '0.01ms !important',
      animationIterationCount: '1 !important',
      scrollBehavior: 'auto',
      transitionDuration: '0.01ms !important',
    },
  },
});

globalStyle('input::-ms-clear, input::-webkit-search-cancel-button', {
  display: 'none',
});

globalStyle('input::-webkit-inner-spin-button, input::-webkit-outer-spin-button', {
  WebkitAppearance: 'none',
});
