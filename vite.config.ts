import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "path";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

function normalizeModuleId(id: string): string {
  return id.replaceAll("\\", "/");
}

function isNodeModulePackage(id: string, packageName: string): boolean {
  const normalizedId = normalizeModuleId(id);
  const packagePath = `/node_modules/${packageName}`;

  return (
    normalizedId.includes(`${packagePath}/`) ||
    normalizedId.endsWith(packagePath)
  );
}

function matchesAnyPackage(id: string, packageNames: string[]): boolean {
  return packageNames.some((packageName) => isNodeModulePackage(id, packageName));
}

function isAppModule(id: string, modulePath: string): boolean {
  return normalizeModuleId(id).endsWith(modulePath);
}

function matchesAnyAppModule(id: string, modulePaths: string[]): boolean {
  return modulePaths.some((modulePath) => isAppModule(id, modulePath));
}

function manualChunks(id: string): string | undefined {
  if (matchesAnyAppModule(id, ["/src/lib/countries.ts"])) {
    return "countries";
  }

  if (id.indexOf("node_modules") === -1) {
    return undefined;
  }

  if (matchesAnyPackage(id, ["i18n-iso-countries"])) {
    return "countries";
  }

  if (matchesAnyPackage(id, ["react-router", "react-router-dom"])) {
    return "router";
  }

  if (
    matchesAnyPackage(id, [
      "i18next",
      "react-i18next",
      "i18next-resources-to-backend",
    ])
  ) {
    return "i18n";
  }

  if (isNodeModulePackage(id, "lucide-react")) {
    return "icons";
  }

  if (matchesAnyPackage(id, ["@tauri-apps/api", "@tauri-apps/plugin-opener"])) {
    return "tauri";
  }

  if (matchesAnyPackage(id, ["react", "react-dom", "scheduler"])) {
    return "react-vendor";
  }

  return undefined;
}

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => {
  // Load env variables for the current mode
  const env = loadEnv(mode, process.cwd(), '');
  const isWebMode = mode === 'web';

  return {
    plugins: [react(), tailwindcss()],
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
