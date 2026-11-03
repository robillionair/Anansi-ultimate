// LARP Mode Slice 1: the global Ctrl+Shift+L predicate. Pure, so unit tests
// can cover the full matrix without a browser. No existing Ctrl+Shift+L
// binding exists in the UI (the only keydown handler is canvas-scoped in
// GhostPrompt3D), so there is no conflict to resolve.

export interface HotkeyEventLike {
  ctrlKey: boolean;
  shiftKey: boolean;
  key: string;
}

export function matchesLarpHotkey(event: HotkeyEventLike): boolean {
  return event.ctrlKey && event.shiftKey && event.key.toLowerCase() === 'l';
}
