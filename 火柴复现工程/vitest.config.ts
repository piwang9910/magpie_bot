/// <reference types="vitest" />
import { defineConfig } from "vitest/config";

// 单测只覆盖纯逻辑（调度器状态机、TriggerRegistry 等），不启动 Tauri 运行时。
export default defineConfig({
  resolve: {
    alias: {
      "@": new URL("./src", import.meta.url).pathname,
    },
  },
  test: {
    environment: "node",
    include: ["tests/**/*.test.ts"],
  },
});
