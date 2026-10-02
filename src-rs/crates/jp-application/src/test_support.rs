use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// 测试用临时目录：每次调用创建一个独占子目录，避免用例互相踩踏。
pub fn temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let directory = std::env::temp_dir().join(format!(
        "jekyllposttool-rs-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).expect("创建测试临时目录");
    directory
}

#[allow(dead_code)]
pub fn cleanup(directory: &Path) {
    let _ = std::fs::remove_dir_all(directory);
}
