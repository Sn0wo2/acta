#![allow(unsafe_code)]

use std::{
    collections::HashMap,
    str,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};

use acta::Logger;
use tracing::Level;

pub const ACTA_FFI_ABI_VERSION: u32 = 1;

pub const ACTA_FFI_STATUS_OK: i32 = 0;
pub const ACTA_FFI_STATUS_INVALID_ARGUMENT: i32 = 1;
pub const ACTA_FFI_STATUS_INVALID_HANDLE: i32 = 2;
pub const ACTA_FFI_STATUS_LOGGER_ERROR: i32 = 3;
pub const ACTA_FFI_STATUS_INTERNAL_ERROR: i32 = 4;

type Handle = u64;
type Status = i32;

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);
struct LoggerHandle {
    logger: Logger,
    output: acta::BufferedWriter,
}

static LOGGERS: OnceLock<Mutex<HashMap<Handle, Arc<LoggerHandle>>>> = OnceLock::new();

fn loggers() -> &'static Mutex<HashMap<Handle, Arc<LoggerHandle>>> {
    LOGGERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_handle() -> Result<Handle, Status> {
    let mut current = NEXT_HANDLE.load(Ordering::Relaxed);
    loop {
        let next = current
            .checked_add(1)
            .ok_or(ACTA_FFI_STATUS_INTERNAL_ERROR)?;
        match NEXT_HANDLE.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => return Ok(current),
            Err(observed) => current = observed,
        }
    }
}

fn with_logger<T>(
    handle: Handle,
    operation: impl FnOnce(&LoggerHandle) -> Result<T, Status>,
) -> Result<T, Status> {
    if handle == 0 {
        return Err(ACTA_FFI_STATUS_INVALID_HANDLE);
    }

    let logger = {
        let guard = loggers()
            .lock()
            .map_err(|_| ACTA_FFI_STATUS_INTERNAL_ERROR)?;
        Arc::clone(guard.get(&handle).ok_or(ACTA_FFI_STATUS_INVALID_HANDLE)?)
    };
    operation(&logger)
}

unsafe fn read_string<'a>(pointer: *const u8, length: usize) -> Result<&'a str, Status> {
    if length == 0 {
        return Ok("");
    }
    if pointer.is_null() || length > isize::MAX as usize {
        return Err(ACTA_FFI_STATUS_INVALID_ARGUMENT);
    }

    let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };
    str::from_utf8(bytes).map_err(|_| ACTA_FFI_STATUS_INVALID_ARGUMENT)
}

#[unsafe(no_mangle)]
pub extern "C" fn acta_abi_version() -> u32 {
    ACTA_FFI_ABI_VERSION
}

#[unsafe(no_mangle)]
unsafe extern "C" fn acta_logger_create(
    config_json: *const u8,
    config_json_len: usize,
    capacity: usize,
    out_handle: *mut Handle,
) -> Status {
    if out_handle.is_null() {
        return ACTA_FFI_STATUS_INVALID_ARGUMENT;
    }
    unsafe { out_handle.write(0) };

    let config_json = match unsafe { read_string(config_json, config_json_len) } {
        Ok(config_json) => config_json,
        Err(status) => return status,
    };
    let output = acta::BufferedWriter::new(capacity);
    let config = match acta::Config::from_json_with_writer(config_json, output.clone()) {
        Ok(config) => config,
        Err(_) => return ACTA_FFI_STATUS_LOGGER_ERROR,
    };
    let logger = match Logger::new(config) {
        Ok(logger) => logger,
        Err(_) => return ACTA_FFI_STATUS_LOGGER_ERROR,
    };

    let handle = match next_handle() {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let mut guard = match loggers().lock() {
        Ok(guard) => guard,
        Err(_) => return ACTA_FFI_STATUS_INTERNAL_ERROR,
    };
    guard.insert(handle, Arc::new(LoggerHandle { logger, output }));
    unsafe { out_handle.write(handle) };
    ACTA_FFI_STATUS_OK
}

#[unsafe(no_mangle)]
unsafe extern "C" fn acta_logger_log(
    handle: Handle,
    level: u32,
    message: *const u8,
    message_len: usize,
) -> Status {
    let message = match unsafe { read_string(message, message_len) } {
        Ok(message) => message,
        Err(status) => return status,
    };
    with_logger(handle, |logger| {
        let level = match level {
            0 => Level::ERROR,
            1 => Level::WARN,
            2 => Level::INFO,
            3 => Level::DEBUG,
            4 => Level::TRACE,
            _ => return Err(ACTA_FFI_STATUS_LOGGER_ERROR),
        };
        logger.logger.log(level, message);
        Ok(())
    })
    .map_or_else(|status| status, |_| ACTA_FFI_STATUS_OK)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn acta_logger_event(
    handle: Handle,
    event_json: *const u8,
    event_json_len: usize,
) -> Status {
    let json = match unsafe { read_string(event_json, event_json_len) } {
        Ok(json) => json,
        Err(status) => return status,
    };
    with_logger(handle, |logger| {
        let event = serde_json::from_str(json).map_err(|_| ACTA_FFI_STATUS_LOGGER_ERROR)?;
        logger
            .logger
            .event(&event)
            .map_err(|_| ACTA_FFI_STATUS_LOGGER_ERROR)
    })
    .map_or_else(|status| status, |_| ACTA_FFI_STATUS_OK)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn acta_logger_create_span(
    handle: Handle,
    event_json: *const u8,
    event_json_len: usize,
    out_id: *mut u32,
) -> Status {
    if out_id.is_null() {
        return ACTA_FFI_STATUS_INVALID_ARGUMENT;
    }
    unsafe { out_id.write(0) };

    let json = match unsafe { read_string(event_json, event_json_len) } {
        Ok(json) => json,
        Err(status) => return status,
    };
    with_logger(handle, |logger| {
        let event = serde_json::from_str(json).map_err(|_| ACTA_FFI_STATUS_LOGGER_ERROR)?;
        logger
            .logger
            .create_span(&event)
            .map_err(|_| ACTA_FFI_STATUS_LOGGER_ERROR)
    })
    .map_or_else(
        |status| status,
        |id| {
            unsafe { out_id.write(id) };
            ACTA_FFI_STATUS_OK
        },
    )
}

#[unsafe(no_mangle)]
unsafe extern "C" fn acta_logger_record_span(
    handle: Handle,
    id: u32,
    fields_json: *const u8,
    fields_json_len: usize,
) -> Status {
    let json = match unsafe { read_string(fields_json, fields_json_len) } {
        Ok(json) => json,
        Err(status) => return status,
    };
    with_logger(handle, |logger| {
        let fields = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(json)
            .map_err(|_| ACTA_FFI_STATUS_LOGGER_ERROR)?;
        logger
            .logger
            .record_span(id, &fields)
            .map_err(|_| ACTA_FFI_STATUS_LOGGER_ERROR)
    })
    .map_or_else(|status| status, |_| ACTA_FFI_STATUS_OK)
}

#[unsafe(no_mangle)]
extern "C" fn acta_logger_close_span(handle: Handle, id: u32) -> Status {
    with_logger(handle, |logger| {
        logger
            .logger
            .close_span(id)
            .map_err(|_| ACTA_FFI_STATUS_LOGGER_ERROR)
    })
    .map_or_else(|status| status, |_| ACTA_FFI_STATUS_OK)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn acta_logger_drain(
    handle: Handle,
    output: *mut u8,
    output_capacity: usize,
    max_bytes: usize,
    out_written: *mut usize,
) -> Status {
    if out_written.is_null()
        || output_capacity > isize::MAX as usize
        || (output_capacity > 0 && output.is_null())
    {
        return ACTA_FFI_STATUS_INVALID_ARGUMENT;
    }
    unsafe { out_written.write(0) };

    let max_bytes = max_bytes.min(output_capacity);
    let bytes = match with_logger(handle, |logger| Ok(logger.output.drain(max_bytes))) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    if bytes.len() > output_capacity {
        return ACTA_FFI_STATUS_INTERNAL_ERROR;
    }
    if !bytes.is_empty() {
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len()) };
    }
    unsafe { out_written.write(bytes.len()) };
    ACTA_FFI_STATUS_OK
}

#[unsafe(no_mangle)]
unsafe extern "C" fn acta_logger_dropped(handle: Handle, out_dropped: *mut u64) -> Status {
    if out_dropped.is_null() {
        return ACTA_FFI_STATUS_INVALID_ARGUMENT;
    }
    unsafe { out_dropped.write(0) };
    match with_logger(handle, |logger| Ok(logger.output.dropped())) {
        Ok(dropped) => {
            unsafe { out_dropped.write(dropped) };
            ACTA_FFI_STATUS_OK
        }
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn acta_logger_set_filter(
    handle: Handle,
    filter: *const u8,
    filter_len: usize,
) -> Status {
    let filter = match unsafe { read_string(filter, filter_len) } {
        Ok(filter) => filter,
        Err(status) => return status,
    };
    with_logger(handle, |logger| {
        logger
            .logger
            .set_filter(filter.parse().map_err(|_| ACTA_FFI_STATUS_LOGGER_ERROR)?)
            .map_err(|_| ACTA_FFI_STATUS_LOGGER_ERROR)
    })
    .map_or_else(|status| status, |_| ACTA_FFI_STATUS_OK)
}

#[unsafe(no_mangle)]
extern "C" fn acta_logger_destroy(handle: Handle) -> Status {
    if handle == 0 {
        return ACTA_FFI_STATUS_INVALID_HANDLE;
    }
    let removed = match loggers().lock() {
        Ok(mut guard) => guard.remove(&handle),
        Err(_) => return ACTA_FFI_STATUS_INTERNAL_ERROR,
    };
    if removed.is_some() {
        drop(removed);
        ACTA_FFI_STATUS_OK
    } else {
        ACTA_FFI_STATUS_INVALID_HANDLE
    }
}
