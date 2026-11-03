// LARP Mode Slice 1: global toggle store + disposal registry.
// useSyncExternalStore over module state (no new dependency). Toggling OFF
// runs every registered disposer before subscribers are notified, so by
// the time React unmounts the HUD the scheduler is stopped and the
// listeners are already gone. Each disposer runs even if an earlier one
// throws, and the registry is cleared either way.

import { useSyncExternalStore } from 'react';

type Listener = () => void;
type Disposer = () => void;

let on = false;
const listeners = new Set<Listener>();
const disposers = new Set<Disposer>();

function getSnapshot(): boolean {
  return on;
}

function subscribe(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function emit(): void {
  for (const listener of [...listeners]) listener();
}

export function useLarpModeStore(): boolean {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}

export function getLarpMode(): boolean {
  return on;
}

export function setLarpMode(next: boolean): void {
  if (on === next) return;
  on = next;
  if (!next) disposeAll();
  emit();
}

export function toggleLarpMode(): void {
  setLarpMode(!on);
}

export function registerDisposer(disposer: Disposer): () => void {
  disposers.add(disposer);
  return () => {
    disposers.delete(disposer);
  };
}

export function disposeAll(): void {
  const pending = [...disposers];
  disposers.clear();
  for (const disposer of pending) {
    try {
      disposer();
    } catch {
      // One failing disposer must not keep the rest alive.
    }
  }
}

export function disposerCount(): number {
  return disposers.size;
}
