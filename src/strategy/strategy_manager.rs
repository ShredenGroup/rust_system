use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::sync::Arc;
use std::time::Instant;

// 静态全局ID生成器
static GLOBAL_ID_GENERATOR: AtomicU64 = AtomicU64::new(0);

pub fn get_global_id() -> u64 {
    GLOBAL_ID_GENERATOR.fetch_add(1, Ordering::Relaxed)
}

pub struct IdGenerator {
    base_id: u64,
    counter: AtomicU64,
}

impl IdGenerator {
    pub fn new(base_id: u64) -> Self {
        Self {
            base_id,
            counter: AtomicU64::new(0),
        }
    }
    
    pub fn next_id(&self) -> u64 {
        let seq = self.counter.fetch_add(1, Ordering::Relaxed);
        self.base_id + seq
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_multithread() {
        let signal_id_generator = Arc::new(IdGenerator::new(0));
        
        // 克隆 Arc 用于第一个线程
        let signal_id_generator_clone1 = Arc::clone(&signal_id_generator);
        let handle1 = thread::spawn(move || {
            for _i in 0..1000 {
                let _new_id = signal_id_generator_clone1.next_id();
            }
        });
        
        // 克隆 Arc 用于第二个线程
        let signal_id_generator_clone2 = Arc::clone(&signal_id_generator);
        let handle2 = thread::spawn(move || {
            for _i in 0..1000 {
                let _new_id = signal_id_generator_clone2.next_id();
            }
        });
        
        // 等待两个线程完成
        handle1.join().unwrap();
        handle2.join().unwrap();
        
        // 现在应该有2000个ID被生成了
        // 下一个ID应该是2000（base_id=0 + counter=2000）
        assert_eq!(signal_id_generator.next_id(), 2000);
    }
    
    #[test]
    fn test_single_thread() {
        let generator = IdGenerator::new(1000);
        
        // 测试连续生成ID
        assert_eq!(generator.next_id(), 1000); // base_id + 0
        assert_eq!(generator.next_id(), 1001); // base_id + 1
        assert_eq!(generator.next_id(), 1002); // base_id + 2
    }
    
    #[test]
    fn test_different_base_ids() {
        let generator1 = IdGenerator::new(0);
        let generator2 = IdGenerator::new(10000);
        
        assert_eq!(generator1.next_id(), 0);
        assert_eq!(generator2.next_id(), 10000);
        assert_eq!(generator1.next_id(), 1);
        assert_eq!(generator2.next_id(), 10001);
    }
}
