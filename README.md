# playground

## Structure

- `crates/golden_core`, `crates/golden_schema`, `crates/golden_net`, `crates/golden_macros`, `crates/golden_prelude`:
	Golden platform crates that can later be split as a submodule.
- `crates/golden_app`:
	Reusable runtime/bootstrap + Tauri launcher APIs (server + engine loop + headless/window launch).
- `src/main.rs`:
	Demo app entrypoint that declares custom/demo nodes and calls Golden launch APIs.
- `src-ui`:
	Host Svelte app (app-specific) that references base UI libraries from `crates/golden_ui/src/lib`.
- `crates/golden_ui/src/lib`:
	Reusable base UI components/stores/panels for Golden.

## Dev workflow

- Start full stack: `scripts/dev.ps1` (Windows) or `scripts/dev.sh` (Unix)
- Rust demo app only: `cargo run`
- Tauri shell only: `cargo run -p playground_demo`
- UI host only: `npm --prefix src-ui run dev`
