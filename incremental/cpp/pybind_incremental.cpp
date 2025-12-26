// sikuwa/incremental/cpp/pybind_incremental.cpp
// Python 绑定 - 使用 pybind11

#include <pybind11/pybind11.h>
#include <pybind11/stl.h>
#include "incremental_core.h"

namespace py = pybind11;
using namespace sikuwa::incremental;

PYBIND11_MODULE(incremental_engine, m) {
    m.doc() = "Sikuwa 减量编译引擎 - 指哪编哪";
    
    // 枚举类型
    py::enum_<UnitType>(m, "UnitType")
        .value("LINE", UnitType::LINE)
        .value("STATEMENT", UnitType::STATEMENT)
        .value("FUNCTION", UnitType::FUNCTION)
        .value("CLASS", UnitType::CLASS)
        .value("MODULE", UnitType::MODULE)
        .value("IMPORT", UnitType::IMPORT)
        .value("DECORATOR", UnitType::DECORATOR)
        .value("BLOCK", UnitType::BLOCK);
    
    py::enum_<UnitState>(m, "UnitState")
        .value("UNKNOWN", UnitState::UNKNOWN)
        .value("UNCHANGED", UnitState::UNCHANGED)
        .value("MODIFIED", UnitState::MODIFIED)
        .value("ADDED", UnitState::ADDED)
        .value("DELETED", UnitState::DELETED)
        .value("AFFECTED", UnitState::AFFECTED);
    
    // CompilationUnit
    py::class_<CompilationUnit>(m, "CompilationUnit")
        .def(py::init<>())
        .def_readwrite("id", &CompilationUnit::id)
        .def_readwrite("file_path", &CompilationUnit::file_path)
        .def_readwrite("start_line", &CompilationUnit::start_line)
        .def_readwrite("end_line", &CompilationUnit::end_line)
        .def_readwrite("type", &CompilationUnit::type)
        .def_readwrite("name", &CompilationUnit::name)
        .def_readwrite("content_hash", &CompilationUnit::content_hash)
        .def_readwrite("dependencies", &CompilationUnit::dependencies)
        .def_readwrite("dependents", &CompilationUnit::dependents)
        .def_readwrite("state", &CompilationUnit::state)
        .def_readwrite("cached_output", &CompilationUnit::cached_output)
        .def_readwrite("cache_valid", &CompilationUnit::cache_valid);
    
    // ChangeRecord
    py::class_<ChangeRecord>(m, "ChangeRecord")
        .def(py::init<>())
        .def_readwrite("unit_id", &ChangeRecord::unit_id)
        .def_readwrite("change_type", &ChangeRecord::change_type)
        .def_readwrite("old_start_line", &ChangeRecord::old_start_line)
        .def_readwrite("old_end_line", &ChangeRecord::old_end_line)
        .def_readwrite("new_start_line", &ChangeRecord::new_start_line)
        .def_readwrite("new_end_line", &ChangeRecord::new_end_line)
        .def_readwrite("reason", &ChangeRecord::reason);
    
    // Snapshot
    py::class_<Snapshot>(m, "Snapshot")
        .def(py::init<>())
        .def_readwrite("file_path", &Snapshot::file_path)
        .def_readwrite("content_hash", &Snapshot::content_hash)
        .def_readwrite("line_hashes", &Snapshot::line_hashes)
        .def_readwrite("timestamp", &Snapshot::timestamp);
    
    // UnitManager
    py::class_<UnitManager>(m, "UnitManager")
        .def(py::init<>())
        .def("add_unit", &UnitManager::add_unit)
        .def("update_unit", &UnitManager::update_unit)
        .def("remove_unit", &UnitManager::remove_unit)
        .def("get_unit", py::overload_cast<const std::string&>(&UnitManager::get_unit),
             py::return_value_policy::reference)
        .def("get_units_by_file", &UnitManager::get_units_by_file,
             py::return_value_policy::reference)
        .def("get_units_in_range", &UnitManager::get_units_in_range,
             py::return_value_policy::reference)
        .def("add_dependency", &UnitManager::add_dependency)
        .def("remove_dependency", &UnitManager::remove_dependency)
        .def("get_dependencies", &UnitManager::get_dependencies)
        .def("get_dependents", &UnitManager::get_dependents)
        .def("get_affected_units", &UnitManager::get_affected_units)
        .def("size", &UnitManager::size)
        .def("clear", &UnitManager::clear)
        .def("serialize", &UnitManager::serialize)
        .def("deserialize", &UnitManager::deserialize);
    
    // ChangeDetector
    py::class_<ChangeDetector>(m, "ChangeDetector")
        .def(py::init<>())
        .def("create_snapshot", &ChangeDetector::create_snapshot)
        .def("detect_changes", &ChangeDetector::detect_changes)
        .def("get_changed_lines", &ChangeDetector::get_changed_lines)
        .def_static("compute_hash", &ChangeDetector::compute_hash)
        .def_static("compute_line_hash", &ChangeDetector::compute_line_hash);
    
    // CompilationCache
    py::class_<CompilationCache>(m, "CompilationCache")
        .def(py::init<const std::string&>())
        .def("has", &CompilationCache::has)
        .def("get", &CompilationCache::get)
        .def("put", &CompilationCache::put)
        .def("invalidate", &CompilationCache::invalidate)
        .def("invalidate_all", &CompilationCache::invalidate_all)
        .def("is_valid", &CompilationCache::is_valid)
        .def("save", &CompilationCache::save)
        .def("load", &CompilationCache::load)
        .def("size", &CompilationCache::size)
        .def("hit_count", &CompilationCache::hit_count)
        .def("miss_count", &CompilationCache::miss_count);
    
    // IncrementalEngine
    py::class_<IncrementalEngine>(m, "IncrementalEngine")
        .def(py::init<const std::string&>())
        .def("register_units", &IncrementalEngine::register_units)
        .def("update_source", &IncrementalEngine::update_source)
        .def("get_units_to_compile", &IncrementalEngine::get_units_to_compile)
        .def("mark_compiled", &IncrementalEngine::mark_compiled)
        .def("get_combined_output", &IncrementalEngine::get_combined_output)
        .def("save_state", &IncrementalEngine::save_state)
        .def("load_state", &IncrementalEngine::load_state);
    
    // 工具函数
    m.def("generate_unit_id", &generate_unit_id);
    m.def("compute_hash", &ChangeDetector::compute_hash);
    m.def("split_lines", &split_lines);
    m.def("join_lines", &join_lines);
}
