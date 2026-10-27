//! C ABI for acta (ABI version 2).
//!
//! Every function returns an `acta_ffi_status`; strings are passed as
//! pointer/length pairs. Logger handles are opaque 64-bit tokens backed by
//! magic-guarded allocations — no global registry, so calls cost one
//! pointer dereference. On failure, `acta_last_error()` returns the detail
//! message for the most recent failed call on the calling thread.
//!
//! `include/acta_ffi.h` is generated from this crate by cbindgen at build
//! time and must not be edited by hand.

#![allow(unsafe_code)]

use std::{slice, str};

use acta_abi as abi;

/// Opaque logger handle, created by `acta_logger_create`.
#[allow(non_camel_case_types)]
pub type acta_ffi_handle = u64;

/// Log level values for `acta_logger_log`.
#[allow(non_camel_case_types)]
#[repr(u32)]
pub enum acta_ffi_level {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

/// Status codes returned by every acta function.
#[allow(non_camel_case_types)]
#[repr(i32)]
pub enum acta_ffi_status {
    Ok = 0,
    InvalidArgument = 1,
    InvalidHandle = 2,
    LoggerError = 3,
    InternalError = 4,
}

/// Bytes stored at the start of every live logger allocation; a stale or
/// forged handle fails with `InvalidHandle` instead of crashing in the
/// common cases.
const HANDLE_MAGIC: u64 = u64::from_be_bytes(*b"ACTALOG2");

struct LoggerBox {
    magic: u64,
    logger: abi::Logger,
}

thread_local! {
    static LAST_ERROR: std::cell::RefCell<Option<std::ffi::CString>> =
        const { std::cell::RefCell::new(None) };
}

fn fail(error: &abi::Error) -> acta_ffi_status {
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = std::ffi::CString::new(error.message.as_str()).ok();
    });
    match error.status {
        abi::STATUS_INVALID_ARGUMENT => acta_ffi_status::InvalidArgument,
        abi::STATUS_INVALID_HANDLE => acta_ffi_status::InvalidHandle,
        abi::STATUS_LOGGER_ERROR => acta_ffi_status::LoggerError,
        _ => acta_ffi_status::InternalError,
    }
}

fn reject(message: &str) -> acta_ffi_status {
    fail(&abi::Error::new(abi::STATUS_INVALID_ARGUMENT, message))
}

/// Maps a binding operation to a status code, recording failures for
/// `acta_last_error`.
fn finish<T>(result: abi::Result<T>, commit: impl FnOnce(T)) -> acta_ffi_status {
    match result {
        Ok(value) => {
            commit(value);
            acta_ffi_status::Ok
        }
        Err(error) => fail(&error),
    }
}

/// # Safety
/// `handle` must be a token returned by `acta_logger_create` that has not
/// been destroyed, or 0.
unsafe fn logger(handle: acta_ffi_handle) -> abi::Result<&'static abi::Logger> {
    if handle == 0 {
        return Err(abi::Error::new(
            abi::STATUS_INVALID_HANDLE,
            "logger handle is null",
        ));
    }
    // SAFETY: per the caller contract `handle` is a live token; the magic
    // check rejects most stale or forged values before any field is used.
    let boxed = unsafe { &*(handle as *const LoggerBox) };
    if boxed.magic != HANDLE_MAGIC {
        return Err(abi::Error::new(
            abi::STATUS_INVALID_HANDLE,
            "logger handle is invalid or already destroyed",
        ));
    }
    Ok(&boxed.logger)
}

/// # Safety
/// When `length > 0`, `pointer` must point to `length` readable bytes.
unsafe fn read_str<'a>(pointer: *const u8, length: usize) -> abi::Result<&'a str> {
    if length == 0 {
        return Ok("");
    }
    if pointer.is_null() || length > isize::MAX as usize {
        return Err(abi::Error::new(
            abi::STATUS_INVALID_ARGUMENT,
            "string pointer is null",
        ));
    }
    // SAFETY: caller contract checked above.
    let bytes = unsafe { slice::from_raw_parts(pointer, length) };
    str::from_utf8(bytes).map_err(|error| {
        abi::Error::new(
            abi::STATUS_INVALID_ARGUMENT,
            format!("input is not valid UTF-8: {error}"),
        )
    })
}

/// Returns the ABI version of this library.
#[unsafe(no_mangle)]
pub extern "C" fn acta_abi_version() -> u32 {
    abi::ABI_VERSION
}

/// Returns the acta version as a NUL-terminated static string.
#[unsafe(no_mangle)]
pub extern "C" fn acta_version() -> *const std::ffi::c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

/// Returns the message of the most recent failed call on this thread, or
/// null. The pointer is valid until the next failing call on the thread.
#[unsafe(no_mangle)]
pub extern "C" fn acta_last_error() -> *const std::ffi::c_char {
    LAST_ERROR.with(|slot| {
        slot.borrow()
            .as_ref()
            .map_or(std::ptr::null(), |error| error.as_ptr())
    })
}

/// Creates a logger from a JSON config, writing a handle to `out_handle`.
///
/// # Safety
/// `config_json`/`config_json_len` follow `read_str`; `out_handle` must
/// point to writable memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn acta_logger_create(
    config_json: *const u8,
    config_json_len: usize,
    capacity: usize,
    out_handle: *mut acta_ffi_handle,
) -> acta_ffi_status {
    if out_handle.is_null() {
        return reject("out_handle pointer is null");
    }
    // SAFETY: out pointer is non-null.
    unsafe { out_handle.write(0) };
    finish(
        unsafe { read_str(config_json, config_json_len) }
            .and_then(|config_json| abi::Logger::from_config_json(config_json, capacity))
            .map(|logger| {
                Box::into_raw(Box::new(LoggerBox {
                    magic: HANDLE_MAGIC,
                    logger,
                })) as acta_ffi_handle
            }),
        |handle| {
            // SAFETY: out pointer is non-null.
            unsafe { out_handle.write(handle) };
        },
    )
}

/// # Safety
/// `message`/`message_len` follow `read_str`; `handle` follows `logger`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn acta_logger_log(
    handle: acta_ffi_handle,
    level: acta_ffi_level,
    message: *const u8,
    message_len: usize,
) -> acta_ffi_status {
    finish(
        unsafe { logger(handle) }.and_then(|logger| {
            let message = unsafe { read_str(message, message_len) }?;
            logger.log(level as u32, message)
        }),
        |_| {},
    )
}

/// # Safety
/// `event_json`/`event_json_len` follow `read_str`; `handle` follows `logger`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn acta_logger_event(
    handle: acta_ffi_handle,
    event_json: *const u8,
    event_json_len: usize,
) -> acta_ffi_status {
    finish(
        unsafe { logger(handle) }.and_then(|logger| {
            let event_json = unsafe { read_str(event_json, event_json_len) }?;
            logger.event_json(event_json)
        }),
        |_| {},
    )
}

/// Opens a span, writing its id to `out_id`.
///
/// # Safety
/// `event_json`/`event_json_len` follow `read_str`; `handle` follows
/// `logger`; `out_id` must point to writable memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn acta_logger_create_span(
    handle: acta_ffi_handle,
    event_json: *const u8,
    event_json_len: usize,
    out_id: *mut u32,
) -> acta_ffi_status {
    if out_id.is_null() {
        return reject("out_id pointer is null");
    }
    // SAFETY: out pointer is non-null.
    unsafe { out_id.write(0) };
    finish(
        unsafe { logger(handle) }.and_then(|logger| {
            let event_json = unsafe { read_str(event_json, event_json_len) }?;
            logger.create_span_json(event_json)
        }),
        |id| {
            // SAFETY: out pointer is non-null.
            unsafe { out_id.write(id) };
        },
    )
}

/// # Safety
/// `fields_json`/`fields_json_len` follow `read_str`; `handle` follows `logger`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn acta_logger_record_span(
    handle: acta_ffi_handle,
    id: u32,
    fields_json: *const u8,
    fields_json_len: usize,
) -> acta_ffi_status {
    finish(
        unsafe { logger(handle) }.and_then(|logger| {
            let fields_json = unsafe { read_str(fields_json, fields_json_len) }?;
            logger.record_span_json(id, fields_json)
        }),
        |_| {},
    )
}

/// # Safety
/// `handle` follows `logger`.
#[unsafe(no_mangle)]
pub extern "C" fn acta_logger_close_span(handle: acta_ffi_handle, id: u32) -> acta_ffi_status {
    finish(
        unsafe { logger(handle) }.and_then(|logger| logger.close_span(id)),
        |_| {},
    )
}

/// Drains up to `output_capacity` buffered bytes into `output` and reports
/// the cumulative dropped-byte count via `out_dropped`.
///
/// # Safety
/// `output` must point to `output_capacity` writable bytes when the capacity
/// is non-zero; `out_written` and `out_dropped` must point to writable memory;
/// `handle` follows `logger`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn acta_logger_drain(
    handle: acta_ffi_handle,
    output: *mut u8,
    output_capacity: usize,
    out_written: *mut usize,
    out_dropped: *mut u64,
) -> acta_ffi_status {
    if out_written.is_null()
        || out_dropped.is_null()
        || output_capacity > isize::MAX as usize
        || (output_capacity > 0 && output.is_null())
    {
        return reject("output pointers are invalid");
    }
    // SAFETY: out pointers are non-null.
    unsafe {
        out_written.write(0);
        out_dropped.write(0);
    }
    finish(
        unsafe { logger(handle) }.map(|logger| {
            let bytes = logger.drain(output_capacity);
            if !bytes.is_empty() {
                // SAFETY: drain returns at most `output_capacity` bytes and
                // `output` is valid for that many.
                unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len()) };
            }
            // SAFETY: out pointers are non-null.
            unsafe {
                out_written.write(bytes.len());
                out_dropped.write(logger.dropped());
            }
        }),
        |_| {},
    )
}

/// # Safety
/// `filter`/`filter_len` follow `read_str`; `handle` follows `logger`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn acta_logger_set_filter(
    handle: acta_ffi_handle,
    filter: *const u8,
    filter_len: usize,
) -> acta_ffi_status {
    finish(
        unsafe { logger(handle) }.and_then(|logger| {
            let filter = unsafe { read_str(filter, filter_len) }?;
            logger.set_filter(filter)
        }),
        |_| {},
    )
}

/// Destroys a logger. The handle must not be used again.
///
/// # Safety
/// `handle` must be a token returned by `acta_logger_create` that has not
/// been destroyed, or 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn acta_logger_destroy(handle: acta_ffi_handle) -> acta_ffi_status {
    match unsafe { logger(handle) } {
        Err(error) => fail(&error),
        // SAFETY: the magic check in `logger` passed; reclaim the allocation.
        Ok(_) => {
            drop(unsafe { Box::from_raw(handle as *mut LoggerBox) });
            acta_ffi_status::Ok
        }
    }
}
