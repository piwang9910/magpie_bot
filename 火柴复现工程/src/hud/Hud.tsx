import { useReducer, useRef, useEffect } from "react";
import {
  EMPTY_STATE,
  schedulerReducer,
  type SchedulerEvent,
  type SchedulerPredicates,
} from "../core/scheduler/state";
import * as ipc from "../core/ipc/client";
import "./Hud.css";

/**
 * HUD 悬浮框（M0 骨架）：输入框 + 状态标识 + 启动器网格空态。
 * M0 不接真实功能，只验证「骨架编译、状态机接线、可唤出、键盘不炸」。
 * M1 用计算器做第一条竖切时在此接入结果卡片与两态展开动效。
 */
export function Hud() {
  const inputRef = useRef<HTMLInputElement>(null);

  // M0 谓词全为 false；M1 注入计算器表达式判定。
  const predicates: SchedulerPredicates = {
    isCalcExpression: () => false,
    isPhoneNumber: () => false,
  };

  // reducer 是纯函数，判定点通过闭包注入（M0 全 false，M1 换成真实判定）。
  const [state, rawDispatch] = useReducer(
    (s: typeof EMPTY_STATE, e: SchedulerEvent) => schedulerReducer(s, e, predicates),
    EMPTY_STATE,
  );
  const dispatch = rawDispatch;

  // 窗口被 show 后自动聚焦输入框（热键回调唤起 → 立即可输入）。
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const onInput = (text: string) => {
    dispatch({ type: "INPUT", text, triggers: ["fy", "翻译"] } satisfies SchedulerEvent);
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Escape") {
      if (state.text === "") {
        void ipc.windows.hideHud();
      } else {
        dispatch({ type: "CLEAR" });
      }
    } else if (e.key === "Tab" || (e.ctrlKey && e.key === "Tab")) {
      e.preventDefault();
      dispatch({ type: "TOGGLE_AI" });
    }
  };

  return (
    <div className="hud" data-mode={state.kind}>
      <div className="hud-input-row">
        <span className="hud-badge" aria-hidden="true">
          {state.kind === "ai" ? "AI" : state.kind === "translate" ? "译" : "搜"}
        </span>
        <input
          ref={inputRef}
          className="hud-input"
          value={state.text}
          onChange={(e) => onInput(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={state.kind === "ai" ? "Enter 发送 · Tab 切回本地" : "输入搜索或输入内容…"}
          aria-label="喜鹊助手输入框"
          autoFocus
        />
      </div>
      {state.kind === "empty" ? (
        <div className="hud-empty">
          <span>骨架就绪 · 启动器网格将在 M1 接入</span>
        </div>
      ) : (
        <div className="hud-hint">调度器态：{state.kind}</div>
      )}
    </div>
  );
}
