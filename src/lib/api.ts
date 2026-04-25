/**
 * API Client for Web Version
 * 
 * This module provides an HTTP-based API client that mimics Tauri IPC invoke.
 * It can be used in place of @tauri-apps/api/core for web/browser deployments.
 * 
 * This file is used as a vite alias target for web mode builds.
 */

const API_BASE = ''; // Uses same-origin with proxy, or set for direct connection

/**
 * Make an API call to the backend server.
 * This mimics the Tauri `invoke` function signature.
 * 
 * @param cmd - The command name (e.g., "start_new_game")
 * @param args - Optional parameters to pass to the command
 * @returns The response data
 */
export async function invoke<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<T> {
  const url = `${API_BASE}/api/${cmd}`;
  
  const response = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: args ? JSON.stringify(args) : JSON.stringify({}),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(errorText || `HTTP ${response.status}: ${response.statusText}`);
  }

  const data = await response.json();
  
  return data as T;
}

/**
 * Check if the API server is available.
 */
export async function ping(): Promise<boolean> {
  try {
    await fetch(`${API_BASE}/api/get_active_game`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({}),
    });
    return true;
  } catch {
    return false;
  }
}


