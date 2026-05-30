//! Raw FFI bindings to Apple's Network.framework C API (`nw_*` functions).
//!
//! Only the subset needed for Xpresso's UDP backend is declared here.
//! Reference: <https://developer.apple.com/documentation/network>
//!
//! # Safety
//! All functions in this module are `unsafe extern "C"`. Callers must ensure
//! correct object lifetimes and retain/release semantics.

#![allow(non_camel_case_types, dead_code, unused_imports)]

use std::ffi::c_char;
use std::os::raw::c_void;

// ---------------------------------------------------------------------------
// Opaque types — Network.framework objects are reference-counted OS objects.
// ---------------------------------------------------------------------------

/// Opaque handle to `nw_parameters_t`.
pub enum nw_parameters {} // TODO: nw_parameters_create_secure_udp, nw_parameters_set_local_endpoint

/// Opaque handle to `nw_endpoint_t`.
pub enum nw_endpoint {} // TODO: nw_endpoint_create_host

/// Opaque handle to `nw_connection_t`.
pub enum nw_connection {} // TODO: nw_connection_create, send, receive, start, cancel

/// Opaque handle to `nw_listener_t`.
pub enum nw_listener {} // TODO: nw_listener_create, start, set_new_connection_handler

/// Opaque handle to `nw_error_t`.
pub enum nw_error {} // TODO: nw_error_get_error_code

/// Opaque handle to `nw_content_context_t`.
pub enum nw_content_context {} // TODO: used in send/receive callbacks

/// Opaque handle to `nw_protocol_options_t`.
pub enum nw_protocol_options {} // TODO: protocol configuration

/// Opaque handle to `nw_protocol_stack_t`.
pub enum nw_protocol_stack {} // TODO: nw_parameters_copy_default_protocol_stack

pub type nw_parameters_t = *mut nw_parameters;
pub type nw_endpoint_t = *mut nw_endpoint;
pub type nw_connection_t = *mut nw_connection;
pub type nw_listener_t = *mut nw_listener;
pub type nw_error_t = *mut nw_error;
pub type nw_content_context_t = *mut nw_content_context;
pub type nw_protocol_options_t = *mut nw_protocol_options;
pub type nw_protocol_stack_t = *mut nw_protocol_stack;

// ---------------------------------------------------------------------------
// dispatch types — from libdispatch (GCD).
// These will be bridged properly via `dispatch2` crate in implementation.
// ---------------------------------------------------------------------------

pub type dispatch_queue_t = *mut c_void;
pub type dispatch_data_t = *mut c_void;

// ---------------------------------------------------------------------------
// Connection state enum
// ---------------------------------------------------------------------------

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum nw_connection_state_t {
    Invalid = 0,
    Waiting = 1,
    Preparing = 2,
    Ready = 3,
    Failed = 4,
    Cancelled = 5,
}

// ---------------------------------------------------------------------------
// Listener state enum
// ---------------------------------------------------------------------------

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum nw_listener_state_t {
    Invalid = 0,
    Waiting = 1,
    Ready = 2,
    Failed = 3,
    Cancelled = 4,
}

// ---------------------------------------------------------------------------
// Block types — these are Objective-C Block signatures.
// In actual implementation, use `block2::RcBlock` to create these from
// Rust closures.
// ---------------------------------------------------------------------------

// nw_parameters_configure_protocol_block_t = ^(nw_protocol_options_t)
// nw_connection_state_changed_handler_t = ^(nw_connection_state_t, nw_error_t)
// nw_connection_send_completion_t = ^(nw_error_t)
// nw_connection_receive_completion_t = ^(dispatch_data_t, nw_content_context_t, bool, nw_error_t)
// nw_listener_state_changed_handler_t = ^(nw_listener_state_t, nw_error_t)
// nw_listener_new_connection_handler_t = ^(nw_connection_t)

// ---------------------------------------------------------------------------
// extern "C" — Network.framework functions
// Link with: -framework Network
// ---------------------------------------------------------------------------

// TODO: Uncomment and implement when adding `block2` + `dispatch2` dependencies.
//
// extern "C" {
//     // -- Parameters --
//     pub fn nw_parameters_create_secure_udp(
//         configure_tls: *const c_void,   // NW_PARAMETERS_DISABLE_PROTOCOL
//         configure_udp: *const c_void,   // NW_PARAMETERS_DEFAULT_CONFIGURATION
//     ) -> nw_parameters_t;
//
//     pub fn nw_parameters_set_local_endpoint(
//         parameters: nw_parameters_t,
//         local_endpoint: nw_endpoint_t,
//     );
//
//     pub fn nw_parameters_copy_default_protocol_stack(
//         parameters: nw_parameters_t,
//     ) -> nw_protocol_stack_t;
//
//     // -- Endpoint --
//     pub fn nw_endpoint_create_host(
//         hostname: *const c_char,
//         port: *const c_char,
//     ) -> nw_endpoint_t;
//
//     pub fn nw_endpoint_get_hostname(endpoint: nw_endpoint_t) -> *const c_char;
//     pub fn nw_endpoint_get_port(endpoint: nw_endpoint_t) -> u16;
//
//     // -- Connection --
//     pub fn nw_connection_create(
//         endpoint: nw_endpoint_t,
//         parameters: nw_parameters_t,
//     ) -> nw_connection_t;
//
//     pub fn nw_connection_set_queue(
//         connection: nw_connection_t,
//         queue: dispatch_queue_t,
//     );
//
//     pub fn nw_connection_set_state_changed_handler(
//         connection: nw_connection_t,
//         handler: *const c_void, // Block: ^(nw_connection_state_t, nw_error_t)
//     );
//
//     pub fn nw_connection_start(connection: nw_connection_t);
//     pub fn nw_connection_cancel(connection: nw_connection_t);
//
//     pub fn nw_connection_send(
//         connection: nw_connection_t,
//         content: dispatch_data_t,            // nullable
//         context: nw_content_context_t,
//         is_complete: bool,
//         completion: *const c_void,           // Block: ^(nw_error_t)
//     );
//
//     pub fn nw_connection_receive(
//         connection: nw_connection_t,
//         minimum_incomplete_length: u32,
//         maximum_length: u32,
//         completion: *const c_void,           // Block: ^(dispatch_data_t, nw_content_context_t, bool, nw_error_t)
//     );
//
//     // -- Listener --
//     pub fn nw_listener_create(parameters: nw_parameters_t) -> nw_listener_t;
//
//     pub fn nw_listener_set_queue(
//         listener: nw_listener_t,
//         queue: dispatch_queue_t,
//     );
//
//     pub fn nw_listener_set_state_changed_handler(
//         listener: nw_listener_t,
//         handler: *const c_void, // Block: ^(nw_listener_state_t, nw_error_t)
//     );
//
//     pub fn nw_listener_set_new_connection_handler(
//         listener: nw_listener_t,
//         handler: *const c_void, // Block: ^(nw_connection_t)
//     );
//
//     pub fn nw_listener_start(listener: nw_listener_t);
//     pub fn nw_listener_cancel(listener: nw_listener_t);
//     pub fn nw_listener_get_port(listener: nw_listener_t) -> u16;
//
//     // -- Retain/Release (OS object) --
//     pub fn nw_retain(obj: *mut c_void);
//     pub fn nw_release(obj: *mut c_void);
//
//     // -- Error --
//     pub fn nw_error_get_error_code(error: nw_error_t) -> i32;
//
//     // -- Constants (linked as extern statics) --
//     // NW_PARAMETERS_DISABLE_PROTOCOL: *const c_void
//     // NW_PARAMETERS_DEFAULT_CONFIGURATION: *const c_void
//     // NW_CONNECTION_DEFAULT_MESSAGE_CONTEXT: nw_content_context_t
//     // NW_CONNECTION_FINAL_MESSAGE_CONTEXT: nw_content_context_t
// }
