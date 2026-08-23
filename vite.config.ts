import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @ts-expect-error process is a nodejs global
const env = process.env;
const host = env.TAURI_DEV_HOST;
const port = Number(env.VITE_PORT ?? (env.CI === "true" ? 1420 : 1422));

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. Tauri and CI expect a fixed port, fail if that port is not available
  server: {
    port,
    strictPort: true,
    host: host || false,
    allowedHosts: env.VITE_ALLOWED_HOSTS
      ? env.VITE_ALLOWED_HOSTS.split(",").map((value) => value.trim()).filter(Boolean)
      : undefined,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: port + 1,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
