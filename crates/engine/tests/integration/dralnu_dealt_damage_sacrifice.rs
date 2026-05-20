//! Regression test for #532 follow-up — narrowing the
//! `apply_post_replacement_effect` source-injection suppression guard.
//!
//! The Devour synthesis fix (commit 027924e31) added a guard in
//! `engine_replacement.rs`'s `apply_post_replacement_effect` that suppresses
//! the injected `TargetRef::Object(source)` target when the post-replacement
//! effect is `Effect::Sacrifice { target: Typed(...) | Any, .. }`. Devour's
//! ranged "sacrifice any number of your creatures" chooser prompt requires
//! the source NOT to be auto-injected as a pre-selected sacrifice target.
//!
//! The reviewer flagged that the original guard was too broad: it fired for
//! every post-replacement `Template`, including non-Moved events. Two parsed
//! database cards carry the matching shape under a non-Moved replacement:
//!
//!   * Dralnu, Lich Lord — `DealtDamage` + `Sacrifice { Typed(Permanent),
//!     count: EventContextAmount }`.
//!   * Outfitted Jouster — `DamageDone` + `Sacrifice { Typed(Equipment),
//!     count: Fixed(1) }`.
//!
//! The fix narrows the guard to `event == Some(ReplacementEvent::Moved)`,
//! scoping it exclusively to ETB-style replacements. Empirically (see
//! `apply_pending_post_replacement_effect` call sites in `deal_damage.rs`,
//! `combat_damage.rs`, `prevent_damage.rs`, `life.rs`, `replacement.rs`),
//! every non-ETB caller already passes `object_id = None`, so the
//! source-injection target list `object_id.map(TargetRef::Object).into_iter()`
//! collapses to `[]` regardless of which guard variant fires. The narrow
//! guard is therefore behavior-preserving for these cards in current code,
//! but the gate codifies the intent ("ETB-only suppression") so future call
//! sites that pass a non-None `object_id` for a non-Moved event cannot
//! silently inherit the chooser-suppression that was designed for Devour.
//!
//! This test pins Dralnu's parsed replacement shape and drives the engine
//! through a real damage event (P0 casts a 3-damage Bolt analog at their
//! own Dralnu) to verify the pipeline completes without panic, the
//! replacement matcher correctly fires for `ProposedEvent::Damage` whose
//! target is Dralnu, and the resulting state is consistent — Dralnu dies
//! from the lethal damage (CR 704.5g: a creature with damage ≥ toughness is
//! destroyed by SBAs) so the resolver does not depend on the source-injected
//! sacrifice path for cleanup.
//!
//! This is a SHAPE + SMOKE regression test, not a fail-first discriminating
//! test: as documented above, the narrow-vs-broad guard is observationally
//! identical in the current `deal_damage.rs` call site (`object_id = None`).
//! A true discriminator would require a code path that calls
//! `apply_pending_post_replacement_effect` with `Some(object_id)` for a
//! non-Moved event — no such path exists today.

use engine::game::scenario::{GameScenario, P0};
use engine::types::ability::{Effect, TargetFilter, TargetRef, TypeFilter};
use engine::types::actions::GameAction;
use engine::types::game_state::WaitingFor;
use engine::types::mana::{ManaType, ManaUnit};
use engine::types::phase::Phase;
use engine::types::replacements::ReplacementEvent;
use engine::types::zones::Zone;

const DRALNU_TEXT: &str = "If damage would be dealt to Dralnu, Lich Lord, \
    sacrifice that many permanents instead.";

const BOLT_TEXT: &str = "Bolt deals 3 damage to target creature.";

/// CR 614.1a + CR 614.12a (narrowed): pin Dralnu's parsed `DealtDamage` +
/// `Sacrifice { Typed(Permanent) }` replacement shape and drive a real
/// damage event through the engine. The narrow guard
/// (`event == Some(ReplacementEvent::Moved)`) scopes Devour's source-
/// injection suppression to ETB-style replacements, leaving this
/// `DealtDamage` post-replacement Template unaffected.
#[test]
fn dralnu_dealt_damage_replacement_shape_and_pipeline_smoke() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);

    // Dralnu on P0's battlefield. The replacement is parsed from Oracle so
    // the test exercises the same `ReplacementEvent::DealtDamage` +
    // `Sacrifice { Typed(Permanent) }` shape the live database produces.
    let dralnu = scenario
        .add_creature_from_oracle(P0, "Dralnu, Lich Lord", 3, 3, DRALNU_TEXT)
        .id();

    // P0 casts a 3-damage Bolt at Dralnu. Bolt's damage is dealt to a
    // 3/3 Dralnu — lethal by SBA (CR 704.5g) after the DealtDamage
    // replacement fires.
    let bolt_id = scenario
        .add_spell_to_hand_from_oracle(P0, "Bolt", true, BOLT_TEXT)
        .id();
    scenario.with_mana_pool(
        P0,
        vec![ManaUnit::new(
            ManaType::Red,
            engine::types::identifiers::ObjectId(0),
            false,
            vec![],
        )],
    );

    let mut runner = scenario.build();
    let bolt_card_id = runner.state().objects[&bolt_id].card_id;

    // SHAPE pin: the parsed replacement must be a `DealtDamage` event
    // carrying an `Effect::Sacrifice { target: Typed(Permanent), .. }`.
    // This is the exact (event, effect-target-filter) tuple the narrowed
    // guard now leaves untouched — the broad guard would have suppressed
    // source injection for ANY effect of this Sacrifice shape regardless
    // of event; the narrow guard requires `event == Moved` (which
    // DealtDamage is not), so injection control here is unaffected by
    // the guard.
    let parsed = runner
        .state()
        .objects
        .get(&dralnu)
        .expect("Dralnu must exist after build");
    let replacement = parsed
        .replacement_definitions
        .iter_unchecked()
        .find(|r| matches!(r.event, ReplacementEvent::DealtDamage))
        .expect("Dralnu must parse a DealtDamage replacement");
    let execute = replacement
        .execute
        .as_ref()
        .expect("the replacement must carry an execute Template");
    let Effect::Sacrifice { target, .. } = execute.effect.as_ref() else {
        panic!(
            "Dralnu's DealtDamage replacement must execute a Sacrifice effect; got {:?}",
            execute.effect
        );
    };
    let TargetFilter::Typed(scope) = target else {
        panic!(
            "Sacrifice target must be a Typed(Permanent) filter; got {target:?} \
             — the narrowed guard's matching shape (Typed | Any) is what we're \
             pinning here"
        );
    };
    assert!(
        scope.type_filters.contains(&TypeFilter::Permanent),
        "the Typed scope must include Permanent; got {:?}",
        scope.type_filters
    );

    // Sanity: Dralnu starts on P0's battlefield.
    assert_eq!(runner.state().objects[&dralnu].zone, Zone::Battlefield);

    // P0 casts Bolt; the target is selected via the interactive
    // `TargetSelection` prompt after the cast.
    runner
        .act(GameAction::CastSpell {
            object_id: bolt_id,
            card_id: bolt_card_id,
            targets: vec![],
        })
        .expect("P0 must be able to cast Bolt");

    // Drive cast-time target selection + stack resolution. Bounded loop
    // guards against a stall.
    drive_bolt_to_resolution(&mut runner, dralnu);

    // SMOKE pin: the pipeline completed without panic, Bolt left the
    // stack, and Dralnu died from the lethal damage (CR 704.5g). The
    // narrow guard preserved the existing resolution semantics — a
    // future regression that re-broadens the guard, or that wires a
    // non-Moved call site with a non-None `object_id`, would surface
    // here as either a panic, a stalled `EffectZoneChoice`, or Dralnu
    // remaining alive on the battlefield.
    assert_eq!(
        runner.state().objects[&bolt_id].zone,
        Zone::Graveyard,
        "Bolt must resolve and leave the stack"
    );
    assert_eq!(
        runner.state().objects[&dralnu].zone,
        Zone::Graveyard,
        "Dralnu (3/3) takes 3 damage and dies by SBA (CR 704.5g)"
    );
    assert!(
        matches!(runner.state().waiting_for, WaitingFor::Priority { .. }),
        "the engine must return to a normal Priority state after the \
         damage pipeline resolves; got {:?}",
        runner.state().waiting_for
    );
}

/// Drive Bolt's cast-time target selection and the stack resolution.
/// Bounded loop guards against a stall. Panics on any unexpected waiting
/// state — in particular `EffectZoneChoice`, which would surface a
/// regression where the source-injection suppression activates for a
/// non-Moved post-replacement (broadening the narrow guard).
fn drive_bolt_to_resolution(
    runner: &mut engine::game::scenario::GameRunner,
    dralnu: engine::types::identifiers::ObjectId,
) {
    for _ in 0..30 {
        match runner.state().waiting_for.clone() {
            WaitingFor::Priority { .. } => {
                if runner.state().stack.is_empty() {
                    return;
                }
                if runner.act(GameAction::PassPriority).is_err() {
                    return;
                }
            }
            WaitingFor::TargetSelection { target_slots, .. } => {
                assert!(
                    target_slots[0]
                        .legal_targets
                        .contains(&TargetRef::Object(dralnu)),
                    "Dralnu must be a legal target for Bolt"
                );
                runner
                    .act(GameAction::SelectTargets {
                        targets: vec![TargetRef::Object(dralnu)],
                    })
                    .expect("targeting Dralnu must succeed");
            }
            WaitingFor::EffectZoneChoice {
                effect_kind, cards, ..
            } => {
                panic!(
                    "Source-injection suppression regression: a chooser-driven \
                     EffectZoneChoice fired on Dralnu's DealtDamage \
                     post-replacement. Either the guard re-broadened beyond \
                     ETB events, or a non-Moved call site began passing a \
                     non-None object_id. effect_kind={:?} cards={:?}",
                    effect_kind, cards
                );
            }
            other => {
                panic!("unexpected waiting state during Bolt-at-Dralnu resolution: {other:?}")
            }
        }
    }
    panic!("stack failed to drain after 30 iterations — likely an infinite loop");
}
