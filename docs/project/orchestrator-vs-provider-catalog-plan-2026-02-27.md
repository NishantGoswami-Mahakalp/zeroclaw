# Orchestrator vs Provider Catalog — Stabilization Plan

Last updated: 2026-02-27

## Context

Investigation against upstream source (`/home/nishantg/Projects/Tools/temp-claw/zeroclaw`) confirms:

- Runtime orchestrator provider/model/key are first-class `config.toml` fields (`default_provider`, `default_model`, `api_key`).
- Delegate/sub-agent style provider usage exists separately under agent/delegation config.
- Integrations in upstream are tied to runtime config updates and do not rely on a separate providers DB API surface.

In this fork, a providers DB API (`/api/providers`) was added, which introduces dual config sources.

## Decision

Do **not** force-merge both systems.

Keep strict purpose separation:

1. **Orchestrator Runtime (controller)**
   - Source: `config.toml`
   - Used by: Agent Chat runtime execution path
   - Fields: `default_provider`, `default_model`, `api_key`, `api_url`, reliability flags

2. **Provider Catalog (sub-agents/profiles)**
   - Source: providers DB
   - Used by: provider inventory, profile-level provider definitions, future sub-agent workflows
   - Not implicitly consumed by orchestrator runtime

## Guardrails

- No implicit DB -> runtime override in chat path.
- No implicit runtime -> DB writes when editing runtime config.
- Any cross-system sync must be explicit user action ("Apply to Runtime").
- No hidden fallback behavior in orchestrator chat path.

## Implementation Sequence

1. Define source-of-truth contract and route ownership.
2. Enforce backend separation (remove implicit overlays).
3. Add explicit bridge endpoint/action with diff + audit trail.
4. Clarify UI copy and controls to remove ambiguity.
5. Add regression tests for precedence and no-fallback semantics.

## Tracking

- Parent: `zeroclaw-dwk`
- `zeroclaw-dwk.1`: precedence contract
- `zeroclaw-dwk.2`: backend separation
- `zeroclaw-dwk.3`: frontend clarity + explicit bridge UX
- `zeroclaw-dwk.4`: regression test suite
