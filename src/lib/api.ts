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
  console.log(`[API] invoke: ${cmd}`, args);
  
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
      errorMsg = errorJson.error || errorJson.message || errorMsg;
    } catch {
      // If the response text is not empty, use it as the error message
      if (responseText.length > 0) {
        errorMsg = responseText;
      }
    }
    throw new Error(errorMsg);
  }

  // Parse response JSON
  if (responseText.length === 0) {
    return undefined as T;
  }

  try {
    return JSON.parse(responseText) as T;
  } catch (e) {
    // If the response is not JSON, return it as a string
    return responseText as unknown as T;
  }
}
