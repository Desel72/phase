# Bug Triage System — Operator Reference

## Quick Commands

```bash
# Full pipeline (fetch new Discord messages → extract → triage → render)
bun scripts/sync-bug-reports.ts fetch
bun scripts/sync-bug-reports.ts extract
bun scripts/sync-bug-reports.ts triage
bun scripts/sync-bug-reports.ts render

# Check a specific card's parser status
jq '.["card name"]' client/public/card-data.json
jq '.["card name"] | {abilities: [.abilities[]? | select(.effect.type == "Unimplemented")], triggers: [.triggers[]? | select(.mode == "Unknown")]}' client/public/card-data.json

# Regenerate card data (after parser changes)
./scripts/gen-card-data.sh

# Single card debug
cargo run --bin oracle-gen -- data --filter "card name"
```

## GitHub Issue Workflow

```bash
# List open issues by priority
gh issue list --repo phase-rs/phase --state open --label "priority:p0-softlock"
gh issue list --repo phase-rs/phase --state open --label "priority:p1-core-mechanic"

# Close a fixed parser-gap issue only after the reported ability is semantically represented
gh issue close <N> --repo phase-rs/phase --comment "Fixed in <commit>. The reported ability now parses to the expected typed semantics with no Unimplemented fallback."

# Transition issue status
gh issue edit <N> --repo phase-rs/phase --remove-label "status:confirmed" --add-label "status:fixed-unreleased"
gh issue edit <N> --repo phase-rs/phase --remove-label "status:fixed-unreleased" --add-label "status:needs-runtime-verify"

# After runtime verification passes
gh issue close <N> --repo phase-rs/phase --comment "Verified in gameplay. Closing."
gh issue edit <N> --repo phase-rs/phase --remove-label "status:needs-runtime-verify" --add-label "status:verified"
```

### Mandatory Post-Fix Review Gate

Every code fix made during bug triage must run the implementation review command before the fix is committed, marked fixed, or described as complete:

```bash
cat .claude/commands/review-impl.md
```

Then apply the review checklist in `.claude/commands/review-impl.md` to the uncommitted diff. This is a required regression gate, not an optional cleanup pass. The review must look for missing sibling coverage, overly broad parser/runtime semantics, weak tests, hidden state leaks, rules-correctness gaps, and card-specific fixes that should have been modeled as reusable building blocks.

If the review finds a gap, fix it immediately, rerun the relevant targeted tests, and run the review gate again. Do not transition GitHub issues to `fixed-unreleased`, `needs-runtime-verify`, `verified`, or closed until this review is clean.

### GitHub Comment Standard

GitHub comments must be concise, user-facing status updates. Do not paste local command output, long command transcripts, local machine paths, target directories, or exhaustive verification command lists into issues. Summarize the evidence at the semantic level instead:
- Good: "Fixed in <commit>. The reported ability now parses as a typed ProduceMana replacement with a tapped-for-mana scope, and regression tests cover both multiplied and non-multiplied mana production."
- Bad: "Verification: `CARGO_TARGET_DIR=... cargo test ...`, `cargo run ...`, `git diff --check`" followed by command details or output.

Keep raw command details in the local working notes or final Codex response when useful, not in GitHub. For issue updates, mention only the commit, the reported behavior now covered, and whether targeted parser/runtime evidence exists.

## Status Lifecycle

```
needs-triage → confirmed → in-progress → fixed-unreleased → needs-runtime-verify → verified → closed
                         → stale → closed
                         → wont-fix → closed
                         → duplicate → closed
```

## Resync Workflow (periodic maintenance)

Run this after parser/engine changes to update triage state:

### Step 1: Regenerate card data
```bash
./scripts/gen-card-data.sh
```

### Step 2: Re-run coverage cross-reference
Spawn a Sonnet agent to re-read `triage/llm-triage-items.jsonl` and cross-reference against the updated `client/public/card-data.json`. Write results to `triage/coverage-crossref.jsonl` and `triage/coverage-crossref-summary.md`.

### Step 3: Identify candidates for verification
Compare the new cross-reference against open GitHub issues. Parser coverage is only a candidate signal:
- If the bug was a parser gap → inspect the reported ability and verify the typed AST/IR represents the reported semantics. Close only after that targeted semantic check passes.
- If the bug was a runtime issue → do not mark fixed from parser coverage. Inspect the relevant runtime code and preferably add/run a reproduction test. Transition only after targeted evidence exists.

### Step 4: Fetch new Discord messages
```bash
bun scripts/sync-bug-reports.ts fetch
```
If new messages exist, re-run extract → triage → render and review new items.

### Step 5: Update dashboard
```bash
bun scripts/sync-bug-reports.ts render
```

## Oracle Text Sourcing — MANDATORY

**Every Oracle text reference in a GitHub issue, comment, or triage note MUST be copied verbatim from `client/public/card-data.json`.** Never quote Oracle text from memory, the user's Discord message, Scryfall, or training data. The card database is the only authoritative source — using anything else risks filing issues against the wrong card text and wasting fix cycles.

```bash
# REQUIRED before quoting Oracle text in any issue body or comment:
jq -r '.["card name"] | .oracle_text' client/public/card-data.json
```

If `oracle_text` is `null` or the card key is missing, do NOT guess — flag the card-data lookup failure in the issue and stop. A missing entry is itself a bug worth reporting (likely a card-data pipeline gap).

When filing or updating an issue, include an explicit **Oracle text (verified from `client/public/card-data.json`)** section quoting the text you looked up. This makes the verification visible to reviewers and prevents downstream agents from re-introducing wrong text.

If you discover an existing issue references wrong Oracle text, fix it as part of the next triage pass — wrong card text in an issue is worse than no quote, because it sends fixers chasing the wrong semantics.

## Investigating Whether a Bug Is Fixed

### Evidence Standard

User reports are presumed real unless there is strong contradictory evidence. Do not mark an issue `likely_fixed`, `fixed-unreleased`, `verified`, or closed from parser coverage alone.

`fully_parsed` only means the parser did not emit `Unimplemented` or `Unknown`. It does not prove the card behaves correctly: text can be swallowed, parsed into overly generic effects, attached to the wrong subject/controller/zone, or represented with the wrong typed semantics.

Acceptable evidence depends on the report type:
- Parser-gap report: the specific reported Oracle clause parses into the expected typed AST/IR/effect, with correct subject, controller, target, zone, condition, quantity, and optional/otherwise wiring.
- Runtime/engine report: a targeted runtime code inspection or regression test proves the reported behavior is handled correctly.
- AI/frontend/deckbuilder report: inspect the subsystem that owns the behavior; card parser coverage is not evidence for these.

When evidence is weaker than this, keep or create the GitHub issue and label it `status:confirmed` or `status:needs-repro`. In notes, say what evidence is missing instead of calling it fixed.

Before calling any bug fixed, run the mandatory post-fix review gate above. Regressions discovered by review are part of the same bug-triage task and must be resolved before issue status changes.

### Parser-gap bugs (area:parser)
1. Check the card: `jq '.["card name"]' client/public/card-data.json`
2. Look for `Unimplemented` effects or `Unknown` triggers
3. Verify the specific ability mentioned in the bug has the expected typed semantics, not just a real effect type
4. If the ability is represented by `GenericEffect`, overly broad filters, wrong controller/target/zone, missing conditions, or swallowed clauses, the parser gap is still open

### Runtime/engine bugs (area:engine)
1. Read the bug description
2. Find the relevant handler in `crates/engine/src/game/effects/` or `crates/engine/src/game/`
3. Check if the described behavior is handled correctly, including the exact subject/controller/zone/timing from the report
4. Best: write a test that reproduces the bug scenario → if the test proves the reported bad behavior cannot occur, the bug is fixed

### AI bugs (area:ai)
1. Check `crates/phase-ai/` for the relevant evaluation/action-generation logic
2. AI bugs are rarely caught by parser coverage — they need gameplay testing

## Triage Data Files

| File | Description | Gitignored |
|------|-------------|------------|
| `triage/raw/discord-messages.jsonl` | Raw Discord messages (775+) | yes |
| `triage/report-items.jsonl` | Heuristic-extracted report items | yes |
| `triage/triage-items.jsonl` | Heuristic triage classifications | yes |
| `triage/llm-triage-items.jsonl` | LLM (Sonnet) triage — 333 items, best quality | yes |
| `triage/coverage-crossref.jsonl` | Cross-reference against parser coverage | yes |
| `triage/coverage-crossref-summary.md` | Human-readable summary | yes |
| `triage/p0-verification.md` | Manual spot-check of P0 likely-fixed bugs | yes |
| `triage/unknown-card-mapping.json` | Card name corrections | yes |
| `triage/no-card-bugs.md` | Engine/UI bugs not tied to cards | yes |
| `triage/threads-compact.json` | Compact thread data for LLM agent input | yes |
| `triage/sync-state.json` | Incremental fetch cursors | yes |
| `triage/dashboard.md` | Generated dashboard | yes |

## Label Taxonomy

| Group | Labels | Purpose |
|-------|--------|---------|
| status | needs-triage, needs-repro, confirmed, in-progress, fixed-unreleased, needs-card-data-regen, needs-runtime-verify, verified, stale, duplicate, wont-fix | Lifecycle |
| area | engine, parser, frontend, ui, ai, card-data, deckbuilder, multiplayer, infra | Ownership |
| priority | p0-softlock, p1-core-mechanic, p1-infinite-loop, p2-wrong-game-result, p2-interaction, p3-card-specific, p3-edge-case | Urgency |
| mechanic | triggered-abilities, mana, combat, tokens, costs, zone-change, continuous-effects, keyword, replacement-effects, counters, layers, attachments, modal, search, card-data-regen, ai-policy, targeting | Subsystem |
| source | discord, github, playtesting | Provenance |
| resolution | split, merged, upstream, cant-reproduce, by-design | Closure reason |
| special | collector | Omnibus issue marker |
