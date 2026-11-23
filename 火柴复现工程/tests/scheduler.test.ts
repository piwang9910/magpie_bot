import { describe, expect, it } from "vitest";
import {
  DEFAULT_TRANSLATE_TRIGGERS,
  EMPTY_STATE,
  NOOP_PREDICATES,
  schedulerReducer,
  type SchedulerPredicates,
} from "../src/core/scheduler/state";

describe("schedulerReducer (M0 skeleton)", () => {
  it("starts empty and returns to empty on CLEAR/RESET", () => {
    expect(EMPTY_STATE).toEqual({ kind: "empty", text: "" });
    const typed = schedulerReducer(EMPTY_STATE, { type: "INPUT", text: "abc" });
    expect(typed.kind).toBe("local");
    expect(schedulerReducer(typed, { type: "CLEAR" })).toEqual(EMPTY_STATE);
    expect(schedulerReducer(typed, { type: "RESET" })).toEqual(EMPTY_STATE);
  });

  it("routes empty/whitespace input to empty state", () => {
    expect(schedulerReducer(EMPTY_STATE, { type: "INPUT", text: "   " }).kind).toBe("empty");
    expect(schedulerReducer(EMPTY_STATE, { type: "INPUT", text: "" }).kind).toBe("empty");
  });

  it("strips translate trigger and keeps the rest", () => {
    const state = schedulerReducer(EMPTY_STATE, {
      type: "INPUT",
      text: "fy hello",
      triggers: DEFAULT_TRANSLATE_TRIGGERS,
    });
    expect(state.kind).toBe("translate");
    expect(state.text).toBe("hello");
    expect(state.trigger).toBe("fy");
  });

  it("supports Chinese translate trigger", () => {
    const state = schedulerReducer(EMPTY_STATE, {
      type: "INPUT",
      text: "翻译 你好",
    });
    expect(state.kind).toBe("translate");
    expect(state.text).toBe("你好");
    expect(state.trigger).toBe("翻译");
  });

  it("does not treat a substring as a trigger", () => {
    const state = schedulerReducer(EMPTY_STATE, { type: "INPUT", text: "efy hello" });
    expect(state.kind).toBe("local");
    expect(state.text).toBe("efy hello");
  });

  it("toggles AI with TOGGLE_AI and returns via BACK_TO_LOCAL", () => {
    const ai = schedulerReducer(EMPTY_STATE, { type: "TOGGLE_AI" });
    expect(ai.kind).toBe("ai");
    const back = schedulerReducer(ai, { type: "BACK_TO_LOCAL" });
    expect(back.kind).toBe("local");
    expect(schedulerReducer(back, { type: "TOGGLE_AI" }).kind).toBe("ai");
  });

  it("keeps AI state while typing (Enter triggers request in later milestones)", () => {
    const ai = schedulerReducer(EMPTY_STATE, { type: "TOGGLE_AI" });
    const typed = schedulerReducer(ai, { type: "INPUT", text: "天气如何" });
    expect(typed.kind).toBe("ai");
    expect(typed.text).toBe("天气如何");
  });

  it("uses injected calc predicate; whole-string semantics", () => {
    const predicates: SchedulerPredicates = {
      ...NOOP_PREDICATES,
      isCalcExpression: (text) => text === "2+2",
    };
    expect(schedulerReducer(EMPTY_STATE, { type: "INPUT", text: "2+2" }, predicates).kind).toBe("calc");
    expect(
      schedulerReducer(EMPTY_STATE, { type: "INPUT", text: "2+2会议纪要" }, predicates).kind,
    ).toBe("local");
  });

  it("phone-number predicate is a placeholder that falls back to local in v1", () => {
    const predicates: SchedulerPredicates = {
      ...NOOP_PREDICATES,
      isPhoneNumber: (text) => /^\d{11}$/.test(text),
    };
    const state = schedulerReducer(EMPTY_STATE, { type: "INPUT", text: "13800138000" }, predicates);
    expect(state.kind).toBe("local");
  });
});
