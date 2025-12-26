// sikuwa/incremental/cpp/incremental_core.h
// 减量编译核心 - C++ 实现高性能组件
// 指哪编哪：只编译源码改变的部分

#ifndef SIKUWA_INCREMENTAL_CORE_H
#define SIKUWA_INCREMENTAL_CORE_H

#include <string>
#include <vector>
#include <unordered_map>
#include <unordered_set>
#include <memory>
#include <functional>
#include <optional>
#include <chrono>

namespace sikuwa {
namespace incremental {

// ============================================================================
// 编译单元类型
// ============================================================================
enum class UnitType {
    LINE,           // 单行
    STATEMENT,      // 语句
    FUNCTION,       // 函数
    CLASS,          // 类
    MODULE,         // 模块级
    IMPORT,         // 导入语句
    DECORATOR,      // 装饰器
    BLOCK           // 代码块
};

// ============================================================================
// 编译单元状态
// ============================================================================
enum class UnitState {
    UNKNOWN,        // 未知
    UNCHANGED,      // 未变更
    MODIFIED,       // 已修改
    ADDED,          // 新增
    DELETED,        // 已删除
    AFFECTED        // 受影响（依赖项变更）
};

// ============================================================================
// 编译单元 - 最小编译粒度
// ============================================================================
struct CompilationUnit {
    std::string id;                     // 唯一标识: file:start_line:end_line:hash
    std::string file_path;              // 源文件路径
    int start_line;                     // 起始行 (1-based)
    int end_line;                       // 结束行 (1-based)
    UnitType type;                      // 单元类型
    std::string name;                   // 名称 (函数名/类名等)
    std::string content_hash;           // 内容哈希
    std::vector<std::string> dependencies;  // 依赖的单元ID列表
    std::vector<std::string> dependents;    // 被依赖的单元ID列表
    UnitState state;                    // 当前状态
    
    // 缓存相关
    std::string cached_output;          // 缓存的编译产物
    int64_t cache_timestamp;            // 缓存时间戳
    bool cache_valid;                   // 缓存是否有效
    
    CompilationUnit() 
        : start_line(0), end_line(0), type(UnitType::LINE), 
          state(UnitState::UNKNOWN), cache_timestamp(0), cache_valid(false) {}
};

// ============================================================================
// 版本快照 - 用于变更检测
// ============================================================================
struct Snapshot {
    std::string file_path;
    std::string content_hash;           // 整体内容哈希
    std::vector<std::string> line_hashes;  // 每行哈希
    std::unordered_map<std::string, CompilationUnit> units;  // 编译单元
    int64_t timestamp;
    
    Snapshot() : timestamp(0) {}
};

// ============================================================================
// 变更记录
// ============================================================================
struct ChangeRecord {
    std::string unit_id;
    UnitState change_type;
    int old_start_line;
    int old_end_line;
    int new_start_line;
    int new_end_line;
    std::string reason;                 // 变更原因
};

// ============================================================================
// 编译单元管理器 - 管理所有编译单元
// ============================================================================
class UnitManager {
public:
    UnitManager();
    ~UnitManager();
    
    // 添加/更新编译单元
    void add_unit(const CompilationUnit& unit);
    void update_unit(const std::string& id, const CompilationUnit& unit);
    void remove_unit(const std::string& id);
    
    // 查询
    CompilationUnit* get_unit(const std::string& id);
    const CompilationUnit* get_unit(const std::string& id) const;
    std::vector<CompilationUnit*> get_units_by_file(const std::string& file_path);
    std::vector<CompilationUnit*> get_units_in_range(const std::string& file_path, int start, int end);
    
    // 依赖关系
    void add_dependency(const std::string& from_id, const std::string& to_id);
    void remove_dependency(const std::string& from_id, const std::string& to_id);
    std::vector<std::string> get_dependencies(const std::string& id) const;
    std::vector<std::string> get_dependents(const std::string& id) const;
    std::vector<std::string> get_affected_units(const std::string& changed_id) const;
    
    // 遍历
    void for_each(std::function<void(CompilationUnit&)> callback);
    size_t size() const { return units_.size(); }
    void clear();
    
    // 序列化
    std::string serialize() const;
    void deserialize(const std::string& data);
    
private:
    std::unordered_map<std::string, CompilationUnit> units_;
    std::unordered_map<std::string, std::vector<std::string>> file_units_;  // file -> unit_ids
    
    // 递归获取所有受影响的单元
    void collect_affected_recursive(const std::string& id, 
                                    std::unordered_set<std::string>& visited) const;
};

// ============================================================================
// 变更检测器 - 检测源码变更
// ============================================================================
class ChangeDetector {
public:
    ChangeDetector();
    ~ChangeDetector();
    
    // 创建快照
    Snapshot create_snapshot(const std::string& file_path, const std::string& content);
    
    // 检测变更
    std::vector<ChangeRecord> detect_changes(const Snapshot& old_snap, const Snapshot& new_snap);
    
    // 定位变更行
    std::vector<int> get_changed_lines(const Snapshot& old_snap, const Snapshot& new_snap);
    
    // 计算哈希
    static std::string compute_hash(const std::string& content);
    static std::string compute_line_hash(const std::string& line);
    
private:
    // LCS 算法找出变更
    std::vector<std::pair<int, int>> compute_lcs(const std::vector<std::string>& old_lines,
                                                  const std::vector<std::string>& new_lines);
};

// ============================================================================
// 编译缓存 - 缓存编译产物
// ============================================================================
class CompilationCache {
public:
    CompilationCache(const std::string& cache_dir);
    ~CompilationCache();
    
    // 缓存操作
    bool has(const std::string& unit_id) const;
    std::string get(const std::string& unit_id) const;
    void put(const std::string& unit_id, const std::string& output, const std::string& content_hash);
    void invalidate(const std::string& unit_id);
    void invalidate_all();
    
    // 验证缓存
    bool is_valid(const std::string& unit_id, const std::string& current_hash) const;
    
    // 持久化
    void save();
    void load();
    
    // 统计
    size_t size() const { return cache_.size(); }
    size_t hit_count() const { return hits_; }
    size_t miss_count() const { return misses_; }
    
private:
    struct CacheEntry {
        std::string output;
        std::string content_hash;
        int64_t timestamp;
    };
    
    std::string cache_dir_;
    std::unordered_map<std::string, CacheEntry> cache_;
    mutable size_t hits_;
    mutable size_t misses_;
};

// ============================================================================
// 减量编译引擎
// ============================================================================
class IncrementalEngine {
public:
    IncrementalEngine(const std::string& cache_dir);
    ~IncrementalEngine();
    
    // 注册编译单元
    void register_units(const std::string& file_path, 
                       const std::vector<CompilationUnit>& units);
    
    // 更新源码并检测变更
    std::vector<ChangeRecord> update_source(const std::string& file_path,
                                            const std::string& new_content);
    
    // 获取需要重新编译的单元
    std::vector<std::string> get_units_to_compile() const;
    
    // 标记单元编译完成
    void mark_compiled(const std::string& unit_id, const std::string& output);
    
    // 获取编译结果（按顺序拼接）
    std::string get_combined_output(const std::string& file_path) const;
    
    // 缓存管理
    CompilationCache& cache() { return cache_; }
    const CompilationCache& cache() const { return cache_; }
    
    // 单元管理
    UnitManager& units() { return units_; }
    const UnitManager& units() const { return units_; }
    
    // 状态
    void save_state();
    void load_state();
    
private:
    UnitManager units_;
    ChangeDetector detector_;
    CompilationCache cache_;
    std::unordered_map<std::string, Snapshot> snapshots_;  // file -> snapshot
    std::vector<std::string> units_to_compile_;
    
    // 扩展编译范围到完整结构
    void expand_to_boundaries(const std::string& file_path, 
                             std::vector<std::string>& unit_ids);
};

// ============================================================================
// 工具函数
// ============================================================================

// 生成单元ID
std::string generate_unit_id(const std::string& file_path, int start_line, 
                            int end_line, const std::string& content_hash);

// 获取当前时间戳
int64_t current_timestamp();

// 读取文件内容
std::string read_file(const std::string& path);

// 写入文件内容
void write_file(const std::string& path, const std::string& content);

// 分割行
std::vector<std::string> split_lines(const std::string& content);

// 合并行
std::string join_lines(const std::vector<std::string>& lines);

}  // namespace incremental
}  // namespace sikuwa

#endif  // SIKUWA_INCREMENTAL_CORE_H
