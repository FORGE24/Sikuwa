#include <stdint.h>
#include <stdio.h>

#include "add.h"

int main(void) {
    int64_t r = skw_add_add(2, 3);
    if (r != 5) {
        fprintf(stderr, "skw_add_add(2,3) = %lld, expected 5\n", (long long)r);
        return 1;
    }
    return 0;
}
