/**
 * IPC 统一封装：前端所有 command 调用只经此处，不散落裸 invoke（PLAN §3.11 第 4 步）。
 * 底层命令与类型来自 tauri-specta 生成的 bindings.ts（由 `cargo test export_bindings`
 * 与 debug 构建生成，勿手改）。这里只做一次薄封装，便于将来统一加日志/错误映射。
 */

import { commands } from "./bindings";
import type { AppSettings, Query } from "./bindings";

export type { AppSettings, Query, SearchResult } from "./bindings";

export const greet = (name: string) => commands.greet(name);
export const getSettings = () => commands.getSettings();
export const saveSettings = (settings: AppSettings) => commands.saveSettings(settings);
export const searchPlaceholder = (query: Query) => commands.searchPlaceholder(query);
export const cancelGeneration = () => commands.cancelGeneration();

/** 窗口控制命令。 */
export const windows = {
  openPanel: () => commands.openPanel(),
  hideHud: () => commands.hideHud(),
  hidePanel: () => commands.hidePanel(),
  focusHud: () => commands.focusHud(),
};
