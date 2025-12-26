// sikuwa/incremental/cpp/incremental_core.cpp
// 减量编译核心 - C++ 实现

#include "incremental_core.h"
#include <fstream>
#include <sstream>
#include <algorithm>
#include <cstring>
#include <iomanip>

namespace sikuwa {
namespace incremental {

// ============================================================================
// 工具函数实现
// ============================================================================

// 简单的哈希函数 (FNV-1a)
static uint64_t fnv1a_hash(const char* data, size_t len) {
    uint64_t hash = 14695981039346656037ULL;
    for (size_t i = 0; i < len; ++i) {
        hash ^= static_cast<uint64_t>(data[i]);
        hash *= 1099511628211ULL;
    }
    return hash;
}

std::string generate_unit_id(const std::string& file_path, int start_line,
                            int end_line, const std::string& content_hash) {
    std::ostringstream oss;
    oss << file_path << ":" << start_line << ":" << end_line << ":" 
        << content_hash.substr(0, 8);
    return oss.str();
}

int64_t current_timestamp() {
    return std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::system_clock::now().time_since_epoch()
    ).count();
}

std::string read_file(const std::string& path) {
    std::ifstream file(path);
    if (!file.is_open()) return "";
    std::ostringstream oss;
    oss << file.rdbuf();
    return oss.str();
}

void write_file(const std::string& path, const std::string& content) {
    std::ofstream file(path);
    if (file.is_open()) {
        file << content;
    }
}

std::vector<std::string> split_lines(const std::string& content) {
    std::vector<std::string> lines;
    std::istringstream iss(content);
    std::string line;
    while (std::getline(iss, line)) {
        lines.push_back(line);
    }
    return lines;
}

std::string join_lines(const std::vector<std::string>& lines) {
    std::ostringstream oss;
    for (size_t i = 0; i < lines.size(); ++i) {
        if (i > 0) oss << "\n";
        oss << lines[i];
    }
    return oss.str();
}

// ============================================================================
// UnitManager 实现
// ============================================================================

UnitManager::UnitManager() {}
UnitManager::~UnitManager() {}

void UnitManager::add_unit(const CompilationUnit& unit) {
    units_[unit.id] = unit;
    file_units_[unit.file_path].push_back(unit.id);
}

void UnitManager::update_unit(const std::string& id, const CompilationUnit& unit) {
    if (units_.find(id) != units_.end()) {
        units_[id] = unit;
    }
}

void UnitManager::remove_unit(const std::string& id) {
    auto it = units_.find(id);
    if (it != units_.end()) {
        // 从文件索引中移除
        auto& file_ids = file_units_[it->second.file_path];
        file_ids.erase(std::remove(file_ids.begin(), file_ids.end(), id), file_ids.end());
        
        // 从依赖关系中移除
        for (const auto& dep_id : it->second.dependencies) {
            auto dep_it = units_.find(dep_id);
            if (dep_it != units_.end()) {
                auto& dependents = dep_it->second.dependents;
                dependents.erase(std::remove(dependents.begin(), dependents.end(), id), 
                               dependents.end());
            }
        }
        
        units_.erase(it);
    }
}

CompilationUnit* UnitManager::get_unit(const std::string& id) {
    auto it = units_.find(id);
    return it != units_.end() ? &it->second : nullptr;
}

const CompilationUnit* UnitManager::get_unit(const std::string& id) const {
    auto it = units_.find(id);
    return it != units_.end() ? &it->second : nullptr;
}

std::vector<CompilationUnit*> UnitManager::get_units_by_file(const std::string& file_path) {
    std::vector<CompilationUnit*> result;
    auto it = file_units_.find(file_path);
    if (it != file_units_.end()) {
        for (const auto& id : it->second) {
            if (auto* unit = get_unit(id)) {
                result.push_back(unit);
            }
        }
    }
    // 按行号排序
    std::sort(result.begin(), result.end(), 
              [](const CompilationUnit* a, const CompilationUnit* b) {
                  return a->start_line < b->start_line;
              });
    return result;
}

std::vector<CompilationUnit*> UnitManager::get_units_in_range(
    const std::string& file_path, int start, int end) {
    std::vector<CompilationUnit*> result;
    auto units = get_units_by_file(file_path);
    for (auto* unit : units) {
        // 检查是否有交集
        if (unit->start_line <= end && unit->end_line >= start) {
            result.push_back(unit);
        }
    }
    return result;
}

void UnitManager::add_dependency(const std::string& from_id, const std::string& to_id) {
    auto* from_unit = get_unit(from_id);
    auto* to_unit = get_unit(to_id);
    
    if (from_unit && to_unit) {
        // from 依赖 to
        if (std::find(from_unit->dependencies.begin(), from_unit->dependencies.end(), to_id)
            == from_unit->dependencies.end()) {
            from_unit->dependencies.push_back(to_id);
        }
        // to 被 from 依赖
        if (std::find(to_unit->dependents.begin(), to_unit->dependents.end(), from_id)
            == to_unit->dependents.end()) {
            to_unit->dependents.push_back(from_id);
        }
    }
}

void UnitManager::remove_dependency(const std::string& from_id, const std::string& to_id) {
    auto* from_unit = get_unit(from_id);
    auto* to_unit = get_unit(to_id);
    
    if (from_unit) {
        auto& deps = from_unit->dependencies;
        deps.erase(std::remove(deps.begin(), deps.end(), to_id), deps.end());
    }
    if (to_unit) {
        auto& dependents = to_unit->dependents;
        dependents.erase(std::remove(dependents.begin(), dependents.end(), from_id), 
                        dependents.end());
    }
}

std::vector<std::string> UnitManager::get_dependencies(const std::string& id) const {
    const auto* unit = get_unit(id);
    return unit ? unit->dependencies : std::vector<std::string>{};
}

std::vector<std::string> UnitManager::get_dependents(const std::string& id) const {
    const auto* unit = get_unit(id);
    return unit ? unit->dependents : std::vector<std::string>{};
}

void UnitManager::collect_affected_recursive(const std::string& id,
                                             std::unordered_set<std::string>& visited) const {
    if (visited.count(id)) return;
    visited.insert(id);
    
    const auto* unit = get_unit(id);
    if (!unit) return;
    
    // 递归收集所有依赖此单元的单元
    for (const auto& dependent_id : unit->dependents) {
        collect_affected_recursive(dependent_id, visited);
    }
}

std::vector<std::string> UnitManager::get_affected_units(const std::string& changed_id) const {
    std::unordered_set<std::string> visited;
    collect_affected_recursive(changed_id, visited);
    visited.erase(changed_id);  // 移除自身
    return std::vector<std::string>(visited.begin(), visited.end());
}

void UnitManager::for_each(std::function<void(CompilationUnit&)> callback) {
    for (auto& pair : units_) {
        callback(pair.second);
    }
}

void UnitManager::clear() {
    units_.clear();
    file_units_.clear();
}

std::string UnitManager::serialize() const {
    std::ostringstream oss;
    oss << units_.size() << "\n";
    for (const auto& pair : units_) {
        const auto& u = pair.second;
        oss << u.id << "\t" << u.file_path << "\t" << u.start_line << "\t" 
            << u.end_line << "\t" << static_cast<int>(u.type) << "\t"
            << u.name << "\t" << u.content_hash << "\t"
            << u.dependencies.size();
        for (const auto& dep : u.dependencies) {
            oss << "\t" << dep;
        }
        oss << "\n";
    }
    return oss.str();
}

void UnitManager::deserialize(const std::string& data) {
    clear();
    std::istringstream iss(data);
    size_t count;
    iss >> count;
    iss.ignore();
    
    for (size_t i = 0; i < count; ++i) {
        std::string line;
        std::getline(iss, line);
        std::istringstream line_iss(line);
        
        CompilationUnit u;
        int type_int;
        size_t dep_count;
        
        std::getline(line_iss, u.id, '\t');
        std::getline(line_iss, u.file_path, '\t');
        line_iss >> u.start_line;
        line_iss.ignore();
        line_iss >> u.end_line;
        line_iss.ignore();
        line_iss >> type_int;
        u.type = static_cast<UnitType>(type_int);
        line_iss.ignore();
        std::getline(line_iss, u.name, '\t');
        std::getline(line_iss, u.content_hash, '\t');
        line_iss >> dep_count;
        
        for (size_t j = 0; j < dep_count; ++j) {
            std::string dep;
            line_iss.ignore();
            std::getline(line_iss, dep, '\t');
            if (!dep.empty()) {
                u.dependencies.push_back(dep);
            }
        }
        
        add_unit(u);
    }
    
    // 重建依赖关系
    for (auto& pair : units_) {
        for (const auto& dep_id : pair.second.dependencies) {
            auto* dep_unit = get_unit(dep_id);
            if (dep_unit) {
                dep_unit->dependents.push_back(pair.first);
            }
        }
    }
}

// ============================================================================
// ChangeDetector 实现
// ============================================================================

ChangeDetector::ChangeDetector() {}
ChangeDetector::~ChangeDetector() {}

std::string ChangeDetector::compute_hash(const std::string& content) {
    uint64_t hash = fnv1a_hash(content.c_str(), content.size());
    std::ostringstream oss;
    oss << std::hex << std::setfill('0') << std::setw(16) << hash;
    return oss.str();
}

std::string ChangeDetector::compute_line_hash(const std::string& line) {
    // 去除首尾空白后计算哈希
    size_t start = line.find_first_not_of(" \t\r\n");
    size_t end = line.find_last_not_of(" \t\r\n");
    if (start == std::string::npos) {
        return "empty";
    }
    std::string trimmed = line.substr(start, end - start + 1);
    return compute_hash(trimmed);
}

Snapshot ChangeDetector::create_snapshot(const std::string& file_path, 
                                         const std::string& content) {
    Snapshot snap;
    snap.file_path = file_path;
    snap.content_hash = compute_hash(content);
    snap.timestamp = current_timestamp();
    
    auto lines = split_lines(content);
    snap.line_hashes.reserve(lines.size());
    for (const auto& line : lines) {
        snap.line_hashes.push_back(compute_line_hash(line));
    }
    
    return snap;
}

std::vector<int> ChangeDetector::get_changed_lines(const Snapshot& old_snap,
                                                   const Snapshot& new_snap) {
    std::vector<int> changed;
    
    size_t old_size = old_snap.line_hashes.size();
    size_t new_size = new_snap.line_hashes.size();
    size_t max_size = std::max(old_size, new_size);
    
    // 使用 LCS 算法进行精确对比
    auto lcs = compute_lcs(old_snap.line_hashes, new_snap.line_hashes);
    
    // 标记所有不在 LCS 中的行为变更
    std::unordered_set<int> lcs_new_lines;
    for (const auto& pair : lcs) {
        lcs_new_lines.insert(pair.second);
    }
    
    for (size_t i = 0; i < new_size; ++i) {
        if (lcs_new_lines.find(static_cast<int>(i)) == lcs_new_lines.end()) {
            changed.push_back(static_cast<int>(i) + 1);  // 1-based
        }
    }
    
    return changed;
}

std::vector<std::pair<int, int>> ChangeDetector::compute_lcs(
    const std::vector<std::string>& old_lines,
    const std::vector<std::string>& new_lines) {
    
    int m = static_cast<int>(old_lines.size());
    int n = static_cast<int>(new_lines.size());
    
    // DP 表
    std::vector<std::vector<int>> dp(m + 1, std::vector<int>(n + 1, 0));
    
    for (int i = 1; i <= m; ++i) {
        for (int j = 1; j <= n; ++j) {
            if (old_lines[i - 1] == new_lines[j - 1]) {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = std::max(dp[i - 1][j], dp[i][j - 1]);
            }
        }
    }
    
    // 回溯找出 LCS 对应关系
    std::vector<std::pair<int, int>> lcs;
    int i = m, j = n;
    while (i > 0 && j > 0) {
        if (old_lines[i - 1] == new_lines[j - 1]) {
            lcs.push_back({i - 1, j - 1});
            --i; --j;
        } else if (dp[i - 1][j] > dp[i][j - 1]) {
            --i;
        } else {
            --j;
        }
    }
    
    std::reverse(lcs.begin(), lcs.end());
    return lcs;
}

std::vector<ChangeRecord> ChangeDetector::detect_changes(const Snapshot& old_snap,
                                                          const Snapshot& new_snap) {
    std::vector<ChangeRecord> records;
    
    // 对比两个快照中的编译单元
    std::unordered_set<std::string> old_ids, new_ids;
    
    for (const auto& pair : old_snap.units) {
        old_ids.insert(pair.first);
    }
    for (const auto& pair : new_snap.units) {
        new_ids.insert(pair.first);
    }
    
    // 检测删除的单元
    for (const auto& id : old_ids) {
        if (new_ids.find(id) == new_ids.end()) {
            ChangeRecord rec;
            rec.unit_id = id;
            rec.change_type = UnitState::DELETED;
            const auto& old_unit = old_snap.units.at(id);
            rec.old_start_line = old_unit.start_line;
            rec.old_end_line = old_unit.end_line;
            rec.reason = "unit deleted";
            records.push_back(rec);
        }
    }
    
    // 检测新增和修改的单元
    for (const auto& pair : new_snap.units) {
        const auto& new_unit = pair.second;
        auto old_it = old_snap.units.find(pair.first);
        
        if (old_it == old_snap.units.end()) {
            // 新增
            ChangeRecord rec;
            rec.unit_id = pair.first;
            rec.change_type = UnitState::ADDED;
            rec.new_start_line = new_unit.start_line;
            rec.new_end_line = new_unit.end_line;
            rec.reason = "unit added";
            records.push_back(rec);
        } else {
            // 检查是否修改
            const auto& old_unit = old_it->second;
            if (old_unit.content_hash != new_unit.content_hash) {
                ChangeRecord rec;
                rec.unit_id = pair.first;
                rec.change_type = UnitState::MODIFIED;
                rec.old_start_line = old_unit.start_line;
                rec.old_end_line = old_unit.end_line;
                rec.new_start_line = new_unit.start_line;
                rec.new_end_line = new_unit.end_line;
                rec.reason = "content changed";
                records.push_back(rec);
            }
        }
    }
    
    return records;
}

// ============================================================================
// CompilationCache 实现
// ============================================================================

CompilationCache::CompilationCache(const std::string& cache_dir)
    : cache_dir_(cache_dir), hits_(0), misses_(0) {}

CompilationCache::~CompilationCache() {
    save();
}

bool CompilationCache::has(const std::string& unit_id) const {
    return cache_.find(unit_id) != cache_.end();
}

std::string CompilationCache::get(const std::string& unit_id) const {
    auto it = cache_.find(unit_id);
    if (it != cache_.end()) {
        ++hits_;
        return it->second.output;
    }
    ++misses_;
    return "";
}

void CompilationCache::put(const std::string& unit_id, const std::string& output,
                           const std::string& content_hash) {
    CacheEntry entry;
    entry.output = output;
    entry.content_hash = content_hash;
    entry.timestamp = current_timestamp();
    cache_[unit_id] = entry;
}

void CompilationCache::invalidate(const std::string& unit_id) {
    cache_.erase(unit_id);
}

void CompilationCache::invalidate_all() {
    cache_.clear();
}

bool CompilationCache::is_valid(const std::string& unit_id, 
                                const std::string& current_hash) const {
    auto it = cache_.find(unit_id);
    if (it == cache_.end()) return false;
    return it->second.content_hash == current_hash;
}

void CompilationCache::save() {
    std::string cache_file = cache_dir_ + "/incremental_cache.dat";
    std::ofstream file(cache_file);
    if (!file.is_open()) return;
    
    file << cache_.size() << "\n";
    for (const auto& pair : cache_) {
        file << pair.first << "\n";
        file << pair.second.content_hash << "\n";
        file << pair.second.timestamp << "\n";
        file << pair.second.output.size() << "\n";
        file << pair.second.output;
    }
}

void CompilationCache::load() {
    std::string cache_file = cache_dir_ + "/incremental_cache.dat";
    std::ifstream file(cache_file);
    if (!file.is_open()) return;
    
    size_t count;
    file >> count;
    file.ignore();
    
    for (size_t i = 0; i < count; ++i) {
        std::string unit_id, content_hash;
        int64_t timestamp;
        size_t output_size;
        
        std::getline(file, unit_id);
        std::getline(file, content_hash);
        file >> timestamp >> output_size;
        file.ignore();
        
        std::string output(output_size, '\0');
        file.read(&output[0], output_size);
        
        CacheEntry entry;
        entry.output = output;
        entry.content_hash = content_hash;
        entry.timestamp = timestamp;
        cache_[unit_id] = entry;
    }
}

// ============================================================================
// IncrementalEngine 实现
// ============================================================================

IncrementalEngine::IncrementalEngine(const std::string& cache_dir)
    : cache_(cache_dir) {
    cache_.load();
}

IncrementalEngine::~IncrementalEngine() {
    save_state();
}

void IncrementalEngine::register_units(const std::string& file_path,
                                       const std::vector<CompilationUnit>& units) {
    // 移除该文件的旧单元
    auto old_units = units_.get_units_by_file(file_path);
    for (auto* old_unit : old_units) {
        units_.remove_unit(old_unit->id);
    }
    
    // 添加新单元
    for (const auto& unit : units) {
        units_.add_unit(unit);
    }
}

std::vector<ChangeRecord> IncrementalEngine::update_source(
    const std::string& file_path, const std::string& new_content) {
    
    // 创建新快照
    Snapshot new_snap = detector_.create_snapshot(file_path, new_content);
    
    // 获取旧快照
    auto old_it = snapshots_.find(file_path);
    
    std::vector<ChangeRecord> changes;
    if (old_it != snapshots_.end()) {
        // 获取变更的行
        auto changed_lines = detector_.get_changed_lines(old_it->second, new_snap);
        
        // 找出受影响的编译单元
        std::unordered_set<std::string> affected_ids;
        for (int line : changed_lines) {
            auto units = units_.get_units_in_range(file_path, line, line);
            for (auto* unit : units) {
                affected_ids.insert(unit->id);
                // 标记为已修改
                unit->state = UnitState::MODIFIED;
                unit->cache_valid = false;
                
                // 获取所有受影响的依赖单元
                auto dependents = units_.get_affected_units(unit->id);
                for (const auto& dep_id : dependents) {
                    affected_ids.insert(dep_id);
                    auto* dep_unit = units_.get_unit(dep_id);
                    if (dep_unit) {
                        dep_unit->state = UnitState::AFFECTED;
                        dep_unit->cache_valid = false;
                    }
                }
            }
        }
        
        // 扩展到完整边界
        std::vector<std::string> ids_to_expand(affected_ids.begin(), affected_ids.end());
        expand_to_boundaries(file_path, ids_to_expand);
        affected_ids = std::unordered_set<std::string>(ids_to_expand.begin(), ids_to_expand.end());
        
        // 生成变更记录
        for (const auto& id : affected_ids) {
            auto* unit = units_.get_unit(id);
            if (unit) {
                ChangeRecord rec;
                rec.unit_id = id;
                rec.change_type = unit->state;
                rec.new_start_line = unit->start_line;
                rec.new_end_line = unit->end_line;
                changes.push_back(rec);
            }
        }
        
        // 需要重新编译的单元
        units_to_compile_.clear();
        for (const auto& id : affected_ids) {
            units_to_compile_.push_back(id);
        }
    } else {
        // 首次编译，所有单元都需要编译
        auto units = units_.get_units_by_file(file_path);
        for (auto* unit : units) {
            unit->state = UnitState::ADDED;
            units_to_compile_.push_back(unit->id);
            
            ChangeRecord rec;
            rec.unit_id = unit->id;
            rec.change_type = UnitState::ADDED;
            rec.new_start_line = unit->start_line;
            rec.new_end_line = unit->end_line;
            changes.push_back(rec);
        }
    }
    
    // 更新快照
    new_snap.units = std::unordered_map<std::string, CompilationUnit>();
    for (auto* unit : units_.get_units_by_file(file_path)) {
        new_snap.units[unit->id] = *unit;
    }
    snapshots_[file_path] = new_snap;
    
    return changes;
}

std::vector<std::string> IncrementalEngine::get_units_to_compile() const {
    return units_to_compile_;
}

void IncrementalEngine::mark_compiled(const std::string& unit_id, 
                                      const std::string& output) {
    auto* unit = units_.get_unit(unit_id);
    if (unit) {
        unit->cached_output = output;
        unit->cache_timestamp = current_timestamp();
        unit->cache_valid = true;
        unit->state = UnitState::UNCHANGED;
        
        // 更新缓存
        cache_.put(unit_id, output, unit->content_hash);
    }
    
    // 从待编译列表中移除
    units_to_compile_.erase(
        std::remove(units_to_compile_.begin(), units_to_compile_.end(), unit_id),
        units_to_compile_.end()
    );
}

std::string IncrementalEngine::get_combined_output(const std::string& file_path) const {
    std::ostringstream oss;
    auto units = const_cast<UnitManager&>(units_).get_units_by_file(file_path);
    
    // 按行号顺序排列
    std::sort(units.begin(), units.end(),
              [](const CompilationUnit* a, const CompilationUnit* b) {
                  return a->start_line < b->start_line;
              });
    
    for (size_t i = 0; i < units.size(); ++i) {
        const auto* unit = units[i];
        
        // 优先使用缓存
        std::string output;
        if (unit->cache_valid) {
            output = unit->cached_output;
        } else if (cache_.is_valid(unit->id, unit->content_hash)) {
            output = cache_.get(unit->id);
        }
        
        if (!output.empty()) {
            if (i > 0) oss << "\n";
            oss << output;
        }
    }
    
    return oss.str();
}

void IncrementalEngine::expand_to_boundaries(const std::string& file_path,
                                             std::vector<std::string>& unit_ids) {
    std::unordered_set<std::string> expanded(unit_ids.begin(), unit_ids.end());
    
    for (const auto& id : unit_ids) {
        auto* unit = units_.get_unit(id);
        if (!unit) continue;
        
        // 对于函数、类等结构，确保整个结构都被包含
        if (unit->type == UnitType::FUNCTION || unit->type == UnitType::CLASS) {
            // 已经是完整结构，不需要扩展
            continue;
        }
        
        // 检查是否在某个大结构内
        auto all_units = units_.get_units_by_file(file_path);
        for (auto* parent : all_units) {
            if (parent->id == id) continue;
            
            // 如果当前单元在父结构范围内
            if (parent->start_line <= unit->start_line && 
                parent->end_line >= unit->end_line) {
                // 父结构是函数或类，需要重新编译整个结构
                if (parent->type == UnitType::FUNCTION || parent->type == UnitType::CLASS) {
                    expanded.insert(parent->id);
                    parent->state = UnitState::AFFECTED;
                    parent->cache_valid = false;
                }
            }
        }
    }
    
    unit_ids = std::vector<std::string>(expanded.begin(), expanded.end());
}

void IncrementalEngine::save_state() {
    cache_.save();
    
    // 保存单元状态
    std::string state_file = cache_.cache().empty() ? "incremental_state.dat" 
                             : cache_dir_ + "/incremental_state.dat";
    // Note: cache_dir_ is private, so we'll save alongside cache
}

void IncrementalEngine::load_state() {
    cache_.load();
}

}  // namespace incremental
}  // namespace sikuwa
