// sikuwa/cpp_cache/smart_cache_simple.cpp
// 简化版智能缓存系统实现

#include "smart_cache_simple.h"
#include <iostream>
#include <fstream>
#include <sstream>
#include <algorithm>
#include <cstddef>
#include <filesystem>
#include <functional>

// LRU Cache Implementation
LRUCache::LRUCache(size_t max_size) : max_size_(max_size) {
}

LRUCache::~LRUCache() {
    clear();
}

bool LRUCache::contains(const std::string& key) {
    std::lock_guard<std::mutex> lock(mutex_);
    return cache_.find(key) != cache_.end();
}

bool LRUCache::put(const std::string& key, const std::string& value) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    // Check if key already exists
    auto it = cache_.find(key);
    if (it != cache_.end()) {
        // Update value and move to front
        it->second.first = value;
        usage_order_.erase(it->second.second);
        usage_order_.push_front(key);
        it->second.second = usage_order_.begin();
        return true;
    }
    
    // Check if cache is full
    if (cache_.size() >= max_size_) {
        // Remove least recently used item
        std::string lru_key = usage_order_.back();
        usage_order_.pop_back();
        cache_.erase(lru_key);
    }
    
    // Add new item
    usage_order_.push_front(key);
    cache_[key] = std::make_pair(value, usage_order_.begin());
    return true;
}

std::string LRUCache::get(const std::string& key) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = cache_.find(key);
    if (it == cache_.end()) {
        return "";
    }
    
    // Move accessed key to front
    usage_order_.erase(it->second.second);
    usage_order_.push_front(key);
    it->second.second = usage_order_.begin();
    
    return it->second.first;
}

bool LRUCache::remove(const std::string& key) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = cache_.find(key);
    if (it == cache_.end()) {
        return false;
    }
    
    usage_order_.erase(it->second.second);
    cache_.erase(it);
    return true;
}

void LRUCache::clear() {
    std::lock_guard<std::mutex> lock(mutex_);
    cache_.clear();
    usage_order_.clear();
}

size_t LRUCache::size() {
    std::lock_guard<std::mutex> lock(mutex_);
    return cache_.size();
}

size_t LRUCache::max_size() {
    return max_size_;
}

void LRUCache::set_max_size(size_t max_size) {
    std::lock_guard<std::mutex> lock(mutex_);
    max_size_ = max_size;
    
    // Evict items if necessary
    while (cache_.size() > max_size_) {
        std::string lru_key = usage_order_.back();
        usage_order_.pop_back();
        cache_.erase(lru_key);
    }
}

void LRUCache::dump_cache_stats() {
    std::lock_guard<std::mutex> lock(mutex_);
    std::cout << "LRU Cache Statistics:" << std::endl;
    std::cout << "  Current size: " << cache_.size() << std::endl;
    std::cout << "  Maximum size: " << max_size_ << std::endl;
}

// BuildCache Implementation

// 使用简单的哈希计算函数
std::string BuildCache::calculate_hash(const std::string& input) {
    std::hash<std::string> hasher;
    size_t hash_val = hasher(input);
    
    std::stringstream ss;
    ss << std::hex << hash_val;
    return ss.str();
}

std::string BuildCache::calculate_file_hash(const std::string& file_path) {
    std::ifstream file(file_path, std::ios::binary);
    if (!file) {
        return "";
    }
    
    std::string content((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
    return calculate_hash(content);
}

BuildCache::BuildCache(const std::string& cache_dir, size_t max_size) : cache_dir_(cache_dir) {
    // Create cache directory if it doesn't exist
    std::filesystem::create_directories(cache_dir_);
    
    // Use LRU cache
    cache_ = std::make_unique<LRUCache>(max_size);
}

BuildCache::~BuildCache() {
}

bool BuildCache::cache_build_result(const std::string& target, 
                                  const std::string& command, 
                                  const std::vector<std::string>& dependencies,
                                  const std::string& result) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    // Create cache key: target + command + dependencies hashes
    std::stringstream key_stream;
    key_stream << "target=" << target << ";";
    key_stream << "command=" << calculate_hash(command) << ";";
    
    for (const auto& dep : dependencies) {
        key_stream << "dep=" << dep << ":" << calculate_file_hash(dep) << ";";
    }
    
    std::string cache_key = calculate_hash(key_stream.str());
    
    // Cache the result
    return cache_->put(cache_key, result);
}

std::string BuildCache::get_cached_build_result(const std::string& target, 
                                              const std::string& command, 
                                              const std::vector<std::string>& dependencies) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    // Create cache key (same as in cache_build_result)
    std::stringstream key_stream;
    key_stream << "target=" << target << ";";
    key_stream << "command=" << calculate_hash(command) << ";";
    
    for (const auto& dep : dependencies) {
        key_stream << "dep=" << dep << ":" << calculate_file_hash(dep) << ";";
    }
    
    std::string cache_key = calculate_hash(key_stream.str());
    
    // Get cached result
    return cache_->get(cache_key);
}

bool BuildCache::needs_rebuild(const std::string& target, 
                             const std::string& command, 
                             const std::vector<std::string>& dependencies) {
    std::string cached_result = get_cached_build_result(target, command, dependencies);
    return cached_result.empty();
}

void BuildCache::clean_all_cache() {
    std::lock_guard<std::mutex> lock(mutex_);
    cache_->clear();
    
    // Also clean the cache directory
    if (std::filesystem::exists(cache_dir_)) {
        std::filesystem::remove_all(cache_dir_);
        std::filesystem::create_directories(cache_dir_);
    }
}

void BuildCache::dump_build_cache_stats() {
    std::lock_guard<std::mutex> lock(mutex_);
    std::cout << "Build Cache Statistics:" << std::endl;
    std::cout << "  Cache directory: " << cache_dir_ << std::endl;
    cache_->dump_cache_stats();
}
