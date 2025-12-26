// sikuwa/cpp_cache/pysmartcache.cpp
// Python扩展模块，用于集成C++智能缓存系统

#include <Python.h>
#include "smart_cache.h"
#include <string>

// LRUCache类的Python包装器
static PyObject* py_lru_cache_new(PyObject* self, PyObject* args) {
    size_t max_size = 1000;
    if (!PyArg_ParseTuple(args, "|k", &max_size)) {
        return NULL;
    }
    
    LRUCache* cache = new LRUCache(max_size);
    return PyCapsule_New(cache, "LRUCache", NULL);
}

static void py_lru_cache_dealloc(PyObject* capsule) {
    LRUCache* cache = (LRUCache*)PyCapsule_GetPointer(capsule, "LRUCache");
    if (cache) {
        delete cache;
    }
}

static PyObject* py_lru_cache_put(PyObject* self, PyObject* args) {
    PyObject* capsule;
    const char* key;
    const char* value;
    if (!PyArg_ParseTuple(args, "Oss", &capsule, &key, &value)) {
        return NULL;
    }
    
    LRUCache* cache = (LRUCache*)PyCapsule_GetPointer(capsule, "LRUCache");
    if (!cache) {
        PyErr_SetString(PyExc_RuntimeError, "Invalid LRUCache pointer");
        return NULL;
    }
    
    bool result = cache->put(key, value);
    return PyBool_FromLong(result);
}

static PyObject* py_lru_cache_get(PyObject* self, PyObject* args) {
    PyObject* capsule;
    const char* key;
    if (!PyArg_ParseTuple(args, "Os", &capsule, &key)) {
        return NULL;
    }
    
    LRUCache* cache = (LRUCache*)PyCapsule_GetPointer(capsule, "LRUCache");
    if (!cache) {
        PyErr_SetString(PyExc_RuntimeError, "Invalid LRUCache pointer");
        return NULL;
    }
    
    std::string result = cache->get(key);
    return PyUnicode_FromString(result.c_str());
}

// 定义Python模块的方法表
static PyMethodDef pysmartcache_methods[] = {
    {"lru_cache_new", py_lru_cache_new, METH_VARARGS, "Create a new LRUCache"},
    {"lru_cache_put", py_lru_cache_put, METH_VARARGS, "Put a key-value pair into LRUCache"},
    {"lru_cache_get", py_lru_cache_get, METH_VARARGS, "Get a value from LRUCache"},
    {NULL, NULL, 0, NULL}  // Sentinel
};

// 定义Python模块的初始化函数
static struct PyModuleDef pysmartcache_module = {
    PyModuleDef_HEAD_INIT,
    "pysmartcache",  // 模块名称
    "C++ Smart Cache Python Extension",  // 模块文档
    -1,  // 模块状态大小
    pysmartcache_methods  // 模块方法表
};

PyMODINIT_FUNC PyInit_pysmartcache(void) {
    return PyModule_Create(&pysmartcache_module);
}