// sikuwa/cpp_cache/smart_cache_simple.h
// 简化版智能缓存系统

#ifndef SMART_CACHE_SIMPLE_H
#define SMART_CACHE_SIMPLE_H

#include <iostream>
#include <unordered_map>
#include <list>
#include <string>
#include <vector>
#include <memory>
#include <mutex>

// LRU (Least Recently Used) 缓存实现
class LRUCache {
private:
    size_t max_size_;
    std::unordered_map<std::string, std::pair<std::string, std::list<std::string>::iterator>> cache_;
    std::list<std::string> usage_order_;
    std::mutex mutex_;

public:
    LRUCache(size_t max_size = 1000);
    ~LRUCache();
    bool contains(const std::string& key);
    bool put(const std::string& key, const std::string& value);
    std::string get(const std::string& key);
    bool remove(const std::string& key);
    void clear();
    size_t size();
    size_t max_size();
    void set_max_size(size_t max_size);
    void dump_cache_stats();
};

// 构建缓存系统
class BuildCache {
private:
    std::unique_ptr<LRUCache> cache_;
    std::string cache_dir_;
    std::mutex mutex_;

    // 计算字符串的哈希值
    std::string calculate_hash(const std::string& input);
    
    // 计算文件的哈希值
    std::string calculate_file_hash(const std::string& file_path);

public:
    BuildCache(const std::string& cache_dir = ".cache", size_t max_size = 1000000000);
    ~BuildCache();
    
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
    
    // 清理所有缓存
    void clean_all_cache();
    
    // 导出缓存统计信息
    void dump_build_cache_stats();
};

#endif // SMART_CACHE_SIMPLE_H