//! WASM host functions exposed to plugin modules.
//!
//! ## Current state (WIP — `wip/wasm-sandbox` branch)
//!
//! Stub module. No host functions are registered. The real surface (logging,
//! metrics, header rewriting, body search) is not yet designed.
//!
//! ## Planned shape
//!
//! ```ignore
//! pub struct WasmState {
//!     pub request: Mutex<InterceptedRequest>,
//! }
//!
//! pub fn add_host_functions(linker: &mut Linker<WasmState>) -> Result<(), String> {
//!     linker.func_wrap("env", "log", |_state: &mut WasmState, ptr: i32, len: i32| {
//!         // pull bytes from plugin memory and emit via log crate
//!     })?;
//!     linker.func_wrap("env", "set_header", |state, name_ptr, name_len, val_ptr, val_len| {
//!         // decode, lock state.request, mutate headers
//!     })?;
//!     Ok(())
//! }
//! ```
//!
//! The exact set of host functions and their signatures will be locked in
//! when the plugin v2 spec is finalized.

#[allow(dead_code)]
pub struct WasmState;
