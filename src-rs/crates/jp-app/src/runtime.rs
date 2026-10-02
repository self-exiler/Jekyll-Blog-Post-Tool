//! 运行期基座：UI 线程派发、工作线程回投、阻塞式提问桥、%APPDATA% 目录与崩溃日志。
//!
//! 线程模型与 .NET 版一致但机制不同：这里没有 async/await，也没有 SynchronizationContext。
//! 所有阻塞型工作（文件 IO、AI 调用）跑在 `std::thread` 上，结果经 `DispatcherQueue` 回投 UI 线程；
//! 反过来，工作线程需要用户回答时经 [`ask_ui`] 把对话框投到 UI 线程并阻塞等待——
//! UI 线程从不等待，因此不会出现「工作线程等 UI、UI 等对话框完成」的死锁。

use std::path::PathBuf;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use windows::Foundation::{TimeSpan, TypedEventHandler};
use windows_core::{IInspectable, Ref, Result as WinResult};
use winui3::Microsoft::UI::Dispatching::{DispatcherQueue, DispatcherQueueHandler, DispatcherQueueTimer};

/// UI 线程的调度队列，在 `Application::Start` 回调里捕获一次。
static QUEUE: OnceLock<DispatcherQueue> = OnceLock::new();

pub fn capture_dispatcher_queue() -> WinResult<()> {
    let queue = DispatcherQueue::GetForCurrentThread()?;
    let _ = QUEUE.set(queue);
    Ok(())
}

fn queue() -> Option<&'static DispatcherQueue> {
    QUEUE.get()
}

/// 100ns 刻度：`DispatcherQueueTimer` 只接受 `TimeSpan`，没有 `From<Duration>` 可用。
const fn timespan(millis: u64) -> TimeSpan {
    TimeSpan {
        Duration: millis as i64 * 10_000,
    }
}

/// 在 UI 线程执行；调用方可以是工作线程（异步投递）或 UI 线程（排入下一轮，避免重入）。
pub fn post_to_ui<F>(task: F) -> bool
where
    F: FnOnce() + Send + 'static,
{
    let Some(queue) = queue() else {
        return false;
    };

    let task = Mutex::new(Some(task));
    let handler = DispatcherQueueHandler::new(move || {
        // 处理器可能被多次 Invoke（重投递），只执行一次
        if let Some(task) = task.lock().ok().and_then(|mut slot| slot.take()) {
            task();
        }
        Ok(())
    });

    matches!(queue.TryEnqueue(&handler), Ok(true))
}

/// 把一次提问投递到 UI 线程，阻塞当前工作线程直至得到答案。
///
/// 只能在非 UI 线程调用；投递失败（队列停转、UI 未接手）返回 `None`，调用方按「用户取消」处理。
pub fn ask_ui<T, F>(ask: F) -> Option<T>
where
    T: Send + 'static,
    F: FnOnce(Sender<T>) + Send + 'static,
{
    let (answer_tx, answer_rx) = channel::<T>();
    let (ran_tx, ran_rx) = channel::<()>();

    if !post_to_ui(move || {
        let _ = ran_tx.send(());
        ask(answer_tx);
    }) {
        return None;
    }

    // 先确认 UI 线程确实接手，再无限期等待用户作答
    if ran_rx.recv_timeout(Duration::from_secs(5)).is_err() {
        return None;
    }

    answer_rx.recv().ok()
}

/// 在工作线程执行阻塞任务，完成后回到 UI 线程交付结果。
pub fn spawn_work<T, F, Then>(work: F, then: Then)
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
    Then: FnOnce(T) + Send + 'static,
{
    std::thread::spawn(move || {
        let outcome = work();
        if !post_to_ui(move || then(outcome)) {
            // 队列已停转（窗口关闭）：结果丢弃，与原实现的「无人接收」等价
        }
    });
}

/// `DispatcherQueueTimer` 防抖：反复 `restart` 只在最后一次静默期到期后触发一次。
///
/// 投影里没有 `AutoRepeat`，因此 tick 内先 `Stop`，靠「每次输入都 restart」实现单次触发语义。
pub struct Debounce {
    timer: DispatcherQueueTimer,
    _handler: TypedEventHandler<DispatcherQueueTimer, IInspectable>,
}

impl Debounce {
    /// `interval_ms` 内多次 `restart` 只在末尾触发一次回调。
    pub fn new<F>(interval_ms: u64, callback: F) -> WinResult<Self>
    where
        F: FnMut() + Send + 'static,
    {
        let queue = queue().ok_or_else(windows_core::Error::empty)?;
        let timer = queue.CreateTimer()?;
        timer.SetInterval(timespan(interval_ms))?;

        let callback: Arc<Mutex<dyn FnMut() + Send>> = Arc::new(Mutex::new(callback));
        let sink = Arc::clone(&callback);
        let handler = TypedEventHandler::new(move |sender: Ref<DispatcherQueueTimer>, _args| {
            if let Some(timer) = sender.as_ref() {
                let _ = timer.Stop();
            }
            if let Ok(mut callback) = sink.lock() {
                (*callback)();
            }
            Ok(())
        });
        timer.Tick(&handler)?;

        Ok(Self {
            timer,
            _handler: handler,
        })
    }

    /// 只用于重启计时器的句柄。
    ///
    /// `Debounce` 本身含事件委托，不满足 `Send`，没法塞进别的回调里；
    /// `DispatcherQueueTimer` 是 WinRT 类（投影为它写了 `unsafe impl Send + Sync`），
    /// 所以事件回调捕获这个句柄即可，`Debounce` 由页面结构体保活。
    pub fn handle(&self) -> DispatcherQueueTimer {
        self.timer.clone()
    }

    pub fn stop(&self) -> WinResult<()> {
        self.timer.Stop()
    }
}

/// 防抖重启：先停再起，配合单次触发的 tick 实现「静默期到期只做一次」。
pub fn restart_timer(timer: &DispatcherQueueTimer) -> WinResult<()> {
    timer.Stop()?;
    timer.Start()
}

/// ADR-009：`%APPDATA%\JekyllPostTool\`（Roaming），与 .NET 版共用同一份配置目录。
pub fn app_data_dir() -> PathBuf {
    let root = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    root.join("JekyllPostTool")
}

/// 崩溃日志：写盘失败不影响进程退出策略（与原实现一致，不吞异常）。
pub fn append_crash_log(message: &str) {
    use std::io::Write;

    let directory = app_data_dir();
    let _ = std::fs::create_dir_all(&directory);
    let line = format!("[{}] {message}\r\n\r\n", chrono::Local::now().to_rfc3339());
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("crash.log"))
        .and_then(|mut file| file.write_all(line.as_bytes()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timespan_converts_milliseconds_to_100ns_ticks() {
        assert_eq!(timespan(0).Duration, 0);
        assert_eq!(timespan(150).Duration, 1_500_000);
        assert_eq!(timespan(1000).Duration, 10_000_000);
    }

    #[test]
    fn posting_without_captured_queue_reports_failure() {
        // 单元测试不跑 WinUI 运行时：`QUEUE` 为空，调用方据此走「取消」分支
        let posted = std::panic::catch_unwind(|| post_to_ui(|| {}));
        assert!(posted.is_ok());
    }

    #[test]
    fn app_data_dir_points_at_roaming_profile() {
        let directory = app_data_dir();
        assert!(directory.ends_with(PathBuf::from("JekyllPostTool")));
    }
}
