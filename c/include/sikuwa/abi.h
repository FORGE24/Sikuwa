#ifndef SIKUWA_ABI_H
#define SIKUWA_ABI_H
/**
 * Sikuwa native C FFI — stable ABI surface (SKW_ABI_1).
 * Do not break existing symbols; only extend.
 */

#include <stddef.h>
#include <stdint.h>

#define SKW_ABI_VERSION_MAJOR 1
#define SKW_ABI_VERSION_MINOR 0
#define SKW_ABI_STRING "1.0"

#if defined(_WIN32) || defined(__CYGWIN__)
  #ifdef SKW_BUILDING_MODULE
    #define SKW_EXPORT __declspec(dllexport)
  #else
    #define SKW_EXPORT __declspec(dllimport)
  #endif
  #define SKW_IMPORT __declspec(dllimport)
  #define SKW_CALL   __cdecl
#else
  #define SKW_EXPORT __attribute__((visibility("default")))
  #define SKW_IMPORT
  #define SKW_CALL
#endif

#ifdef SKW_BUILDING_MODULE
  #define SKW_API SKW_EXPORT
#else
  #define SKW_API SKW_IMPORT
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef enum skw_status {
    SKW_OK = 0,
    SKW_ERR_TYPE = 1,
    SKW_ERR_RANGE = 2,
    SKW_ERR_OOM = 3,
    SKW_ERR_UNREACHABLE = 4,
    SKW_ERR_PYTHON = 5,
} skw_status_t;

typedef struct skw_result_i64 {
    skw_status_t status;
    int64_t value;
} skw_result_i64_t;

#ifdef __cplusplus
}
#endif

#endif /* SIKUWA_ABI_H */
