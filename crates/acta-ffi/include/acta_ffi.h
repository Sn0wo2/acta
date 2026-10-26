#ifndef ACTA_FFI_H
#define ACTA_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef uint64_t acta_ffi_handle;

typedef enum acta_ffi_level {
    ACTA_FFI_LEVEL_ERROR = 0,
    ACTA_FFI_LEVEL_WARN = 1,
    ACTA_FFI_LEVEL_INFO = 2,
    ACTA_FFI_LEVEL_DEBUG = 3,
    ACTA_FFI_LEVEL_TRACE = 4
} acta_ffi_level;

typedef enum acta_ffi_status {
    ACTA_FFI_STATUS_OK = 0,
    ACTA_FFI_STATUS_INVALID_ARGUMENT = 1,
    ACTA_FFI_STATUS_INVALID_HANDLE = 2,
    ACTA_FFI_STATUS_LOGGER_ERROR = 3,
    ACTA_FFI_STATUS_INTERNAL_ERROR = 4
} acta_ffi_status;

uint32_t acta_abi_version(void);
int32_t acta_logger_create(const uint8_t *config_json, size_t config_json_len,
                               size_t capacity, acta_ffi_handle *out_handle);
int32_t acta_logger_log(acta_ffi_handle handle, uint32_t level,
                            const uint8_t *message, size_t message_len);
int32_t acta_logger_event(acta_ffi_handle handle, const uint8_t *event_json,
                              size_t event_json_len);
int32_t acta_logger_create_span(acta_ffi_handle handle,
                                     const uint8_t *event_json,
                                     size_t event_json_len, uint32_t *out_id);
int32_t acta_logger_record_span(acta_ffi_handle handle, uint32_t id,
                                    const uint8_t *fields_json,
                                    size_t fields_json_len);
int32_t acta_logger_close_span(acta_ffi_handle handle, uint32_t id);
int32_t acta_logger_drain(acta_ffi_handle handle, uint8_t *output,
                               size_t output_capacity, size_t max_bytes,
                               size_t *out_written);
int32_t acta_logger_dropped(acta_ffi_handle handle, uint64_t *out_dropped);
int32_t acta_logger_set_filter(acta_ffi_handle handle, const uint8_t *filter,
                                    size_t filter_len);
int32_t acta_logger_destroy(acta_ffi_handle handle);

#ifdef __cplusplus
}
#endif

#endif
