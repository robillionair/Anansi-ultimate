// LARP Mode Slice 1: the RGB Chrome toggle. Lives in the Workspace
// header next to the connection badge (the workspace status), so it is
// present on every view. Click toggles; the global Ctrl+Shift+L hotkey
// toggles from anywhere. The RGB border is a conic-gradient frame whose
// hue rotates while ON and sits static at low opacity while OFF.

import { useEffect } from 'react';
import { toggleLarpMode, useLarpModeStore } from './store.ts';
import { matchesLarpHotkey } from './hotkey.ts';

export function LarpToggle() {
  const on = useLarpModeStore();

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (matchesLarpHotkey(event)) {
        event.preventDefault();
        toggleLarpMode();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  return (
    <span className={'larp-toggle-frame' + (on ? ' larp-toggle-frame-on' : '')}>
      <button
        type="button"
        className={'larp-toggle' + (on ? ' larp-toggle-on' : ' larp-toggle-off')}
        aria-pressed={on}
        aria-label="Toggle LARP mode"
        title="Toggle LARP mode (Ctrl+Shift+L)"
        onClick={() => toggleLarpMode()}
      >
        <span className={'larp-toggle-led' + (on ? ' larp-toggle-led-on' : '')} aria-hidden="true" />
        <span className="larp-toggle-label">{on ? '[ LARP: ACTIVE ]' : '[ LARP: OFF ]'}</span>
      </button>
    </span>
  );
}
