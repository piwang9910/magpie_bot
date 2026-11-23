/**
 * TriggerRegistry（PLAN §3.6 / CONTEXT TriggerRegistry）：
 * 维护「触发词 → 处理器」映射，只做匹配、剥离、转发，不关心处理器内部实现。
 * 新增小程序只注册一条，不改调度器（ADR-5 框架先行）。
 *
 * M0 交付骨架：注册表结构、注册/注销/匹配能力齐备，但不注册任何真实处理器。
 * calculator 处理器在 M1 接入，translate 在 M4 接入。
 */

export interface MiniAppContext {
  /** 剥离触发词后的输入文本 */
  input: string;
  /** 原始完整输入 */
  raw: string;
}

export interface MiniAppResult {
  /** 是否消费了本次输入；false 表示让位给默认本地搜索 */
  handled: boolean;
  /** 结果卡片负载；M0 为空，由后续处理器填充 */
  payload?: unknown;
}

/** 小程序处理器统一契约（统一输入/输出，进程内实现）。 */
export type MiniAppHandler = (ctx: MiniAppContext) => MiniAppResult | Promise<MiniAppResult>;

export interface RegisteredApp {
  trigger: string;
  handler: MiniAppHandler;
  /** 是否走「整串可解析」判定（如计算器/号码），而非触发词 */
  wholeString?: boolean;
}

/** 触发词匹配结果：返回剥离后的文本与命中的处理器。 */
export interface TriggerMatch {
  app: RegisteredApp;
  input: string;
  trigger: string;
}

/**
 * TriggerRegistry：纯数据结构 + 纯函数方法，无副作用，便于单测。
 */
export class TriggerRegistry {
  private readonly apps = new Map<string, RegisteredApp>();

  /** 注册一个小程序；同名触发词会被覆盖。 */
  register(app: RegisteredApp): void {
    this.apps.set(app.trigger, app);
  }

  /** 注销一个小程序。 */
  unregister(trigger: string): boolean {
    return this.apps.delete(trigger);
  }

  /** 查询已注册触发词（供设置页/调试）。 */
  triggers(): string[] {
    return [...this.apps.keys()];
  }

  /**
   * 匹配输入开头的触发词；命中则剥离触发词并返回处理器。
   * 匹配规则：最长触发词优先，避免 `翻` 抢在 `翻译` 前面。
   */
  match(raw: string): TriggerMatch | null {
    const trimmed = raw.trimStart();
    const candidates = [...this.apps.values()]
      .filter((app) => !app.wholeString && trimmed.startsWith(app.trigger))
      .sort((a, b) => b.trigger.length - a.trigger.length);
    const app = candidates[0];
    if (!app) return null;
    return {
      app,
      trigger: app.trigger,
      input: trimmed.slice(app.trigger.length).trimStart(),
    };
  }

  /**
   * 转发执行：命中触发词则调用对应处理器；未命中返回 handled:false。
   * 这是调度器调用小程序的唯一入口。
   */
  async dispatch(raw: string): Promise<MiniAppResult> {
    const match = this.match(raw);
    if (!match) {
      return { handled: false };
    }
    return match.app.handler({ input: match.input, raw });
  }
}

/** 全局注册表单例：M0 不注册任何处理器（框架位，逐个实现）。 */
export function createEmptyRegistry(): TriggerRegistry {
  return new TriggerRegistry();
}
