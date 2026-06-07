#include <stdint.h>
#include <stdio.h>

#include "add.h"
#include "plan5_caller.h"

int main(void) {
    if (skw_add_add(1, 2) != 3) {
        fprintf(stderr, "skw_add_add(1,2) failed\n");
        return 1;
    }
    int64_t r = skw_plan5_caller_twice(3, 4);
    if (r != 7) {
        fprintf(stderr, "skw_plan5_caller_twice(3,4) = %lld, expected 7\n", (long long)r);
        return 1;
    }
    return 0;
}
