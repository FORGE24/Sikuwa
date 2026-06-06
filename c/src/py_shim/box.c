#define SKW_BUILDING_MODULE
#define SKW_PYTHON_EMBED
#include "sikuwa/py_shim.h"

SKW_API int64_t skw_py_unbox_i64(PyObject *obj, skw_status_t *st) {
    if (st) {
        *st = SKW_OK;
    }
    if (!obj || !PyLong_Check(obj)) {
        if (st) {
            *st = SKW_ERR_TYPE;
        }
        return 0;
    }
    return (int64_t)PyLong_AsLongLong(obj);
}

SKW_API PyObject *skw_py_box_i64(int64_t v) {
    return PyLong_FromLongLong(v);
}
