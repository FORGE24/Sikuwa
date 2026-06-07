#ifndef SIKUWA_HOTPATH_H
#define SIKUWA_HOTPATH_H
/**
 * Sikuwa x86_64 hot paths — SIMD hash / checked int / tagged fast paths.
 * Implemented in asm/x86_64/{linux,win}/ with C fallbacks in hotpath/dispatch.c.
 */

#include "sikuwa/abi.h"
#include "sikuwa/runtime.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Fast 64-bit hash for cache keys (not cryptographic; use blake3 for source_hash). */
SKW_API uint64_t skw_hash64(const void *data, size_t len);

/** Integer add with overflow check; *st is SKW_OK or SKW_ERR_RANGE. */
SKW_API int64_t skw_i64_add_checked(int64_t a, int64_t b, skw_status_t *st);

/** Extract int64 from skw_tagged_t when tag == SKW_TAG_INT. */
SKW_API int64_t skw_tagged_as_i64(const skw_tagged_t *t, skw_status_t *st);

/** Pure C reference implementations (tests / non-x86). */
SKW_API uint64_t skw_hash64_c(const void *data, size_t len);
SKW_API int64_t skw_i64_add_checked_c(int64_t a, int64_t b, skw_status_t *st);
SKW_API int64_t skw_tagged_as_i64_c(const skw_tagged_t *t, skw_status_t *st);

#ifdef __cplusplus
}
#endif

#endif /* SIKUWA_HOTPATH_H */
