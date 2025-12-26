// smart_cache_minimal.cpp
// Minimal implementation of smart cache system

#include <iostream>
#include <string>
#include <unordered_map>
#include <fstream>
#include <ctime>
#include <cstdlib>
#include "smart_cache_minimal.h"

// LRUCache implementation
LRUCache::LRUCache(size_t max_size) : max_size_(max_size) {}

bool LRUCache::put(const std::string& key, const std::string& value) {
    std::lock_guard<std::mutex> lock(mutex_);
    cache_[key] = value;
    return true;
}

std::string LRUCache::get(const std::string& key) {
    std::lock_guard<std::mutex> lock(mutex_);
    auto it = cache_.find(key);
    if (it == cache_.end()) {
        return "";
    }
    return it->second;
}

bool LRUCache::contains(const std::string& key) {
    std::lock_guard<std::mutex> lock(mutex_);
    return cache_.find(key) != cache_.end();
}

void LRUCache::clear() {
    std::lock_guard<std::mutex> lock(mutex_);
    cache_.clear();
}

// Helper function to create directories
bool create_directory_if_not_exists(const std::string& path) {
    std::string cmd = "mkdir " + path;
    int result = system(cmd.c_str());
    return result == 0;
}

// BuildCache implementation
BuildCache::BuildCache(const std::string& cache_dir) : cache_(10000), cache_dir_(cache_dir) {
    create_directory_if_not_exists(cache_dir_);
}

bool BuildCache::cache_result(const std::string& target, const std::string& command, const std::string& result) {
    std::lock_guard<std::mutex> lock(mutex_);
    std::string key = target + "|" + command;
    cache_.put(key, result);
    return true;
}

std::string BuildCache::get_result(const std::string& target, const std::string& command) {
    std::lock_guard<std::mutex> lock(mutex_);
    std::string key = target + "|" + command;
    return cache_.get(key);
}

bool BuildCache::needs_rebuild(const std::string& target, const std::string& command) {
    std::lock_guard<std::mutex> lock(mutex_);
    std::string key = target + "|" + command;
    return !cache_.contains(key);
}
