import { useRef, useEffect } from "react";
import * as ipc from "../core/ipc/client";
import "./Panel.css";

/**
 * 会话面板（M0 骨架）：只服务 AI 问答与追问，不承载翻译/计算器/本地结果。
 * M0 空壳：展示初始问题占位 + Esc 关闭回焦；流式渲染在 M3 接入。
 * HUD 与 Panel 是两个独立 JS 上下文，不共享前端状态。
 */
export function Panel() {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    // Esc 关闭面板（v1 关闭即销毁会话，不落盘）。
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        void ipc.windows.hidePanel();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className="panel">
      <header className="panel-header">
        <strong>喜鹊助手 · AI 会话</strong>
        <span className="panel-esc">Esc 关闭</span>
      </header>
      <div className="panel-body">
        <div className="panel-placeholder">
          骨架就绪。AI 流式问答将在 M3 接入，本窗口仅服务 AI 问答与追问。
        </div>
      </div>
      <div className="panel-input-row">
        <input ref={inputRef} className="panel-input" placeholder="追问…（M3 接入）" aria-label="追问输入框" />
      </div>
    </div>
  );
}
