/**
 * Minimal Python stdlib shims for static compilation (feb.py / dyn tier).
 */
#include "sikuwa/runtime.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

SKW_API int64_t skw_fn_ref(const char *sym) {
    (void)sym;
    return 0;
}

SKW_API int64_t skw_builtin_range(int64_t n) {
    if (n < 0) {
        return 0;
    }
    return n;
}

SKW_API void skw_builtin_print(const char *s) {
    if (s) {
        (void)fputs(s, stdout);
        (void)fputc('\n', stdout);
    }
}

SKW_API double skw_builtin_float(int64_t v) {
    return (double)v;
}

SKW_API double skw_builtin_float_str(const char *s) {
    if (!s) {
        return 0.0;
    }
    if (strcmp(s, "inf") == 0 || strcmp(s, "+inf") == 0) {
        return INFINITY;
    }
    if (strcmp(s, "-inf") == 0) {
        return -INFINITY;
    }
    return atof(s);
}

SKW_API const char *skw_builtin_str(int64_t v) {
    static char buf[32];
    (void)snprintf(buf, sizeof(buf), "%lld", (long long)v);
    return buf;
}

SKW_API const char *skw_py_joined_str(int64_t a, int64_t b) {
    static char buf[128];
    (void)snprintf(buf, sizeof(buf), "%lld %lld", (long long)a, (long long)b);
    return buf;
}

SKW_API int64_t skw_call_indirect_i64(int64_t callee, int64_t arg) {
    (void)callee;
    (void)arg;
    return 0;
}

SKW_API double skw_time_perf_counter(void) {
    return 0.0;
}

SKW_API void skw_sys_setrecursionlimit(int64_t n) {
    (void)n;
}
