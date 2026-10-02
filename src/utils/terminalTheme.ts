import type { ITheme } from 'xterm';

/** Read the shared console palette after the application's theme is applied. */
export function terminalTheme() {
  const styles = getComputedStyle(document.documentElement);
  const color = (name: string) => styles.getPropertyValue(`--terminal-${name}`).trim();
  return {
    background: color('bg'),
    foreground: color('fg'),
    cursor: color('cursor'),
    cursorAccent: color('bg'),
    selectionBackground: color('selection'),
    black: color('black'),
    red: color('red'),
    green: color('green'),
    yellow: color('yellow'),
    blue: color('blue'),
    magenta: color('magenta'),
    cyan: color('cyan'),
    white: color('white'),
    brightBlack: color('bright-black'),
    brightRed: color('bright-red'),
    brightGreen: color('bright-green'),
    brightYellow: color('bright-yellow'),
    brightBlue: color('bright-blue'),
    brightMagenta: color('bright-magenta'),
    brightCyan: color('bright-cyan'),
    brightWhite: color('bright-white'),
  } satisfies ITheme;
}
