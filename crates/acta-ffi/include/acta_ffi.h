#ifndef ACTA_FFI_H
#define ACTA_FFI_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Status codes returned by every acta function.
 */
enum acta_ffi_status
#if __STDC_VERSION__ >= 202311L
  : int32_t
#endif // __STDC_VERSION__ >= 202311L
 {
  ACTA_FFI_STATUS_OK = 0,
  ACTA_FFI_STATUS_INVALID_ARGUMENT = 1,
  ACTA_FFI_STATUS_INVALID_HANDLE = 2,
  ACTA_FFI_STATUS_LOGGER_ERROR = 3,
  ACTA_FFI_STATUS_INTERNAL_ERROR = 4,
};
#if __STDC_VERSION__ >= 202311L
typedef enum acta_ffi_status acta_ffi_status;
#else
typedef int32_t acta_ffi_status;
#endif // __STDC_VERSION__ >= 202311L

/**
 * Log level values for `acta_logger_log`.
 */
enum acta_ffi_level
#if __STDC_VERSION__ >= 202311L
  : uint32_t
#endif // __STDC_VERSION__ >= 202311L
 {
  ACTA_FFI_LEVEL_ERROR = 0,
  ACTA_FFI_LEVEL_WARN = 1,
  ACTA_FFI_LEVEL_INFO = 2,
  ACTA_FFI_LEVEL_DEBUG = 3,
  ACTA_FFI_LEVEL_TRACE = 4,
};
#if __STDC_VERSION__ >= 202311L
typedef enum acta_ffi_level acta_ffi_level;
#else
typedef uint32_t acta_ffi_level;
#endif // __STDC_VERSION__ >= 202311L

/**
 * Opaque logger handle, created by `acta_logger_create`.
 */
typedef uint64_t acta_ffi_handle;

/**
 * Returns the ABI version of this library.
 */
uint32_t acta_abi_version(void);

/**
 * Returns the acta version as a NUL-terminated static string.
 */
const char *acta_version(void);

/**
 * Returns the message of the most recent failed call on this thread, or
 * null. The pointer is valid until the next failing call on the thread.
 */
const char *acta_last_error(void);

/**
 * Creates a logger from a JSON config, writing a handle to `out_handle`.
 *
 * # Safety
 * `config_json`/`config_json_len` follow `read_str`; `out_handle` must
 * point to writable memory.
 */
acta_ffi_status acta_logger_create(const uint8_t *config_json,
                                   uintptr_t config_json_len,
                                   uintptr_t capacity,
                                   acta_ffi_handle *out_handle);

/**
 * # Safety
 * `message`/`message_len` follow `read_str`; `handle` follows `logger`.
 */
acta_ffi_status acta_logger_log(acta_ffi_handle handle,
                                acta_ffi_level level,
                                const uint8_t *message,
                                uintptr_t message_len);

/**
 * # Safety
 * `event_json`/`event_json_len` follow `read_str`; `handle` follows `logger`.
 */
acta_ffi_status acta_logger_event(acta_ffi_handle handle,
                                  const uint8_t *event_json,
                                  uintptr_t event_json_len);

/**
 * Opens a span, writing its id to `out_id`.
 *
 * # Safety
 * `event_json`/`event_json_len` follow `read_str`; `handle` follows
 * `logger`; `out_id` must point to writable memory.
 */
acta_ffi_status acta_logger_create_span(acta_ffi_handle handle,
                                        const uint8_t *event_json,
                                        uintptr_t event_json_len,
                                        uint32_t *out_id);

/**
 * # Safety
 * `fields_json`/`fields_json_len` follow `read_str`; `handle` follows `logger`.
 */
acta_ffi_status acta_logger_record_span(acta_ffi_handle handle,
                                        uint32_t id,
                                        const uint8_t *fields_json,
                                        uintptr_t fields_json_len);

/**
 * # Safety
 * `handle` follows `logger`.
 */
acta_ffi_status acta_logger_close_span(acta_ffi_handle handle, uint32_t id);

/**
 * Drains up to `output_capacity` buffered bytes into `output` and reports
 * the cumulative dropped-byte count via `out_dropped`.
 *
 * # Safety
 * `output` must point to `output_capacity` writable bytes when the capacity
 * is non-zero; `out_written` and `out_dropped` must point to writable memory;
 * `handle` follows `logger`.
 */
acta_ffi_status acta_logger_drain(acta_ffi_handle handle,
                                  uint8_t *output,
                                  uintptr_t output_capacity,
                                  uintptr_t *out_written,
                                  uint64_t *out_dropped);

/**
 * # Safety
 * `filter`/`filter_len` follow `read_str`; `handle` follows `logger`.
 */
acta_ffi_status acta_logger_set_filter(acta_ffi_handle handle,
                                       const uint8_t *filter,
                                       uintptr_t filter_len);

/**
 * Destroys a logger. The handle must not be used again.
 *
 * # Safety
 * `handle` must be a token returned by `acta_logger_create` that has not
 * been destroyed, or 0.
 */
acta_ffi_status acta_logger_destroy(acta_ffi_handle handle);

#endif  /* ACTA_FFI_H */
