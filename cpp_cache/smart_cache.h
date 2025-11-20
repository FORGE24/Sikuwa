// sikuwa/cpp_cache/smart_cache.h
// 智能缓存策略和构建缓存系统
// 仅使用C++标准库和Python C API

#ifndef SMART_CACHE_H
#define SMART_CACHE_H

#include <iostream>
#include <unordered_map>
#include <list>
#include <string>
#include <chrono>
#include <vector>
#include <memory>
#include <mutex>
#include <functional>

// 缓存项的元数据
struct CacheItemMetadata {
    std::chrono::time_point<std::chrono::system_clock> created_at;
    std::chrono::time_point<std::chrono::system_clock> last_accessed;
    size_t size_in_bytes;
    int access_count;
    std::vector<std::string> dependencies;
};

// 缓存项
template<typename T>
struct CacheItem {
    T value;
    CacheItemMetadata metadata;
};

// 基础缓存接口
class BaseCache {
public:
    virtual ~BaseCache() = default;
    virtual bool contains(const std::string& key) const = 0;
    virtual bool put(const std::string& key, const std::string& value) = 0;
    virtual std::string get(const std::string& key) = 0;
    virtual bool remove(const std::string& key) = 0;
    virtual void clear() = 0;
    virtual size_t size() const = 0;
    virtual size_t max_size() const = 0;
    virtual void set_max_size(size_t max_size) = 0;
    virtual void dump_cache_stats() const = 0;
};

// LRU (Least Recently Used) 缓存实现
class LRUCache : public BaseCache {
private:
    size_t max_size_;
    std::unordered_map<std::string, std::pair<std::string, std::list<std::string>::iterator>> cache_;
    std::list<std::string> usage_order_;
    mutable std::mutex mutex_;

public:
    explicit LRUCache(size_t max_size = 1000);
    ~LRUCache() override;
    bool contains(const std::string& key) const override;
    bool put(const std::string& key, const std::string& value) override;
    std::string get(const std::string& key) override;
    bool remove(const std::string& key) override;
    void clear() override;
    size_t size() const override;
    size_t max_size() const override;
    void set_max_size(size_t max_size) override;
    void dump_cache_stats() const override;
};

// LFU (Least Frequently Used) 缓存实现
class LFUCache : public BaseCache {
private:
    size_t max_size_;
    
    struct Node {
        std::string value;
        size_t frequency;
        std::list<std::string>::iterator usage_iter;
    };
    
    std::unordered_map<std::string, Node> cache_;
    std::unordered_map<size_t, std::list<std::string>> freq_map_;
    size_t min_frequency_;
    mutable std::mutex mutex_;

public:
    explicit LFUCache(size_t max_size = 1000);
    ~LFUCache() override;
    bool contains(const std::string& key) const override;
    bool put(const std::string& key, const std::string& value) override;
    std::string get(const std::string& key) override;
    bool remove(const std::string& key) override;
    void clear() override;
    size_t size() const override;
    size_t max_size() const override;
    void set_max_size(size_t max_size) override;
    void dump_cache_stats() const override;
};

// 构建缓存系统
class BuildCache {
private:
    std::unique_ptr<BaseCache> cache_;
    std::string cache_dir_;
    mutable std::mutex mutex_;

    // 计算文件的哈希值
    std::string calculate_file_hash(const std::string& file_path);
    
    // 计算构建命令的哈希值
    std::string calculate_command_hash(const std::string& command);
    
    // 检查文件是否已更改
    bool has_file_changed(const std::string& file_path, const std::string& last_hash);

public:
    BuildCache(const std::string& cache_dir = ".cache", size_t max_size = 1000000000);
    ~BuildCache();
    
    // 设置缓存策略 ("lru" 或 "lfu")
    void set_cache_strategy(const std::string& strategy);
    
    // 缓存构建结果
    bool cache_build_result(const std::string& target, 
                          const std::string& command, 
                          const std::vector<std::string>& dependencies,
                          const std::string& result);
    
    // 获取缓存的构建结果
    std::string get_cached_build_result(const std::string& target, 
                                      const std::string& command, 
                                      const std::vector<std::string>& dependencies);
    
    // 检查是否需要重新构建
    bool needs_rebuild(const std::string& target, 
                     const std::string& command, 
                     const std::vector<std::string>& dependencies);
    
    // 清理过期缓存
    void clean_expired_cache(std::chrono::duration<int> max_age);
    
    // 清理特定目标的缓存
    void clean_target_cache(const std::string& target);
    
    // 清理所有缓存
    void clean_all_cache();
    
    // 导出缓存统计信息
    void dump_build_cache_stats() const;
};

#endif // SMART_CACHE_H