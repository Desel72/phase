import { describe, expect, it } from "vitest";

import type { AttackTarget, GameState, ObjectId } from "../../adapter/types";
import {
  buildAttacks,
  getValidAttackTargetsForAttacker,
  selectedAttackersNeedTargetPicker,
} from "../combat";

function playerTarget(playerId: number): AttackTarget {
  return { type: "Player", data: playerId };
}

function declareAttackersState(
  validAttackTargetsByAttacker: Record<string, AttackTarget[]>,
): GameState {
  return {
    waiting_for: {
      type: "DeclareAttackers",
      data: {
        player: 0,
        valid_attacker_ids: [11, 12],
        valid_attack_targets: [playerTarget(1), playerTarget(2)],
        valid_attack_targets_by_attacker: validAttackTargetsByAttacker,
      },
    },
    players: [
      { id: 0, life: 20, poison_counters: 0, mana_pool: { mana: [] }, library: [], hand: [], graveyard: [], has_drawn_this_turn: false, lands_played_this_turn: 0, turns_taken: 0 },
      { id: 1, life: 20, poison_counters: 0, mana_pool: { mana: [] }, library: [], hand: [], graveyard: [], has_drawn_this_turn: false, lands_played_this_turn: 0, turns_taken: 0 },
      { id: 2, life: 20, poison_counters: 0, mana_pool: { mana: [] }, library: [], hand: [], graveyard: [], has_drawn_this_turn: false, lands_played_this_turn: 0, turns_taken: 0 },
    ],
    seat_order: [0, 1, 2],
  } as unknown as GameState;
}

describe("combat utils", () => {
  it("builds per-attacker defaults from engine legality instead of seat order", () => {
    const state = declareAttackersState({
      "11": [playerTarget(2)],
      "12": [playerTarget(1)],
    });

    expect(buildAttacks([11, 12], state, 0)).toEqual([
      [11, playerTarget(2)],
      [12, playerTarget(1)],
    ]);
  });

  it("does not force the target picker when each attacker has only one legal target", () => {
    const attackers: ObjectId[] = [11, 12];
    const state = declareAttackersState({
      "11": [playerTarget(2)],
      "12": [playerTarget(1)],
    });

    expect(selectedAttackersNeedTargetPicker(state, attackers)).toBe(false);
  });

  it("forces the target picker when an attacker has multiple legal targets", () => {
    const state = declareAttackersState({
      "11": [playerTarget(1), playerTarget(2)],
      "12": [playerTarget(1)],
    });

    expect(getValidAttackTargetsForAttacker(state, 11)).toEqual([
      playerTarget(1),
      playerTarget(2),
    ]);
    expect(selectedAttackersNeedTargetPicker(state, [11, 12])).toBe(true);
  });

  it("treats an attacker missing from the per-attacker map as having no legal targets", () => {
    const state = declareAttackersState({
      "12": [playerTarget(1)],
    });

    expect(getValidAttackTargetsForAttacker(state, 11)).toEqual([]);
  });
});
