/**
 * Hotpath C fallbacks + weak stubs overridden by asm/x86_64 on x86_64.
 */
#include "sikuwa/hotpath.h"

#include <string.h>

#define SKW_HASH_SEED  UINT64_C(0x243F6A8885A308D3)
#define SKW_HASH_MUL   UINT64_C(0x9E3779B97F4A7C15)
#define SKW_HASH_MIX1  UINT64_C(0xFF51AFD7ED558CCD)
#define SKW_HASH_MIX2  UINT64_C(0xC4CEB9FE1A85EC53)

#if defined(__GNUC__) || defined(__clang__)
#define SKW_NOINLINE __attribute__((noinline))
#else
#define SKW_NOINLINE
#endif

static uint64_t skw_hash64_finalize(uint64_t h)
{
    h ^= h >> 33;
    h *= SKW_HASH_MIX1;
    h ^= h >> 33;
    h *= SKW_HASH_MIX2;
    h ^= h >> 33;
    return h;
}

SKW_NOINLINE SKW_API uint64_t skw_hash64_c(const void *data, size_t len)
{
    const uint8_t *p = (const uint8_t *)data;
    uint64_t h = SKW_HASH_SEED ^ (uint64_t)len;

    while (len >= 8) {
        uint64_t k;
        memcpy(&k, p, 8);
        h ^= k;
        h *= SKW_HASH_MUL;
        p += 8;
        len -= 8;
    }

    if (len > 0) {
        uint64_t tail = 0;
        memcpy(&tail, p, len);
        h ^= tail;
        h *= SKW_HASH_MUL;
    }

    return skw_hash64_finalize(h);
}

SKW_NOINLINE SKW_API int64_t skw_i64_add_checked_c(int64_t a, int64_t b, skw_status_t *st)
{
    if (st) {
        *st = SKW_OK;
    }
#if defined(__GNUC__) || defined(__clang__)
    int64_t out;
    if (__builtin_add_overflow(a, b, &out)) {
        if (st) {
            *st = SKW_ERR_RANGE;
        }
        return 0;
    }
    return out;
#else
    if ((b > 0 && a > INT64_MAX - b) || (b < 0 && a < INT64_MIN - b)) {
        if (st) {
            *st = SKW_ERR_RANGE;
        }
        return 0;
    }
    return a + b;
#endif
}

SKW_NOINLINE SKW_API int64_t skw_tagged_as_i64_c(const skw_tagged_t *t, skw_status_t *st)
{
    if (!t) {
        if (st) {
            *st = SKW_ERR_TYPE;
        }
        return 0;
    }
    if (t->tag != SKW_TAG_INT) {
        if (st) {
            *st = SKW_ERR_TYPE;
        }
        return 0;
    }
    if (st) {
        *st = SKW_OK;
    }
    return t->as.i;
}

#if !defined(SKW_HOTPATH_ASM)
SKW_API uint64_t skw_hash64(const void *data, size_t len)
{
    return skw_hash64_c(data, len);
}

SKW_API int64_t skw_i64_add_checked(int64_t a, int64_t b, skw_status_t *st)
{
    return skw_i64_add_checked_c(a, b, st);
}

SKW_API int64_t skw_tagged_as_i64(const skw_tagged_t *t, skw_status_t *st)
{
    return skw_tagged_as_i64_c(t, st);
}
#endif
