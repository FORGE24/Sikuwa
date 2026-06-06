#ifndef SIKUWA_MODULE_H
#define SIKUWA_MODULE_H

#include "sikuwa/abi.h"

#ifdef __cplusplus
extern "C" {
#endif

/** DTSS slot tier string in manifest / export table. */
#define SKW_SLOT_S0 "S0"
#define SKW_SLOT_S1 "S1"
#define SKW_SLOT_S2 "S2"
#define SKW_SLOT_S3 "S3"

typedef struct skw_fn_entry {
    const char *symbol;   /* PIR SymbolRef, e.g. "add.add" */
    const char *slot;     /* SKW_SLOT_S0 .. S3 */
    void *fn;
} skw_fn_entry_t;

typedef struct skw_module {
    uint32_t abi_major;
    uint32_t abi_minor;
    const char *name;
    uint8_t source_hash[32];
    size_t fn_count;
    const skw_fn_entry_t *fns;
} skw_module_t;

#ifdef __cplusplus
}
#endif

#endif /* SIKUWA_MODULE_H */
