/* Microbenchmark — calls compiled Sikuwa-C symbols in a tight loop. */
#include <stdint.h>
#include <stdio.h>
#if defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#else
#include <time.h>
#endif

#include "add.h"

#if defined(SKW_BENCH_SUM_RANGE)
#include "sum_range.h"
#endif

static double now_sec(void) {
#if defined(_WIN32)
    static LARGE_INTEGER freq = {0};
    LARGE_INTEGER count;
    if (freq.QuadPart == 0) {
        QueryPerformanceFrequency(&freq);
    }
    QueryPerformanceCounter(&count);
    return (double)count.QuadPart / (double)freq.QuadPart;
#else
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
#endif
}

static int bench_add(int64_t iterations) {
    volatile int64_t sink = 0;
    double t0 = now_sec();
    for (int64_t i = 0; i < iterations; i++) {
        sink += skw_add_add(i & 0xff, (i >> 8) & 0xff);
    }
    double elapsed = now_sec() - t0;
    if (sink == 0) {
        fprintf(stderr, "add bench: unexpected zero sink\n");
        return 1;
    }
    double ns = elapsed * 1e9 / (double)iterations;
    printf("  add: %lld calls in %.3f ms (%.1f ns/call) sink=%lld\n",
           (long long)iterations, elapsed * 1000.0, ns, (long long)sink);
    return 0;
}

#if defined(SKW_BENCH_SUM_RANGE)
static int bench_sum_range(int64_t outer, int64_t n) {
    volatile int64_t sink = 0;
    double t0 = now_sec();
    for (int64_t i = 0; i < outer; i++) {
        sink += skw_sum_range_sum_range(n);
    }
    double elapsed = now_sec() - t0;
    int64_t expect = n * (n - 1) / 2;
    if (sink != expect * outer) {
        fprintf(stderr, "sum_range bench: sink=%lld expect=%lld\n",
                (long long)sink, (long long)(expect * outer));
        return 1;
    }
    printf("  sum_range(n=%lld) x %lld: %.3f ms total (%.3f us/iter)\n",
           (long long)n, (long long)outer, elapsed * 1000.0,
           elapsed * 1e6 / (double)outer);
    return 0;
}
#endif

int main(void) {
    printf("=== Sikuwa native runtime microbench ===\n");
    if (bench_add(10 * 1000 * 1000) != 0) {
        return 1;
    }
#if defined(SKW_BENCH_SUM_RANGE)
    if (bench_sum_range(5000, 500) != 0) {
        return 1;
    }
#endif
    return 0;
}
