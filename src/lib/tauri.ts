/**
 * Tauri API Compatibility Layer
 * 
 * This module re-exports the API functions for compatibility with existing code
 * that imports from "@tauri-apps/api/core".
 */

// Flag to indicate web mode (vs Tauri desktop mode)
export const isWeb = true;

// Re-export everything from api.ts under the tauri namespace
export { invoke, ping } from './api';
