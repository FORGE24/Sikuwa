// pysmartcache_minimal.cpp
// Minimal Python extension for smart cache system

#include <Python.h>
#include "smart_cache_minimal.h"

// LRUCache functions

static PyObject* py_lru_cache_new(PyObject* self, PyObject* args) {
    int max_size = 1000;
    if (!PyArg_ParseTuple(args, "|i", &max_size)) {
        return NULL;
    }
    
    LRUCache* cache = new LRUCache(max_size);
    return PyCapsule_New(cache, "LRUCache", NULL);
}

static PyObject* py_lru_cache_put(PyObject* self, PyObject* args) {
    PyObject* capsule;
    const char* key;
    const char* value;
    if (!PyArg_ParseTuple(args, "Os|s", &capsule, &key, &value)) {
        return NULL;
    }
    
    LRUCache* cache = (LRUCache*)PyCapsule_GetPointer(capsule, "LRUCache");
    if (cache == NULL) {
        return NULL;
    }
    
    const char* val = value ? value : "";
    bool success = cache->put(key, val);
    return PyBool_FromLong(success);
}

static PyObject* py_lru_cache_get(PyObject* self, PyObject* args) {
    PyObject* capsule;
    const char* key;
    if (!PyArg_ParseTuple(args, "Os", &capsule, &key)) {
        return NULL;
    }
    
    LRUCache* cache = (LRUCache*)PyCapsule_GetPointer(capsule, "LRUCache");
    if (cache == NULL) {
        return NULL;
    }
    
    std::string result = cache->get(key);
    if (result.empty()) {
        Py_RETURN_NONE;
    }
    
    return PyUnicode_FromString(result.c_str());
}

// BuildCache functions

static PyObject* py_build_cache_new(PyObject* self, PyObject* args) {
    const char* cache_dir = ".cache";
    if (!PyArg_ParseTuple(args, "|s", &cache_dir)) {
        return NULL;
    }
    
    BuildCache* cache = new BuildCache(cache_dir);
    return PyCapsule_New(cache, "BuildCache", NULL);
}

static PyObject* py_build_cache_result(PyObject* self, PyObject* args) {
    PyObject* capsule;
    const char* target;
    const char* command;
    const char* result;
    if (!PyArg_ParseTuple(args, "Oss|s", &capsule, &target, &command, &result)) {
        return NULL;
    }
    
    BuildCache* cache = (BuildCache*)PyCapsule_GetPointer(capsule, "BuildCache");
    if (cache == NULL) {
        return NULL;
    }
    
    const char* res = result ? result : "";
    bool success = cache->cache_result(target, command, res);
    return PyBool_FromLong(success);
}

static PyObject* py_build_cache_get(PyObject* self, PyObject* args) {
    PyObject* capsule;
    const char* target;
    const char* command;
    if (!PyArg_ParseTuple(args, "Oss", &capsule, &target, &command)) {
        return NULL;
    }
    
    BuildCache* cache = (BuildCache*)PyCapsule_GetPointer(capsule, "BuildCache");
    if (cache == NULL) {
        return NULL;
    }
    
    std::string res = cache->get_result(target, command);
    if (res.empty()) {
        Py_RETURN_NONE;
    }
    
    return PyUnicode_FromString(res.c_str());
}

static PyObject* py_build_cache_needs_rebuild(PyObject* self, PyObject* args) {
    PyObject* capsule;
    const char* target;
    const char* command;
    if (!PyArg_ParseTuple(args, "Oss", &capsule, &target, &command)) {
        return NULL;
    }
    
    BuildCache* cache = (BuildCache*)PyCapsule_GetPointer(capsule, "BuildCache");
    if (cache == NULL) {
        return NULL;
    }
    
    bool needs = cache->needs_rebuild(target, command);
    return PyBool_FromLong(needs);
}

// Method definitions
static PyMethodDef PySmartCacheMethods[] = {
    {"lru_cache_new", py_lru_cache_new, METH_VARARGS, "Create LRUCache"},
    {"lru_cache_put", py_lru_cache_put, METH_VARARGS, "Put to LRUCache"},
    {"lru_cache_get", py_lru_cache_get, METH_VARARGS, "Get from LRUCache"},
    {"build_cache_new", py_build_cache_new, METH_VARARGS, "Create BuildCache"},
    {"build_cache_result", py_build_cache_result, METH_VARARGS, "Cache build result"},
    {"build_cache_get", py_build_cache_get, METH_VARARGS, "Get cached build result"},
    {"build_cache_needs_rebuild", py_build_cache_needs_rebuild, METH_VARARGS, "Check if rebuild needed"},
    {NULL, NULL, 0, NULL}
};

// Module definition
static struct PyModuleDef pysmartcachemodule = {
    PyModuleDef_HEAD_INIT,
    "pysmartcache",
    "Sikuwa Smart Cache Python Extension",
    -1,
    PySmartCacheMethods
};

// Module initialization
PyMODINIT_FUNC PyInit_pysmartcache(void) {
    return PyModule_Create(&pysmartcachemodule);
}
