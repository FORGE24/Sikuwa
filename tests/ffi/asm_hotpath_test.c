#ifndef SKW_BUILDING_MODULE
#define SKW_BUILDING_MODULE
#endif
#include "sikuwa/hotpath.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

static void expect_hash_match(const char *label, const void *data, size_t len)
{
    uint64_t fast = skw_hash64(data, len);
    uint64_t ref = skw_hash64_c(data, len);
    if (fast != ref) {
        fprintf(stderr, "%s: asm=%llu c=%llu\n",
                label, (unsigned long long)fast, (unsigned long long)ref);
        assert(0);
    }
}

int main(void)
{
    const char *msg = "sikuwa-hotpath";
    expect_hash_match("msg", msg, strlen(msg));

    const uint8_t blob[] = {1, 2, 3, 4, 5, 6, 7, 8, 9};
    expect_hash_match("blob", blob, sizeof blob);

    skw_status_t st = SKW_OK;
    assert(skw_i64_add_checked(40, 2, &st) == 42);
    assert(st == SKW_OK);
    assert(skw_i64_add_checked_c(40, 2, &st) == 42);

    (void)skw_i64_add_checked(INT64_MAX, 1, &st);
    assert(st == SKW_ERR_RANGE);

    skw_tagged_t t = skw_tagged_from_i64(99);
    assert(skw_tagged_as_i64(&t, &st) == 99);
    assert(st == SKW_OK);
    assert(skw_tagged_as_i64_c(&t, &st) == 99);

    t.tag = SKW_TAG_FLOAT;
    assert(skw_tagged_as_i64(&t, &st) == 0);
    assert(st == SKW_ERR_TYPE);

    puts("asm hotpath ok");
    return 0;
}
