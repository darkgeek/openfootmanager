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

  // Read response as text first to handle errors properly
  const responseText = await response.text();
  
  if (!response.ok) {
    // Try to parse error as JSON, otherwise use raw text
    let errorMsg = `HTTP ${response.status}: ${response.statusText}`;
    try {
      const errorJson = JSON.parse(responseText);
      errorMsg = errorJson.error || errorJson.message || responseText;
    } catch {
      // If it's HTML (starts with <), extract just the error message
      if (responseText.trim().startsWith('<')) {
        console.error("[API Error] Server returned HTML instead of JSON:", responseText.slice(0, 500));
        errorMsg = `Server error (${response.status}). Check console for details.`;
      } else {
        errorMsg = responseText || errorMsg;
      }
    }
    throw new Error(errorMsg);
  }

  // Parse successful JSON response
  if (!responseText.trim()) {
    return {} as T;
  }
  
  try {
    return JSON.parse(responseText) as T;
  } catch (e) {
    console.error("[API] Failed to parse response as JSON:", responseText.slice(0, 500));
    throw new Error(`Invalid JSON response from ${cmd}`);
  }
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


