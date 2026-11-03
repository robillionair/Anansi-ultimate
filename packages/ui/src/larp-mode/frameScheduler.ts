// LARP Mode Slice 1: frame scheduler. 60 FPS while the user is
// interacting, 24 FPS after 5 s without interaction, 0 FPS while the
// document is hidden. Injectable clock/RAF so unit tests can drive it
// without a browser. The loop never re-schedules after stop() — the
// stopped flag is checked before every re-schedule, which is what makes
// the cold-unmount zero-overhead claim testable.

export interface FrameSchedulerDeps {
  requestFrame: (callback: (time: number) => void) => number;
  cancelFrame: (handle: number) => void;
  now: () => number;
}

export interface FrameScheduler {
  start: () => void;
  stop: () => void;
  notifyInteraction: () => void;
  setHidden: (hidden: boolean) => void;
  isRunning: () => boolean;
  /** Current target rate: 0 while hidden, 24 while idle, else 60. */
  fps: () => number;
}

const IDLE_AFTER_MS = 5000;
const IDLE_FPS = 24;

export function createFrameScheduler(
  deps: FrameSchedulerDeps,
  onTick: (time: number) => void,
): FrameScheduler {
  let running = false;
  let hidden = false;
  let pending = false;
  let handle = 0;
  let lastInteraction = 0;
  let lastTick = 0;

  function schedule(): void {
    if (running && !hidden && !pending) {
      pending = true;
      handle = deps.requestFrame(tick);
    }
  }

  function tick(time: number): void {
    pending = false;
    if (!running || hidden) return;
    const idle = time - lastInteraction > IDLE_AFTER_MS;
    const interval = idle ? 1000 / IDLE_FPS : 0;
    if (interval === 0 || time - lastTick >= interval) {
      lastTick = time;
      onTick(time);
    }
    schedule();
  }

  return {
    start(): void {
      if (running) return;
      running = true;
      lastInteraction = deps.now();
      lastTick = 0;
      schedule();
    },
    stop(): void {
      running = false;
      if (pending) {
        deps.cancelFrame(handle);
        pending = false;
      }
    },
    notifyInteraction(): void {
      lastInteraction = deps.now();
    },
    setHidden(next: boolean): void {
      if (hidden === next) return;
      hidden = next;
      if (hidden) {
        if (pending) {
          deps.cancelFrame(handle);
          pending = false;
        }
      } else {
        schedule();
      }
    },
    isRunning(): boolean {
      return running;
    },
    fps(): number {
      if (hidden) return 0;
      return deps.now() - lastInteraction > IDLE_AFTER_MS ? IDLE_FPS : 60;
    },
  };
}
