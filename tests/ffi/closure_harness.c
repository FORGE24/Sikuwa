#include <stdint.h>
#include <stdio.h>

#include "plan3.h"

int main(void) {
    skw_plan3_make_adder_add_closure_t add5 = skw_plan3_make_adder(5);
    int64_t r = add5.fn(&add5.env, 10);
    if (r != 15) {
        fprintf(stderr, "closure add5(10) = %lld, expected 15\n", (long long)r);
        return 1;
    }

    skw_plan3_Point_t p;
    skw_plan3_Point___init__(&p, 3, 4);
    if (p.x != 3 || p.y != 4) {
        fprintf(stderr, "Point init failed: x=%lld y=%lld\n", (long long)p.x, (long long)p.y);
        return 1;
    }
    return 0;
}
