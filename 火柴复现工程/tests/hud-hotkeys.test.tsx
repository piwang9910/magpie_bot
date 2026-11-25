// @vitest-environment jsdom
/**
 * HUD 键盘行为组件级验证：
 *   - Tab / Ctrl+Tab  →  dispatch TOGGLE_AI  →  AI 态 ↔ 本地态互换
 *   - Esc 两步契约（各态一致）：
 *       有文本 → 第 1 次 Esc 清空文本（CLEAR），不退出；
 *       空文本 → Esc 直接调用 hideHud 收起 HUD（即清空后的第 2 次 Esc 退出）。
 *
 * 纯前端 DOM 事件验证，不注入系统级按键、不截屏。
 */
import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";

const hideHudMock = vi.fn();
vi.mock("../src/core/ipc/client", () => ({
  windows: {
    hideHud: () => hideHudMock(),
  },
}));

import { Hud } from "../src/hud/Hud";

// 让 React act() 在 jsdom 下生效并消除 "not configured to support act" 警告。
(globalThis as any).IS_REACT_ACT_ENVIRONMENT = true;

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  hideHudMock.mockClear();
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

function renderHud() {
  act(() => {
    root.render(<Hud />);
  });
}

function input(): HTMLInputElement {
  return container.querySelector("input") as HTMLInputElement;
}

function badgeText(): string | null {
  const el = container.querySelector(".hud-badge");
  return el ? el.textContent : null;
}

function press(el: HTMLElement, key: string, opts: KeyboardEventInit = {}) {
  act(() => {
    el.dispatchEvent(
      new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...opts }),
    );
  });
}

function setValue(el: HTMLInputElement, value: string) {
  act(() => {
    const proto = Object.getPrototypeOf(el) as object;
    const desc = Object.getOwnPropertyDescriptor(proto, "value");
    desc!.set!.call(el, value);
    el.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

describe("Hud 键盘接线：Tab 切 AI 态 / Esc 收起", () => {
  it("默认处于本地搜索态（badge = 搜）", () => {
    renderHud();
    expect(badgeText()).toBe("搜");
    expect(input().placeholder).toContain("输入搜索");
  });

  it("Tab 从本地态切入 AI 态，再按 Tab 切回本地", () => {
    renderHud();
    press(input(), "Tab");
    expect(badgeText()).toBe("AI");
    expect(input().placeholder).toContain("Tab 切回本地");

    press(input(), "Tab");
    expect(badgeText()).toBe("搜");
  });

  it("Ctrl+Tab 同样切入 AI 态", () => {
    renderHud();
    press(input(), "Tab", { ctrlKey: true });
    expect(badgeText()).toBe("AI");
  });

  it("AI 态下输入文本仍保持 AI 态（不回退本地）", () => {
    renderHud();
    press(input(), "Tab");
    expect(badgeText()).toBe("AI");
    setValue(input(), "今天天气怎么样");
    expect(badgeText()).toBe("AI");
    expect(input().value).toBe("今天天气怎么样");
  });

  it("空输入时按 Esc 调用 hideHud 收起窗口", () => {
    renderHud();
    expect(input().value).toBe("");
    press(input(), "Escape");
    expect(hideHudMock).toHaveBeenCalledTimes(1);
  });

  it("有输入时按 Esc 清空文本且不隐藏窗口", () => {
    renderHud();
    setValue(input(), "hello");
    expect(badgeText()).toBe("搜");

    press(input(), "Escape");
    expect(hideHudMock).toHaveBeenCalledTimes(0);
    expect(input().value).toBe("");
    expect(badgeText()).toBe("搜");
  });

  it("AI 态有文本：第 1 次 Esc 清空文本，第 2 次 Esc 退出 HUD", () => {
    renderHud();
    press(input(), "Tab"); // 切入 AI 态
    setValue(input(), "今天天气怎么样");
    expect(badgeText()).toBe("AI");

    // 第 1 次 Esc：清空文本，不退出
    press(input(), "Escape");
    expect(hideHudMock).toHaveBeenCalledTimes(0);
    expect(input().value).toBe("");

    // 第 2 次 Esc：文本已空 → 收起 HUD
    press(input(), "Escape");
    expect(hideHudMock).toHaveBeenCalledTimes(1);
  });

  it("AI 态空文本：第 1 次 Esc 直接退出 HUD", () => {
    renderHud();
    press(input(), "Tab"); // 切入 AI 态，文本仍为空
    expect(badgeText()).toBe("AI");
    expect(input().value).toBe("");

    press(input(), "Escape");
    expect(hideHudMock).toHaveBeenCalledTimes(1);
  });
});
