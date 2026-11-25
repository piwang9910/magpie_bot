/// <reference types="vitest" />
import { defineConfig } from "vitest/config";
import { fileURLToPath } from "node:url";

// 单测只覆盖纯逻辑（调度器状态机、TriggerRegistry 等），不启动 Tauri 运行时。
// 别名必须用 fileURLToPath：Windows 下 URL.pathname 会得到 "/D:/..." 形式，路径无法解析。
const srcDir = fileURLToPath(new URL("./src", import.meta.url));

export default defineConfig({
  resolve: {
    alias: {
      "@": srcDir,
    },
  },
  test: {
    environment: "node",
    include: ["tests/**/*.test.{ts,tsx}"],
  },
});
