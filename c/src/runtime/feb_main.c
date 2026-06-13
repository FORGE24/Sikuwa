/**
 * Native feb.py entry when IR `main` is still dyn-stub.
 * Calls S0-compiled fib helpers; iterative/matrix inlined here until codegen catches up.
 */
#include "sikuwa/runtime.h"

#include <stdio.h>
#include <time.h>

extern SKW_API int64_t skw_feb_fib_recursive(int64_t n);
extern SKW_API int64_t skw_feb_fib_memo(int64_t n);

static int64_t skw_feb_fib_iterative_native(int64_t n) {
    int64_t a = 0;
    int64_t b = 1;
    for (int64_t i = 0; i < n; i++) {
        int64_t next = a + b;
        a = b;
        b = next;
    }
    return a;
}

static int64_t skw_feb_fib_matrix_native(int64_t n) {
    int64_t m00 = 1, m01 = 1, m10 = 1, m11 = 0;
    int64_t r00 = 1, r01 = 0, r10 = 0, r11 = 1;
    int64_t exp = n > 0 ? n - 1 : 0;
    if (n == 0) {
        return 0;
    }
    while (exp > 0) {
        if (exp & 1) {
            int64_t t00 = r00 * m00 + r01 * m10;
            int64_t t01 = r00 * m01 + r01 * m11;
            int64_t t10 = r10 * m00 + r11 * m10;
            int64_t t11 = r10 * m01 + r11 * m11;
            r00 = t00;
            r01 = t01;
            r10 = t10;
            r11 = t11;
        }
        {
            int64_t t00 = m00 * m00 + m01 * m10;
            int64_t t01 = m00 * m01 + m01 * m11;
            int64_t t10 = m10 * m00 + m11 * m10;
            int64_t t11 = m10 * m01 + m11 * m11;
            m00 = t00;
            m01 = t01;
            m10 = t10;
            m11 = t11;
        }
        exp >>= 1;
    }
    return r00;
}

static double skw_now_sec(void) {
    return (double)clock() / (double)CLOCKS_PER_SEC;
}

typedef int64_t (*skw_fib_fn)(int64_t);

static double skw_benchmark_fn(skw_fib_fn fn, int64_t n, int64_t repeat) {
    (void)fn(n);
    double total = 0.0;
    for (int64_t i = 0; i < repeat; i++) {
        double start = skw_now_sec();
        (void)fn(n);
        total += skw_now_sec() - start;
    }
    return total / (double)repeat;
}

static void skw_print_line(const char *s) {
    (void)puts(s);
    (void)fflush(stdout);
}

SKW_API void skw_feb_main_native(void) {
    static const struct {
        int64_t n;
        int64_t repeat;
    } cases[] = {
        {30, 1},
        {35, 1},
        {100, 100},
        {1000, 100},
        {10000, 10},
    };

    skw_print_line("======================================================================");
    skw_print_line("Fibonacci benchmark (seconds)");
    skw_print_line("======================================================================");

    for (size_t ci = 0; ci < sizeof(cases) / sizeof(cases[0]); ci++) {
        int64_t n = cases[ci].n;
        int64_t repeat = cases[ci].repeat;
        char hdr[128];
        (void)snprintf(hdr, sizeof(hdr), "\n>>> n = %lld (repeat %lld, average)", (long long)n, (long long)repeat);
        skw_print_line(hdr);
        skw_print_line("----------------------------------------");

        struct {
            const char *name;
            skw_fib_fn fn;
            int64_t skip_above_n;
        } funcs[] = {
            {"recursive(plain)", skw_feb_fib_recursive, 35},
            /* @lru_cache not lowered — same cost as plain recursive */
            {"recursive(memo)", skw_feb_fib_memo, 35},
            {"iterative", skw_feb_fib_iterative_native, 0},
            {"matrix pow", skw_feb_fib_matrix_native, 0},
        };

        for (size_t fi = 0; fi < sizeof(funcs) / sizeof(funcs[0]); fi++) {
            char line[160];
            if (funcs[fi].skip_above_n > 0 && n > funcs[fi].skip_above_n) {
                (void)snprintf(line, sizeof(line), "  %-16s: skip (n too large)", funcs[fi].name);
                skw_print_line(line);
                continue;
            }
            double t = skw_benchmark_fn(funcs[fi].fn, n, repeat);
            (void)snprintf(line, sizeof(line), "  %-16s: %.6f sec", funcs[fi].name, t);
            skw_print_line(line);
        }
    }
}
