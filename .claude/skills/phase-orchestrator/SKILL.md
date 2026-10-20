---
name: phase-orchestrator
description: Work through a roadmap phase in docs/progress.md by running one subagent per session-sized step, sequentially, committing each step's work before launching the next. Use when asked to "manage agents", "continue phase N", or "dispatch the next step".
---

# Phase orchestrator

You coordinate; subagents do the work. Keep your own context small: don't read code or the
subagents' transcripts, and only skim `docs/progress.md` when you need to answer the user.

## Loop

1. Check `git status` is clean (commit or ask about stray changes first) and skim
   `docs/progress.md` "Next steps, one per session" so you can tell the user what's next.
2. Launch ONE agent: `subagent_type: general-purpose`, `run_in_background: true`, prompt = the
   template below with `{PHASE}` filled in. Never run two at once: they share the working tree,
   the GPU (benchmarks skew each other), and the BRP port.
3. When it reports back:
   - `git status --short` to see what changed. Sanity check against AGENTS.md: build config
     (`Cargo.toml` profiles/features, `.cargo/config.toml`, `.justfile`) should be untouched unless
     the step was about it; a generator output change needs a `BAKER_VERSION` bump.
   - If the report says tests fail or something is left broken, stop and tell the user.
   - Otherwise `git add -A` and commit with the agent's suggested one-line message. **No
     attribution trailers** (no `Claude-Session:`, no `Co-Authored-By`).
   - Give the user a short summary: what was built, key measurements, anything uncertain.
4. Launch the next agent, until a report says `PHASE{N}_COMPLETE: yes` or the user says stop.

Push only when the user asks.

## Housekeeping

- If a roadmap item's section in `docs/progress.md` grows into a session-by-session log, or an
  item is being closed out, run a docs-only agent to condense it: final design, final
  measurements (one table), one-line dead ends, decisions, open issues; and rewrite the next-steps
  list. Then commit (`docs: ...`).
- If an item is stuck chasing a quality or performance target across several sessions, surface it
  to the user (e.g. ship it opt-in and move on) rather than looping.
- Scope changes the user agrees to (e.g. a richer version of a roadmap item) go into the
  next-steps list via the condensing agent or the next step's agent.

## Subagent prompt template

```
You are one of a sequence of agents working through Phase {PHASE} of VantaBlock (repo: C:\Users\bottm\dev\vantablock, current branch, Windows, RTX 3080 / Vulkan). Each agent does exactly ONE session-sized step, then stops. A coordinator commits your work and launches the next agent, so keep your context lean: read only what you need, and don't commit.

1. Read `AGENTS.md` (project conventions — follow them exactly), then `docs/progress.md` (especially "Status", "Phase {PHASE} results", and "Next steps, one per session") and the Phase {PHASE} section of `docs/design.md`. Consult `docs/architecture.md` only for the parts you touch.
2. Pick the next unfinished step: the first item in "Next steps, one per session". If that list is empty, pick the next Phase {PHASE} roadmap item and break it into session-sized steps, doing the first one. If a step is clearly too big for one session, do a coherent first slice and split the rest into new steps.
3. Implement it. Respect CPU/GPU parity, shader variant coverage, and the layout sync rules in AGENTS.md. Measure before replacing working algorithms.
4. Verify: `just test` (cargo-nextest), `just clippy` (must be warning-free), and `just test-gpu` for any shader/render change. Run GPU benchmarks one at a time with `--release` when a step calls for measurement, and record numbers. `nix fmt` is unavailable on Windows: use `cargo fmt` and match the existing WESL formatting by hand. Builds link Bevy dynamically by default (see "Fast Builds" in AGENTS.md); don't change build configuration.
   BRP is available: the `mcp__brp__*` tools are deferred, so load them with ToolSearch (e.g. `select:mcp__brp__brp_launch,mcp__brp__brp_extras_screenshot,mcp__brp__brp_shutdown,...`). For any render, UI, or lifecycle change, follow "Interactive Testing (BRP)" in AGENTS.md: build with `just brp-build --release` (it copies the hashed bevy dylib next to the exe, so a direct launch works without PATH changes), launch, enter Singleplayer, take screenshots (view them with Read), toggle settings via `world_mutate_resources` where relevant, and always shut the game down when finished.
   Do not leave failing tests; if something can't be made to pass, stop and report precisely.
5. Update `docs/progress.md` concisely: fold results into the item's section in "Phase {PHASE} results" (final design, final measurements, decisions, open issues; no session-by-session log or superseded numbers), and rewrite "Next steps, one per session" so the next agent knows exactly what to do. Update Status and `docs/architecture.md`/`AGENTS.md` if what you built changes what they describe. If all of Phase {PHASE} is complete, say so explicitly in Status.
6. Do NOT `git commit` or push. Finish with a short report (under 250 words): what you did, test/clippy/test-gpu results, BRP checks done (or why skipped), measurements, anything left broken or uncertain, a suggested one-line conventional commit message (style: `feat: ...`, `fix: ...`, lower case), and whether Phase {PHASE} is now complete (`PHASE{PHASE}_COMPLETE: yes|no`).
```
