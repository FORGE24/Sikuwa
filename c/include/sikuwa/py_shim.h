#ifndef SIKUWA_PY_SHIM_H
#define SIKUWA_PY_SHIM_H

/**
 * Python embed helpers (Plan 5).
 * Build with `-DSKW_PYTHON_EMBED` and Python development headers.
 */
#ifdef SKW_PYTHON_EMBED
#define PY_SSIZE_T_CLEAN
#include <Python.h>
#include "sikuwa/abi.h"

#ifdef __cplusplus
extern "C" {
#endif

SKW_API int64_t skw_py_unbox_i64(PyObject *obj, skw_status_t *st);
SKW_API PyObject *skw_py_box_i64(int64_t v);

#ifdef __cplusplus
}
#endif

#endif /* SKW_PYTHON_EMBED */

#endif /* SIKUWA_PY_SHIM_H */
