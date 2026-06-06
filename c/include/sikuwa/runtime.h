#ifndef SIKUWA_RUNTIME_H
#define SIKUWA_RUNTIME_H

#include "sikuwa/abi.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Opaque dynamic value (S3). Plan 4: forward decl only. */
typedef struct skw_value skw_value_t;

typedef struct skw_str {
    const char *data;
    size_t len;
} skw_str_t;

typedef enum skw_tag {
    SKW_TAG_NONE = 0,
    SKW_TAG_BOOL = 1,
    SKW_TAG_INT = 2,
    SKW_TAG_FLOAT = 3,
    SKW_TAG_STR = 4,
    SKW_TAG_OBJECT = 5,
} skw_tag_t;

typedef struct skw_tagged {
    skw_tag_t tag;
    union {
        int64_t i;
        double f;
        skw_str_t s;
        skw_value_t *obj;
    } as;
} skw_tagged_t;

/** S3 value helpers (libsikuwa_rt). */
SKW_API skw_value_t *skw_value_from_i64(int64_t v);
SKW_API int64_t skw_value_to_i64(skw_value_t *v, skw_status_t *st);
SKW_API void skw_value_release(skw_value_t *v);
SKW_API skw_tagged_t skw_tagged_from_i64(int64_t v);

#ifdef __cplusplus
}
#endif

#endif /* SIKUWA_RUNTIME_H */
