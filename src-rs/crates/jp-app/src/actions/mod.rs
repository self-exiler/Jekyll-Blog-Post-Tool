//! 动作层：把 .NET 侧的 `RelayCommand` 改写成「工作线程命令」。
//!
//! 统一形态——每个命令都长这样：
//! ```ignore
//! pub fn xxx(services: Arc<Services>, refresh: Refresh) {
//!     spawn_work(move || { /* 阻塞：选择器、对话框、IO、AI */ },
//!                move |_: ()| notify(&refresh));
//! }
//! ```
//! 三点约定：
//! 1. 命令在 UI 线程被调用，但立刻 `spawn_work` 到工作线程；选择器与对话框内部要
//!    `ask_ui`（阻塞等回答），在 UI 线程调用会自锁；
//! 2. `refresh` 是页面交给动作的「把状态写回控件」闭包，动作只在结束时经 [`notify`]
//!    投回 UI 线程执行，期间不持有共享状态的锁（`PostForm` 先取快照再用）；
//! 3. 失败一律弹对话框说明，不静默吞错（对偶 .NET 侧的 `catch → ShowInfoAsync`）。

pub mod authors;
pub mod post;
pub mod project;
pub mod settings;

use std::path::{Component, Path};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::runtime::post_to_ui;

/// 页面注册的刷新回调：把状态写回控件。可在任意线程调用。
pub type Refresh = Arc<dyn Fn() + Send + Sync>;

/// 页面状态句柄。页面每次进入新建一份（对偶 .NET「每次导航 new 一个 ViewModel」），
/// 但动作跑在工作线程上，所以需要 `Arc<Mutex<..>>` 而非裸字段。
pub type PageState<T> = Arc<Mutex<T>>;

/// 取页面状态：毒锁继续用——持锁期间 panic 的只有 UI 回调，状态本身仍然有效。
pub fn held<T>(state: &PageState<T>) -> MutexGuard<'_, T> {
    state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 把一次刷新投回 UI 线程；UI 已停转（窗口关闭）时静默丢弃。
pub fn notify(refresh: &Refresh) {
    let refresh = Arc::clone(refresh);
    if !post_to_ui(move || refresh()) {
        // 入队失败 = 窗口已关，没有任何控件需要刷新了
    }
}

/// AI 提取关键字的默认上限（对偶 .NET `ExtractKeywordsAsync(body, maxCount = 5)`）。
pub const MAX_KEYWORDS: usize = 5;

/// 可选的图片扩展名（对偶 `ImageExtensions`）。
pub const IMAGE_EXTENSIONS: [&str; 6] = [".jpg", ".jpeg", ".png", ".gif", ".webp", ".svg"];

/// 保存成功后展示给用户的相对路径（对偶 `Path.GetRelativePath`）。
///
/// 两侧路径都来自本机文件系统，按 Windows 语义逐段不区分大小写比较；
/// 首段就不同（不同盘符 / UNC 主机）时无法相对，直接返回完整路径。
pub fn relative_display(base: &Path, target: &Path) -> String {
    let base = components(base);
    let parts = components(target);

    let shared = base
        .iter()
        .zip(parts.iter())
        .take_while(|(from_base, from_target)| part_eq(from_base, from_target))
        .count();

    // 首段就不同（不同盘符 / UNC 主机）：无法相对，原样展示
    if shared == 0 {
        return target.to_string_lossy().into_owned();
    }

    let mut result: Vec<String> = vec!["..".to_owned(); base.len() - shared];
    result.extend_from_slice(&parts[shared..]);
    if result.is_empty() {
        return ".".to_owned();
    }
    result.join("\\")
}

fn components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|part| match part {
            Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().into_owned()),
            Component::RootDir => Some("\\".to_owned()),
            Component::CurDir => None,
            Component::ParentDir => Some("..".to_owned()),
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
        })
        .collect()
}

fn part_eq(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_display_strips_shared_prefix() {
        let base = Path::new("C:\\blog");
        let target = Path::new("C:\\blog\\_posts\\2026-07-28-a.md");
        assert_eq!(relative_display(base, target), "_posts\\2026-07-28-a.md");
    }

    #[test]
    fn relative_display_ignores_case() {
        let base = Path::new("C:\\Blog");
        let target = Path::new("c:\\blog\\_posts\\a.md");
        assert_eq!(relative_display(base, target), "_posts\\a.md");
    }

    #[test]
    fn relative_display_keeps_path_when_roots_differ() {
        let base = Path::new("C:\\blog");
        let target = Path::new("D:\\blog\\_posts\\a.md");
        assert_eq!(relative_display(base, target), "D:\\blog\\_posts\\a.md");
    }

    #[test]
    fn relative_display_climbs_when_target_is_shorter() {
        let base = Path::new("C:\\blog\\_posts");
        let target = Path::new("C:\\blog\\assets\\img\\a.png");
        assert_eq!(
            relative_display(base, target),
            "..\\assets\\img\\a.png"
        );
    }

    #[test]
    fn relative_display_of_same_directory_is_dot() {
        assert_eq!(
            relative_display(Path::new("C:\\blog"), Path::new("C:\\blog")),
            "."
        );
    }

    #[test]
    fn notification_without_dispatcher_is_dropped() {
        // 单元测试没有 UI 线程：post_to_ui 失败，notify 只要求不 panic
        let refresh: Refresh = Arc::new(|| {});
        notify(&refresh);
    }
}
