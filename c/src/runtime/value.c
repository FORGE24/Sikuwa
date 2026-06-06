/**
 * Sikuwa S3 dynamic value runtime (Plan 4 minimal).
 */
#include "sikuwa/runtime.h"

#include <stdlib.h>

struct skw_value {
    skw_tag_t tag;
    union {
        int64_t i;
        double f;
    } as;
};

SKW_API skw_value_t *skw_value_from_i64(int64_t v) {
    skw_value_t *o = (skw_value_t *)malloc(sizeof(skw_value_t));
    if (!o) {
        return NULL;
    }
    o->tag = SKW_TAG_INT;
    o->as.i = v;
    return o;
}

SKW_API int64_t skw_value_to_i64(skw_value_t *v, skw_status_t *st) {
    if (st) {
        *st = SKW_OK;
    }
    if (!v) {
        if (st) {
            *st = SKW_ERR_TYPE;
        }
        return 0;
    }
    if (v->tag != SKW_TAG_INT) {
        if (st) {
            *st = SKW_ERR_TYPE;
        }
        return 0;
    }
    return v->as.i;
}

SKW_API void skw_value_release(skw_value_t *v) {
    free(v);
}

SKW_API skw_tagged_t skw_tagged_from_i64(int64_t v) {
    skw_tagged_t t;
    t.tag = SKW_TAG_INT;
    t.as.i = v;
    return t;
}
