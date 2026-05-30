//! Bridge utilities between `dispatch_data_t` and Rust `&[u8]`.
//!
//! Network.framework uses `dispatch_data_t` for zero-copy data passing.
//! This module will provide conversions:
//!
//! - `&[u8]` → `dispatch_data_t`  (for send)
//! - `dispatch_data_t` → `Vec<u8>` (for receive, with copy)
//! - `dispatch_data_t` → borrowed `&[u8]` (for receive, zero-copy where possible)
//!
//! # Implementation Notes
//!
//! `dispatch_data_t` is a reference-counted, potentially non-contiguous buffer.
//! For contiguous access, `dispatch_data_create_map()` creates a contiguous view.
//!
//! Key dispatch functions to bind:
//! - `dispatch_data_create(buffer, size, queue, destructor)` — wrap &[u8]
//! - `dispatch_data_create_map(data, &buffer_ptr, &size)` — get contiguous view
//! - `dispatch_data_get_size(data)` — get byte count
//! - `dispatch_release(data)` — release
//!
//! In the actual implementation, prefer using the `dispatch2` crate which
//! provides safe Rust wrappers for these operations.

// TODO: Implement when adding `dispatch2` dependency.
//
// use std::os::raw::c_void;
//
// /// Wrap a byte slice as dispatch_data_t for sending.
// /// The data is copied into dispatch-managed memory.
// pub fn bytes_to_dispatch_data(bytes: &[u8]) -> *mut c_void {
//     // dispatch_data_create(
//     //     bytes.as_ptr() as *const c_void,
//     //     bytes.len(),
//     //     std::ptr::null_mut(),          // NULL queue = default
//     //     DISPATCH_DATA_DESTRUCTOR_DEFAULT,
//     // )
//     todo!()
// }
//
// /// Extract bytes from dispatch_data_t into a Vec<u8>.
// /// This copies the data out of dispatch-managed memory.
// pub fn dispatch_data_to_vec(data: *mut c_void) -> Vec<u8> {
//     // let mut ptr: *const c_void = std::ptr::null();
//     // let mut size: usize = 0;
//     // let mapped = dispatch_data_create_map(data, &mut ptr, &mut size);
//     // let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, size) };
//     // let result = bytes.to_vec();
//     // dispatch_release(mapped);
//     // result
//     todo!()
// }
