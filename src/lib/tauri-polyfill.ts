/**
 * Tauri-specific API polyfills for web version
 * 
 * These provide stub implementations for Tauri APIs that don't exist in web mode.
 * Import from this module instead of @tauri-apps/api/window for web compatibility.
 */

// Type for the window stub
interface WindowStub {
  destroy: () => Promise<void>;
  close: () => Promise<void>;
  minimize: () => Promise<void>;
  maximize: () => Promise<void>;
  unmaximize: () => Promise<void>;
  isMaximized: () => Promise<boolean>;
  setTitle: (title: string) => Promise<void>;
  setSize: (size: { width: number; height: number }) => Promise<void>;
  setMinSize: (size: { width: number; height: number }) => Promise<void>;
  setMaxSize: (size: { width: number; height: number }) => Promise<void>;
  setResizable: (resizable: boolean) => Promise<void>;
  setAlwaysOnTop: (alwaysOnTop: boolean) => Promise<void>;
  setFocus: () => Promise<void>;
  onCloseRequested: (callback: (event: { preventDefault: () => void }) => void) => Promise<() => void>;
  onMoved: (callback: (event: { payload: { x: number; y: number } }) => void) => Promise<() => void>;
  onResized: (callback: (event: { payload: { width: number; height: number } }) => void) => Promise<() => void>;
}

// Get current window - stub for web mode
export async function getCurrentWindow(): Promise<WindowStub> {
  // Web mode: return a stub object
  return {
    destroy: async () => {
      // In web mode, reload to home
      window.location.href = '/';
    },
    close: async () => {
      window.location.href = '/';
    },
    minimize: async () => {
      // Cannot minimize in web, do nothing
    },
    maximize: async () => {
      // Cannot maximize in web, do nothing
    },
    unmaximize: async () => {
      // Cannot unmaximize in web, do nothing
    },
    isMaximized: async () => false,
    setTitle: async (_title: string) => {
      // Cannot set title in web, do nothing
    },
    setSize: async () => {
      // Cannot set size in web, do nothing
    },
    setMinSize: async () => {
      // Cannot set min size in web, do nothing
    },
    setMaxSize: async () => {
      // Cannot set max size in web, do nothing
    },
    setResizable: async () => {
      // Cannot set resizable in web, do nothing
    },
    setAlwaysOnTop: async () => {
      // Cannot set always on top in web, do nothing
    },
    setFocus: async () => {
      // Focus the window
      window.focus();
    },
    onCloseRequested: async (callback: (event: { preventDefault: () => void }) => void) => {
      // In web mode, listen to beforeunload
      const handler = (e: BeforeUnloadEvent) => {
        e.preventDefault();
        callback({ preventDefault: () => {} });
      };
      window.addEventListener('beforeunload', handler);
      // Return unlisten function
      return () => {
        window.removeEventListener('beforeunload', handler);
      };
    },
    onMoved: async () => {
      // Cannot detect move in web, return no-op
      return () => {};
    },
    onResized: async () => {
      // Cannot detect resize in web, return no-op
      return () => {};
    },
  };
}
