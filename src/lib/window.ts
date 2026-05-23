/**
 * Web-compatible stub for @tauri-apps/api/window
 * 
 * Provides no-op implementations of Tauri window APIs for browser/web usage.
 */

/** Event-like object passed to close-requested handlers (stub). */
interface CloseRequestedEvent {
  preventDefault(): void;
}

type CloseHandler = (event: CloseRequestedEvent) => void | Promise<void>;

class WebWindow {
  async onCloseRequested(_handler: CloseHandler): Promise<() => void> {
    // In web mode there is no window-manager close event → never fire the handler.
    // Return a no-op unlisten function so the caller doesn't crash.
    return () => {};
  }

  async destroy(): Promise<void> {
    window.close();
  }
}

const currentWindow = new WebWindow();

export function getCurrentWindow(): WebWindow {
  return currentWindow;
}
