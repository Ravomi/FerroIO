# RavomIO AI Agent Engineering Guide (MCP, Skill & Google Rust Standards)

*   **Author:** Ravomi
*   **Status:** Draft
*   **Date:** 2026-11-26

--------------------------------------------------------------------------------

## Overview

As RavomIO evolves into the PlatformIO for Rust embedded development, AI coding agents increasingly author crates, extend `ravom-plugin`, and orchestrate `mio` CLI workflows. Without an explicit, machine-readable engineering contract in the repository root, agents risk introducing `.unwrap()` calls, breaking `init -> build` routing, or wrapping CLI commands with brittle stdout scraping.

This design defines the structure and governance of `AGENTS.md` (and supporting MCP/Skill conventions) so AI agents operate safely and consistently across Rust code generation, MCP tool integration, and Agent Skill authoring.

## The Problem

1.  **Missing Rust style enforcement for agents**: While `Cargo.toml` configures `clippy::unwrap_used` and `clippy::wildcard_imports`, agents lack upfront guidance on Google Rust Style Guide conventions (error propagation via `CoreError`, `// SAFETY:` invariants, and `///` rustdoc requirements) before writing code.
2.  **Unconstrained MCP tool wrapping**: Without explicit rules, agents exposing RavomIO over MCP may shell out to interactive prompts or parse human-readable stdout instead of structured `serde::Serialize` outputs and `--dry-run` / `plan_build` checks.
3.  **Ad-hoc Skill workflows**: Without a canonical `boards -> init -> build -> flash -> monitor` sequence and idempotency checks, generated `SKILL.md` workflows can overwrite existing `ravom.toml` files or trigger hardware flashes without prior build verification.

## Goals

*   **Single authoritative AI entry point**: Maintain `AGENTS.md` at the repository root so every coding agent automatically loads architecture boundaries, Google Rust Style rules, MCP contracts, and Skill conventions.
*   **Zero-ambiguity Rust quality gates**: Require `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` before task completion, with zero `.unwrap()` or `todo!()` in production code.
*   **Safe, deterministic MCP & Skill contracts**: Mandate non-interactive CLI flags, structured outputs, `plan_build` dry-runs before hardware mutation, and `ravom.toml` existence checks before `init`.

## Non-Goals

*   **Implementing a full `ravom-mcp` server binary**: This document defines the engineering and interface contract for MCP integration; building a dedicated MCP server crate is a separate effort.
*   **Replacing human-facing `README.md` or `CONTRIBUTING.md`**: `AGENTS.md` targets AI agents and does not duplicate end-user installation tutorials.

## Critical User Journeys (CUJs)

### CUJ 1: AI Agent Authoring Rust Code in `ravom-core` or `ravom-plugin`

*   **Given**: An AI coding agent is asked to add a new framework plugin or modify `ravom-core`.
*   **When**: The agent reads `AGENTS.md` at session start.
*   **Then**: The agent propagates errors with `?` and `CoreError`, adds `///` rustdoc comments and unit tests, and verifies `cargo fmt` and `cargo clippy --workspace --all-targets -- -D warnings` pass.

### CUJ 2: AI Agent Wrapping `mio` in an MCP Tool or `SKILL.md`

*   **Given**: An AI agent is tasked with creating a RavomIO `SKILL.md` or MCP tool wrapper.
*   **When**: The agent designs the command sequence.
*   **Then**: The workflow uses explicit non-interactive flags (`--board`, `--framework`), checks for existing `ravom.toml` before `init`, and runs `plan_build` / `mio build` before `mio flash`.

## Technical Plan (Architecture)

### 1. Document Structure (`AGENTS.md`)

`AGENTS.md` is organized into four machine-scannable sections loaded automatically by AI coding harnesses:

1.  **Repository Architecture (Quick Map)**: Maps `ravom-plugin`, `ravom-core`, `ravom-cli`, `ravom-ui`, `boards/`, and `docs/designs/` so agents locate symbols without full-repo scans.
2.  **Rust Engineering Standards (Google Rust Style Guide)**:
    *   Error handling: `?` + `CoreError` (`thiserror`); zero `.unwrap()` or `todo!()` in production code.
    *   Safety: `#![warn(unsafe_op_in_unsafe_fn)]` + `// SAFETY:` comments.
    *   Formatting & lints: `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`.
    *   Documentation & tests: `///` rustdoc on all `pub` items and unit tests for new plugins/routes.
3.  **MCP Integration Guidelines**:
    *   Structured `serde::Serialize` return types over stdout scraping.
    *   Non-interactive execution with explicit CLI flags (`--board`, `--framework`, `--env`, `--release`).
    *   `plan_build` / `--dry-run` before hardware-mutating `flash` or `west` operations.
4.  **Agent Skill Authoring & Usage**:
    *   Canonical workflow order: `boards --search` -> `init` -> `build` -> `flash` -> `monitor`.
    *   Idempotency guard: check for existing `ravom.toml` before `init`.

```text
AI Agent / MCP Client
  │
  ├── Reads AGENTS.md (Google Rust Style + Architecture Map)
  │     └── Enforces: no .unwrap(), CoreError, rustdoc, clippy -D warnings
  │
  └── Invokes Skill / MCP Tool
        └── 1. boards --search (discover)
            2. init --board --framework (idempotent check: ravom.toml)
            3. plan_build / mio build (verify backend & compile)
            4. mio flash -> mio monitor (only when hardware attached)
```

## Alternatives Considered

### Alternative A: Embed AI Rules Inside `CONTRIBUTING.md`

*   **Pros**: Fewer root-level markdown files.
*   **Cons**: Most AI coding harnesses (Pi, Claude Code, Codex, Antigravity) automatically load `AGENTS.md` at session start, whereas `CONTRIBUTING.md` is ignored unless manually read.
*   **Reason for Rejection**: Rejected by author: `AGENTS.md` is the standard auto-loaded contract for coding agents.

### Alternative B: Split Rules Across Multiple Files Without a Root Index

*   **Pros**: Keeps individual topic files short.
*   **Cons**: Agents miss rules that are not linked or auto-loaded, leading to inconsistent Rust lint compliance and unsafe MCP/Skill wrappers.
*   **Reason for Rejection**: Rejected by author: a single concise `AGENTS.md` at the repo root guarantees full visibility on every turn.

--------------------------------------------------------------------------------

## Detailed Implementation Plan

### Target Files to Create/Modify

#### 1. `AGENTS.md` (Create / Maintain)

*   **Type / Scope**: Root AI Agent Engineering Guide
*   **Proposed Changes**:
    *   Maintain the 4-section specification: Repository Architecture, Google Rust Style Guide Standards (`?` + `CoreError`, no `.unwrap()`, `///` rustdoc, `cargo fmt` & `cargo clippy -D warnings`), MCP Integration Guidelines (structured `serde::Serialize` outputs, non-interactive flags, `plan_build` dry-runs), and Agent Skill Authoring (`boards -> init -> build -> flash -> monitor` + `ravom.toml` idempotency check).

### Verification Plan

*   Verify `AGENTS.md` exists at `/home/waya/Projects/RavomIO/AGENTS.md` and is auto-loaded in the harness context.
*   Verify all referenced crates, commands (`mio boards`, `mio init`, `mio build`, `mio flash`, `mio monitor`), and lints (`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`) match `Cargo.toml` and `crates/`.
