import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

function manualChunks(id: string): string | undefined {
  if (id.indexOf("node_modules") === -1) {
    return undefined;
  }

  if (id.indexOf("react-router-dom") !== -1) {
    return "router";
  }

  if (
    id.indexOf("react") !== -1 ||
    id.indexOf("react-dom") !== -1 ||
    id.indexOf("scheduler") !== -1
  ) {
    return "react-vendor";
  }

  if (id.indexOf("@tauri-apps") !== -1) {
    return "tauri";
  }

  if (id.indexOf("i18next") !== -1) {
    return "i18n";
  }

  if (id.indexOf("lucide-react") !== -1) {
    return "icons";
  }

  return undefined;
}

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => {
  // Load env variables for the current mode
  const env = loadEnv(mode, process.cwd(), '');
  const isWebMode = mode === 'web';

  return {
    plugins: [react()],
    test: {
      environment: "jsdom",
      globals: true,
      include: ["src/**/*.test.{ts,tsx}"],
      setupFiles: ["src/test-setup.ts"],
      coverage: {
        exclude: ["src/i18n/locales/**", "src/**/*.test.{ts,tsx}", "src/test-setup.ts"],
      },
    },

    // Mode-specific configuration
    ...(isWebMode ? {
      // Web mode: serve from dist folder with API proxy
      server: {
        port: 5173,
        host: true, // Bind to all network interfaces for remote access
        proxy: {
          '/api': {
            target: 'http://localhost:3001',
            changeOrigin: true,
          },
        },
      },
      build: {
        outDir: 'dist-web',
        rollupOptions: {
          output: {
            manualChunks,
          },
        },
      },
    } : {
      // Tauri mode: default Vite options
      clearScreen: false,
      build: {
        rollupOptions: {
          output: {
            manualChunks,
          },
        },
      },
      server: {
        port: 1420,
        strictPort: true,
        host: host || false,
        hmr: host
          ? {
              protocol: "ws",
              host,
              port: 1421,
            }
          : undefined,
        watch: {
          ignored: ["**/src-tauri/**"],
        },
      },
    }),

    // Define environment variables for web mode
    define: {
      'import.meta.env.VITE_WEB_MODE': JSON.stringify(isWebMode ? 'true' : 'false'),
      'import.meta.env.VITE_API_BASE': JSON.stringify(env.VITE_API_BASE || ''),
    },

    // Resolve aliases for different modes
    resolve: {
      alias: {
        '@tauri-apps/api/core': isWebMode 
          ? path.resolve(__dirname, 'src/lib/api.ts')
          : '@tauri-apps/api/core',
      },
    },
  };
});
