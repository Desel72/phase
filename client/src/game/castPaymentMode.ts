import type { CastPaymentMode, GameAction } from "../adapter/types";
import { usePreferencesStore } from "../stores/preferencesStore";

const MANUAL_CAST_PAYMENT_MODE: CastPaymentMode = { type: "Manual" };

export function applySpellPaymentPreference(action: GameAction): GameAction {
  if (usePreferencesStore.getState().spellPaymentMode !== "manual") return action;

  switch (action.type) {
    case "CastSpell":
      return {
        type: "CastSpellWithPaymentMode",
        data: { ...action.data, payment_mode: MANUAL_CAST_PAYMENT_MODE },
      };
    case "CastSpellForFree":
      return {
        type: "CastSpellForFreeWithPaymentMode",
        data: { ...action.data, payment_mode: MANUAL_CAST_PAYMENT_MODE },
      };
    case "CastSpellAsMiracle":
      return {
        type: "CastSpellAsMiracleWithPaymentMode",
        data: { ...action.data, payment_mode: MANUAL_CAST_PAYMENT_MODE },
      };
    case "CastSpellAsMadness":
      return {
        type: "CastSpellAsMadnessWithPaymentMode",
        data: { ...action.data, payment_mode: MANUAL_CAST_PAYMENT_MODE },
      };
    case "CastSpellAsSneak":
      return {
        type: "CastSpellAsSneakWithPaymentMode",
        data: { ...action.data, payment_mode: MANUAL_CAST_PAYMENT_MODE },
      };
    case "CastSpellAsWebSlinging":
      return {
        type: "CastSpellAsWebSlingingWithPaymentMode",
        data: { ...action.data, payment_mode: MANUAL_CAST_PAYMENT_MODE },
      };
    default:
      return action;
  }
}
