import { describe, expect, it } from "vitest";
import { createEmptyRegistry, TriggerRegistry, type MiniAppResult } from "../src/core/miniapps/registry";

describe("TriggerRegistry (M0 skeleton)", () => {
  it("starts empty", () => {
    const registry = createEmptyRegistry();
    expect(registry.triggers()).toEqual([]);
  });

  it("registers, matches (longest first) and strips the trigger", async () => {
    const registry = new TriggerRegistry();
    const calls: string[] = [];
    registry.register({
      trigger: "翻",
      handler: (ctx): MiniAppResult => {
        calls.push(`short:${ctx.input}`);
        return { handled: true };
      },
    });
    registry.register({
      trigger: "翻译",
      handler: (ctx): MiniAppResult => {
        calls.push(`long:${ctx.input}`);
        return { handled: true };
      },
    });
    expect(registry.triggers().sort()).toEqual(["翻", "翻译"]);
    const match = registry.match("  翻译 hello");
    expect(match?.trigger).toBe("翻译");
    expect(match?.input).toBe("hello");
    const result = await registry.dispatch("  翻译 hello");
    expect(result.handled).toBe(true);
    expect(calls).toEqual(["long:hello"]);
  });

  it("returns handled:false when no trigger matches", async () => {
    const registry = createEmptyRegistry();
    expect(registry.match("hello")).toBeNull();
    expect(await registry.dispatch("hello")).toEqual({ handled: false });
  });

  it("unregisters and overwrites by trigger name", async () => {
    const registry = new TriggerRegistry();
    registry.register({ trigger: "x", handler: () => ({ handled: false }) });
    expect(registry.unregister("x")).toBe(true);
    expect(registry.unregister("x")).toBe(false);
    registry.register({ trigger: "x", handler: () => ({ handled: true }) });
    registry.register({ trigger: "x", handler: () => ({ handled: false }) });
    expect(await registry.dispatch("x test")).toEqual({ handled: false });
  });

  it("skips wholeString apps in trigger matching (dedicated predicate point in scheduler)", async () => {
    const registry = new TriggerRegistry();
    registry.register({
      trigger: "2+2",
      wholeString: true,
      handler: () => ({ handled: true }),
    });
    expect(registry.match("2+2")).toBeNull();
    expect(await registry.dispatch("2+2")).toEqual({ handled: false });
  });
});
