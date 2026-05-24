//! 内存滑动窗口缓存 — 存储设备遥测的近期数据。
//!
//! 每个 (设备, 指标) 对维护一个固定大小的滑动窗口。
//! 规则引擎用变化率计算，AI Bridge 用作上下文聚合。
//!
//! 线程安全：基于 DashMap 的无锁并发实现。

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::collections::VecDeque;

/// 每对 (设备, 指标) 默认最大数据点数
const DEFAULT_WINDOW_SIZE: usize = 100;

/// 单条遥测数据点
#[derive(Debug, Clone)]
pub struct DataPoint {
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 数值
    pub value: f64,
    /// 原始载荷（可选）
    pub raw_payload: Option<String>,
}

/// 线程安全的滑动窗口缓存
pub struct SlidingWindowCache {
    /// 内部存储：key = "device_id:metric"
    windows: DashMap<String, VecDeque<DataPoint>>,
    /// 每窗口最大容量
    max_size: usize,
}

impl SlidingWindowCache {
    /// 创建指定容量的缓存
    pub fn new(max_size: usize) -> Self {
        Self {
            windows: DashMap::new(),
            max_size,
        }
    }

    /// 插入数据点。超出容量时自动淘汰最旧的数据
    pub fn insert(&self, device_id: &str, metric: &str, point: DataPoint) {
        let key = cache_key(device_id, metric);
        let mut window = self.windows.entry(key).or_insert_with(VecDeque::new);

        window.push_back(point);
        while window.len() > self.max_size {
            window.pop_front(); // 淘汰最旧
        }
    }

    /// 获取 (设备, 指标) 的近期数据窗口，返回 (时间戳, 值) 列表
    pub fn get_window(&self, device_id: &str, metric: &str) -> Vec<(DateTime<Utc>, f64)> {
        let key = cache_key(device_id, metric);
        if let Some(window) = self.windows.get(&key) {
            window.iter().map(|dp| (dp.timestamp, dp.value)).collect()
        } else {
            Vec::new()
        }
    }

    /// 获取最新一条数据
    pub fn get_latest(&self, device_id: &str, metric: &str) -> Option<DataPoint> {
        let key = cache_key(device_id, metric);
        self.windows.get(&key).and_then(|w| w.back().cloned())
    }

    /// 清空所有缓存
    pub fn clear(&self) {
        self.windows.clear();
    }
}

impl Default for SlidingWindowCache {
    fn default() -> Self {
        Self::new(DEFAULT_WINDOW_SIZE)
    }
}

/// 生成缓存 key："device_id:metric"
fn cache_key(device_id: &str, metric: &str) -> String {
    format!("{}:{}", device_id, metric)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_retrieve() {
        let cache = SlidingWindowCache::new(3);
        let now = Utc::now();

        cache.insert("dev1", "temp", DataPoint {
            timestamp: now, value: 25.0, raw_payload: None,
        });
        cache.insert("dev1", "temp", DataPoint {
            timestamp: now, value: 26.0, raw_payload: None,
        });

        let window = cache.get_window("dev1", "temp");
        assert_eq!(window.len(), 2);
        assert_eq!(window[0].1, 25.0);
        assert_eq!(window[1].1, 26.0);
    }

    #[test]
    fn test_eviction() {
        let cache = SlidingWindowCache::new(2);
        let now = Utc::now();

        cache.insert("dev1", "temp", DataPoint { timestamp: now, value: 1.0, raw_payload: None });
        cache.insert("dev1", "temp", DataPoint { timestamp: now, value: 2.0, raw_payload: None });
        cache.insert("dev1", "temp", DataPoint { timestamp: now, value: 3.0, raw_payload: None });

        let window = cache.get_window("dev1", "temp");
        assert_eq!(window.len(), 2);
        assert_eq!(window[0].1, 2.0); // 1.0 已被淘汰
        assert_eq!(window[1].1, 3.0);
    }
}
