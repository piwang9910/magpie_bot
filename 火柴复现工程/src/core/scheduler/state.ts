/**
 * 搜索调度器状态机（PLAN §3.6：五态状态机 + reducer，纯函数重点单测）。
 *
 * 五个状态对应「一个输入框、四种去向」加空态：
 *   empty     空输入 → 启动器网格
 *   local     本地搜索（默认，实时出结果）
 *   calc      计算器（M1 第一条竖切，整串可解析才算）
 *   translate 翻译（触发词唤起，Enter 触发）
 *   ai        AI 直答（Tab 切入，Enter 发送）
 *
 * M0 只交付空壳：状态定义、迁移规则与判定点齐备，但 calc/translate 的具体
 * 判定函数默认返回 false（实现排在 M1/M4）。调度器自身不做系统调用。
 */

export type SchedulerKind = "empty" | "local" | "calc" | "translate" | "ai";

export interface SchedulerState {
  kind: SchedulerKind;
  /** 当前输入文本（翻译态下已剥离触发词） */
  text: string;
  /** 命中的触发词（仅 translate / miniapp 态） */
  trigger?: string;
}

export const EMPTY_STATE: SchedulerState = { kind: "empty", text: "" };

/** 判定点：由外部注入，M0 默认全 false（框架先行，逻辑后置）。 */
export interface SchedulerPredicates {
  /** 整串可解析为表达式 → 计算器 */
  isCalcExpression: (text: string) => boolean;
  /** 整串可解析为号码 → 来电归属（P2 判定点，REQUIREMENTS §10.1） */
  isPhoneNumber: (text: string) => boolean;
}

export const NOOP_PREDICATES: SchedulerPredicates = {
  isCalcExpression: () => false,
  isPhoneNumber: () => false,
};

/** 翻译触发词默认值（PLAN §10 已定默认值）。 */
export const DEFAULT_TRANSLATE_TRIGGERS = ["fy", "翻译"];

export type SchedulerEvent =
  | { type: "INPUT"; text: string; triggers?: string[] }
  | { type: "TOGGLE_AI" } // Tab / Ctrl+Tab：AI 态 ↔ 本地态互换
  | { type: "CLEAR" } // Esc 清空文本；文本已空时由组件层调用 hideHud 收起 HUD
  | { type: "RESET" }; // 窗口隐藏后复位

/** 判定翻译触发词：返回剥离后的文本与触发词。 */
function matchTranslateTrigger(
  text: string,
  triggers: string[],
): { text: string; trigger: string } | null {
  const trimmed = text.trimStart();
  for (const trigger of triggers) {
    if (trimmed.startsWith(trigger)) {
      const rest = trimmed.slice(trigger.length).trimStart();
      return { text: rest, trigger };
    }
  }
  return null;
}

/**
 * 纯 reducer：输入 + 当前状态 → 新状态。
 * 判定优先级：空 → 翻译触发词 → 计算器 → 号码（P2 占位）→ 本地；AI 为独立显式态。
 */
export function schedulerReducer(
  state: SchedulerState,
  event: SchedulerEvent,
  predicates: SchedulerPredicates = NOOP_PREDICATES,
): SchedulerState {
  switch (event.type) {
    case "RESET":
      return EMPTY_STATE;

    case "CLEAR":
      return EMPTY_STATE;

    case "TOGGLE_AI":
      // Tab：本地态切 AI；AI 态切回本地。
      if (state.kind === "ai") {
        return { kind: "local", text: state.text };
      }
      return { kind: "ai", text: state.text };

    case "INPUT": {
      const text = event.text;
      if (text.trim() === "") {
        return EMPTY_STATE;
      }
      const triggers = event.triggers ?? DEFAULT_TRANSLATE_TRIGGERS;
      const translate = matchTranslateTrigger(text, triggers);
      if (translate) {
        return { kind: "translate", text: translate.text, trigger: translate.trigger };
      }
      if (predicates.isCalcExpression(text)) {
        return { kind: "calc", text };
      }
      if (predicates.isPhoneNumber(text)) {
        // P2 判定点已接通，但 v1 无处理器；落回本地，避免空卡片。
        return { kind: "local", text };
      }
      // AI 态下继续输入仍留在 AI（不发请求，按 Enter 才发送）。
      if (state.kind === "ai") {
        return { kind: "ai", text };
      }
      return { kind: "local", text };
    }
  }
}
