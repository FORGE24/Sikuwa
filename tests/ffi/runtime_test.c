#define SKW_BUILDING_MODULE
#include "sikuwa/runtime.h"

#include <assert.h>

int main(void) {
    skw_value_t *v = skw_value_from_i64(42);
    skw_status_t st = SKW_OK;
    assert(skw_value_to_i64(v, &st) == 42);
    assert(st == SKW_OK);
    skw_value_release(v);
    return 0;
}
