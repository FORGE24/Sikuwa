// sikuwa/cpp_cache/smart_cache.cpp
// 智能缓存策略和构建缓存系统实现

#include "smart_cache.h"
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

bool LRUCache::contains(const std::string& key) const {
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

size_t LRUCache::size() const {
    std::lock_guard<std::mutex> lock(mutex_);
    return cache_.size();
}

size_t LRUCache::max_size() const {
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

void LRUCache::dump_cache_stats() const {
    std::lock_guard<std::mutex> lock(mutex_);
    std::cout << "LRU Cache Statistics:" << std::endl;
    std::cout << "  Current size: " << cache_.size() << std::endl;
    std::cout << "  Maximum size: " << max_size_ << std::endl;
    std::cout << "  Item hit ratio: -" << std::endl; // 需要实现命中计数器
}

// LFU Cache Implementation
LFUCache::LFUCache(size_t max_size) : max_size_(max_size), min_frequency_(0) {
}

LFUCache::~LFUCache() {
    clear();
}

bool LFUCache::contains(const std::string& key) const {
    std::lock_guard<std::mutex> lock(mutex_);
    return cache_.find(key) != cache_.end();
}

bool LFUCache::put(const std::string& key, const std::string& value) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    // Check if key already exists
    auto it = cache_.find(key);
    if (it != cache_.end()) {
        // Update value and frequency
        it->second.value = value;
        // Increment frequency and update position
        size_t old_freq = it->second.frequency;
        size_t new_freq = old_freq + 1;
        
        // Remove from old frequency list
        freq_map_[old_freq].erase(it->second.usage_iter);
        if (freq_map_[old_freq].empty() && old_freq == min_frequency_) {
            min_frequency_++;
        }
        
        // Add to new frequency list
        freq_map_[new_freq].push_front(key);
        it->second.frequency = new_freq;
        it->second.usage_iter = freq_map_[new_freq].begin();
        
        return true;
    }
    
    // Check if cache is full
    if (cache_.size() >= max_size_) {
        // Remove least frequently used item
        std::string lfu_key = freq_map_[min_frequency_].back();
        freq_map_[min_frequency_].pop_back();
        cache_.erase(lfu_key);
    }
    
    // Add new item
    min_frequency_ = 1;
    freq_map_[1].push_front(key);
    cache_[key] = {value, 1, freq_map_[1].begin()};
    
    return true;
}

std::string LFUCache::get(const std::string& key) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = cache_.find(key);
    if (it == cache_.end()) {
        return "";
    }
    
    // Increment frequency
    size_t old_freq = it->second.frequency;
    size_t new_freq = old_freq + 1;
    
    // Remove from old frequency list
    freq_map_[old_freq].erase(it->second.usage_iter);
    if (freq_map_[old_freq].empty() && old_freq == min_frequency_) {
        min_frequency_++;
    }
    
    // Add to new frequency list
    freq_map_[new_freq].push_front(key);
    it->second.frequency = new_freq;
    it->second.usage_iter = freq_map_[new_freq].begin();
    
    return it->second.value;
}

bool LFUCache::remove(const std::string& key) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    auto it = cache_.find(key);
    if (it == cache_.end()) {
        return false;
    }
    
    // Remove from frequency list
    size_t freq = it->second.frequency;
    freq_map_[freq].erase(it->second.usage_iter);
    if (freq_map_[freq].empty() && freq == min_frequency_) {
        min_frequency_++;
    }
    
    // Remove from cache
    cache_.erase(it);
    return true;
}

void LFUCache::clear() {
    std::lock_guard<std::mutex> lock(mutex_);
    cache_.clear();
    freq_map_.clear();
    min_frequency_ = 0;
}

size_t LFUCache::size() const {
    std::lock_guard<std::mutex> lock(mutex_);
    return cache_.size();
}

size_t LFUCache::max_size() const {
    return max_size_;
}

void LFUCache::set_max_size(size_t max_size) {
    std::lock_guard<std::mutex> lock(mutex_);
    max_size_ = max_size;
    
    // Evict items if necessary
    while (cache_.size() > max_size_) {
        std::string lfu_key = freq_map_[min_frequency_].back();
        freq_map_[min_frequency_].pop_back();
        cache_.erase(lfu_key);
    }
}

void LFUCache::dump_cache_stats() const {
    std::lock_guard<std::mutex> lock(mutex_);
    std::cout << "LFU Cache Statistics:" << std::endl;
    std::cout << "  Current size: " << cache_.size() << std::endl;
    std::cout << "  Maximum size: " << max_size_ << std::endl;
    std::cout << "  Minimum frequency: " << min_frequency_ << std::endl;
    std::cout << "  Frequency distribution:" << std::endl;
    for (const auto& freq_entry : freq_map_) {
        if (!freq_entry.second.empty()) {
            std::cout << "    Frequency " << freq_entry.first << ": " << freq_entry.second.size() << " items" << std::endl;
        }
    }
}

// BuildCache Implementation

// 使用C++标准库实现的哈希计算函数
std::string calculate_string_hash(const std::string& input) {
    std::hash<std::string> hasher;
    size_t hash_val = hasher(input);
    
    std::stringstream ss;
    ss << std::hex << hash_val;
    return ss.str();
}

BuildCache::BuildCache(const std::string& cache_dir, size_t max_size) : cache_dir_(cache_dir) {
    // Create cache directory if it doesn't exist
    std::filesystem::create_directories(cache_dir_);
    
    // Default to LRU cache
    cache_ = std::make_unique<LRUCache>(max_size);
}

BuildCache::~BuildCache() {
}

void BuildCache::set_cache_strategy(const std::string& strategy) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    size_t current_size = cache_->size();
    size_t max_size = cache_->max_size();
    
    if (strategy == "lfu") {
        cache_ = std::make_unique<LFUCache>(max_size);
    } else {
        // Default to LRU
        cache_ = std::make_unique<LRUCache>(max_size);
    }
}

std::string BuildCache::calculate_file_hash(const std::string& file_path) {
    std::ifstream file(file_path, std::ios::binary);
    if (!file) {
        return "";
    }
    
    std::string content((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
    return calculate_string_hash(content);
}

std::string BuildCache::calculate_command_hash(const std::string& command) {
    return calculate_string_hash(command);
}

bool BuildCache::has_file_changed(const std::string& file_path, const std::string& last_hash) {
    std::string current_hash = calculate_file_hash(file_path);
    return current_hash != last_hash;
}

bool BuildCache::cache_build_result(const std::string& target, 
                                 const std::string& command, 
                                 const std::vector<std::string>& dependencies,
                                 const std::string& result) {
    std::lock_guard<std::mutex> lock(mutex_);
    
    // Create cache key: target + command + dependencies hashes
    std::stringstream key_stream;
    key_stream << "target=" << target << ";";
    key_stream << "command=" << calculate_command_hash(command) << ";";
    
    for (const auto& dep : dependencies) {
        key_stream << "dep=" << dep << ":" << calculate_file_hash(dep) << ";";
    }
    
    std::string cache_key = calculate_string_hash(key_stream.str());
    
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
    key_stream << "command=" << calculate_command_hash(command) << ";";
    
    for (const auto& dep : dependencies) {
        key_stream << "dep=" << dep << ":" << calculate_file_hash(dep) << ";";
    }
    
    std::string cache_key = calculate_string_hash(key_stream.str());
    
    // Get cached result
    return cache_->get(cache_key);
}

bool BuildCache::needs_rebuild(const std::string& target, 
                            const std::string& command, 
                            const std::vector<std::string>& dependencies) {
    std::string cached_result = get_cached_build_result(target, command, dependencies);
    return cached_result.empty();
}

void BuildCache::clean_expired_cache(std::chrono::duration<int> max_age) {
    // For simplicity, we're not implementing this yet
    // In a real implementation, we would track item creation times and remove old items
    std::cout << "Cleaning expired cache not implemented yet." << std::endl;
}

void BuildCache::clean_target_cache(const std::string& target) {
    // For simplicity, we're not implementing this yet
    // In a real implementation, we would find all cache entries related to the target and remove them
    std::cout << "Cleaning target cache not implemented yet." << std::endl;
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

void BuildCache::dump_build_cache_stats() const {
    std::lock_guard<std::mutex> lock(mutex_);
    std::cout << "Build Cache Statistics:" << std::endl;
    std::cout << "  Cache directory: " << cache_dir_ << std::endl;
    cache_->dump_cache_stats();
}
