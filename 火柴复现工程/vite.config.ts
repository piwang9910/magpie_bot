/// <reference types="node" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

// 两个 HTML 入口：HUD（index.html）与 Panel（panel.html）。
// Tauri 窗口分别指向这两个入口；它们是两个独立的 JS 上下文，不共享前端状态。
const rootDir = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig({
  plugins: [react()],
  // 与 tauri.conf.json 的 devUrl 对齐：Tauri dev 只认 1420，端口被占用时快速失败而不是静默换端口。
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    rollupOptions: {
      input: {
        hud: resolve(rootDir, "index.html"),
        panel: resolve(rootDir, "panel.html"),
      },
    },
  },
});
