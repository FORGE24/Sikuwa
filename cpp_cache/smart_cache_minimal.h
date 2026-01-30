// sikuwa/cpp_cache/smart_cache_minimal.h
// 最小化版本智能缓存系统

#ifndef SMART_CACHE_MINIMAL_H
#define SMART_CACHE_MINIMAL_H

#include <iostream>
#include <unordered_map>
#include <string>
#include <vector>
#include <mutex>

// 简单的LRU缓存实现
class LRUCache {
private:
    size_t max_size_;
    std::unordered_map<std::string, std::string> cache_;
    std::mutex mutex_;

public:
    LRUCache(size_t max_size = 1000);
    bool put(const std::string& key, const std::string& value);
    std::string get(const std::string& key);
    bool contains(const std::string& key);
    void clear();
};

// 简单的构建缓存系统
class BuildCache {
private:
    LRUCache cache_;
    std::string cache_dir_;
    std::mutex mutex_;

public:
    BuildCache(const std::string& cache_dir = ".cache");
    bool cache_result(const std::string& target, 
                     const std::string& command, 
                     const std::string& result);
    std::string get_result(const std::string& target, 
                          const std::string& command);
    bool needs_rebuild(const std::string& target, 
                     const std::string& command);
};

#endif // SMART_CACHE_MINIMAL_H
