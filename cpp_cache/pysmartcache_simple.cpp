// sikuwa/cpp_cache/pysmartcache_simple.cpp
// 简化版智能缓存系统的Python扩展

#include <Python.h>
#include "smart_cache_simple.h"

// 为LRUCache和BuildCache创建Python对象类型

// 简单的Python扩展，仅提供基本功能

// LRUCache相关函数

static PyObject* py_lru_cache_new(PyObject* self, PyObject* args) {
    int max_size = 1000;
    if (!PyArg_ParseTuple(args, "|i", &max_size)) {
        return nullptr;
    }
    
    LRUCache* cache = new LRUCache(max_size);
    return PyCapsule_New(cache, "LRUCache", [](PyObject* capsule) {
        LRUCache* cache = static_cast<LRUCache*>(PyCapsule_GetPointer(capsule, "LRUCache"));
        delete cache;
    });
}

static PyObject* py_lru_cache_put(PyObject* self, PyObject* args) {
    PyObject* capsule;
    const char* key;
    const char* value;
    if (!PyArg_ParseTuple(args, "Os|s", &capsule, &key, &value)) {
        return nullptr;
    }
    
    LRUCache* cache = static_cast<LRUCache*>(PyCapsule_GetPointer(capsule, "LRUCache"));
    if (cache == nullptr) {
        return nullptr;
    }
    
    bool success = cache->put(key, value ? value : "");
    return PyBool_FromLong(success);
}

static PyObject* py_lru_cache_get(PyObject* self, PyObject* args) {
    PyObject* capsule;
    const char* key;
    if (!PyArg_ParseTuple(args, "Os", &capsule, &key)) {
        return nullptr;
    }
    
    LRUCache* cache = static_cast<LRUCache*>(PyCapsule_GetPointer(capsule, "LRUCache"));
    if (cache == nullptr) {
        return nullptr;
    }
    
    std::string result = cache->get(key);
    if (result.empty()) {
        Py_RETURN_NONE;
    }
    
    return PyUnicode_FromString(result.c_str());
}

// 构建缓存相关函数

static PyObject* py_build_cache_new(PyObject* self, PyObject* args) {
    const char* cache_dir = ".cache";
    int max_size = 1000000000;
    
    if (!PyArg_ParseTuple(args, "|si", &cache_dir, &max_size)) {
        return nullptr;
    }
    
    BuildCache* cache = new BuildCache(cache_dir, max_size);
    return PyCapsule_New(cache, "BuildCache", [](PyObject* capsule) {
        BuildCache* cache = static_cast<BuildCache*>(PyCapsule_GetPointer(capsule, "BuildCache"));
        delete cache;
    });
}

static PyObject* py_cache_build_result(PyObject* self, PyObject* args) {
    PyObject* capsule;
    const char* target;
    const char* command;
    PyObject* dependencies_obj;
    const char* result;
    
    if (!PyArg_ParseTuple(args, "OsOs|s", &capsule, &target, &command, &dependencies_obj, &result)) {
        return nullptr;
    }
    
    BuildCache* cache = static_cast<BuildCache*>(PyCapsule_GetPointer(capsule, "BuildCache"));
    if (cache == nullptr) {
        return nullptr;
    }
    
    // Convert Python list to C++ vector
    std::vector<std::string> dependencies;
    if (!PyList_Check(dependencies_obj)) {
        PyErr_SetString(PyExc_TypeError, "dependencies must be a list");
        return nullptr;
    }
    
    Py_ssize_t len = PyList_Size(dependencies_obj);
    for (Py_ssize_t i = 0; i < len; i++) {
        PyObject* item = PyList_GetItem(dependencies_obj, i);
        if (!PyUnicode_Check(item)) {
            PyErr_SetString(PyExc_TypeError, "dependencies must contain strings");
            return nullptr;
        }
        dependencies.push_back(PyUnicode_AsUTF8(item));
    }
    
    bool success = cache->cache_build_result(target, command, dependencies, result ? result : "");
    return PyBool_FromLong(success);
}

static PyObject* py_get_cached_build_result(PyObject* self, PyObject* args) {
    PyObject* capsule;
    const char* target;
    const char* command;
    PyObject* dependencies_obj;
    
    if (!PyArg_ParseTuple(args, "OsOs", &capsule, &target, &command, &dependencies_obj)) {
        return nullptr;
    }
    
    BuildCache* cache = static_cast<BuildCache*>(PyCapsule_GetPointer(capsule, "BuildCache"));
    if (cache == nullptr) {
        return nullptr;
    }
    
    // Convert Python list to C++ vector
    std::vector<std::string> dependencies;
    if (!PyList_Check(dependencies_obj)) {
        PyErr_SetString(PyExc_TypeError, "dependencies must be a list");
        return nullptr;
    }
    
    Py_ssize_t len = PyList_Size(dependencies_obj);
    for (Py_ssize_t i = 0; i < len; i++) {
        PyObject* item = PyList_GetItem(dependencies_obj, i);
        if (!PyUnicode_Check(item)) {
            PyErr_SetString(PyExc_TypeError, "dependencies must contain strings");
            return nullptr;
        }
        dependencies.push_back(PyUnicode_AsUTF8(item));
    }
    
    std::string cached_result = cache->get_cached_build_result(target, command, dependencies);
    if (cached_result.empty()) {
        Py_RETURN_NONE;
    }
    
    return PyUnicode_FromString(cached_result.c_str());
}

static PyObject* py_needs_rebuild(PyObject* self, PyObject* args) {
    PyObject* capsule;
    const char* target;
    const char* command;
    PyObject* dependencies_obj;
    
    if (!PyArg_ParseTuple(args, "OsOs", &capsule, &target, &command, &dependencies_obj)) {
        return nullptr;
    }
    
    BuildCache* cache = static_cast<BuildCache*>(PyCapsule_GetPointer(capsule, "BuildCache"));
    if (cache == nullptr) {
        return nullptr;
    }
    
    // Convert Python list to C++ vector
    std::vector<std::string> dependencies;
    if (!PyList_Check(dependencies_obj)) {
        PyErr_SetString(PyExc_TypeError, "dependencies must be a list");
        return nullptr;
    }
    
    Py_ssize_t len = PyList_Size(dependencies_obj);
    for (Py_ssize_t i = 0; i < len; i++) {
        PyObject* item = PyList_GetItem(dependencies_obj, i);
        if (!PyUnicode_Check(item)) {
            PyErr_SetString(PyExc_TypeError, "dependencies must contain strings");
            return nullptr;
        }
        dependencies.push_back(PyUnicode_AsUTF8(item));
    }
    
    bool needs = cache->needs_rebuild(target, command, dependencies);
    return PyBool_FromLong(needs);
}

// 模块方法定义
static PyMethodDef PySmartCacheMethods[] = {
    // LRUCache方法
    {"lru_cache_new", py_lru_cache_new, METH_VARARGS, "Create a new LRUCache instance"},
    {"lru_cache_put", py_lru_cache_put, METH_VARARGS, "Put a key-value pair into the LRUCache"},
    {"lru_cache_get", py_lru_cache_get, METH_VARARGS, "Get a value from the LRUCache"},
    
    // BuildCache方法
    {"build_cache_new", py_build_cache_new, METH_VARARGS, "Create a new BuildCache instance"},
    {"cache_build_result", py_cache_build_result, METH_VARARGS, "Cache a build result"},
    {"get_cached_build_result", py_get_cached_build_result, METH_VARARGS, "Get a cached build result"},
    {"needs_rebuild", py_needs_rebuild, METH_VARARGS, "Check if a build needs to be redone"},
    
    {nullptr, nullptr, 0, nullptr}
};

// 模块定义
static struct PyModuleDef pysmartcachemodule = {
    PyModuleDef_HEAD_INIT,
    "pysmartcache",
    "Sikuwa Smart Cache Python Extension",
    -1,
    PySmartCacheMethods
};

// 模块初始化函数
PyMODINIT_FUNC PyInit_pysmartcache(void) {
    return PyModule_Create(&pysmartcachemodule);
}
