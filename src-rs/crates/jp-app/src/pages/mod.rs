//! 页面：每个页面对应 .NET 侧的 `Pages/*.xaml(.cs)`，控件全部由代码构建。
//!
//! 生命周期与 .NET 对齐：
//! - `activate()` 每次导航新建页面实例（对偶「每次导航 new 一个 ViewModel」），
//!   页面级状态因此天然是新的一份；
//! - `entered()` 里搭控件树、接线、回填状态（对偶构造函数 + `OnLoaded`）；
//! - 控件的 `TextChanged`/`Toggled` 把值拉回状态（Rust 投影没有双向绑定，
//!   等价于 `UpdateSourceTrigger=PropertyChanged`）；
//! - 动作在工作线程改状态，结束时把 `Refresh` 投回 UI 线程推回控件。
//!
//! 页面结构体的字段一律 `Arc`/`Mutex` 包裹：winui3 投影里的 COM 句柄本身 `Send + Sync`，
//! 事件委托又要求 `Send + 'static`，所以句柄可以被闭包直接捕获（克隆即共享同一实例）。

pub mod advanced;
pub mod authors;
pub mod post_body;
pub mod post_header;
pub mod project;

use std::sync::{Arc, Mutex};

use windows_core::Result as WinResult;
use winui3::Microsoft::UI::Xaml::{FrameworkElement, SizeChangedEventHandler, VerticalAlignment};
use winui3::Microsoft::UI::Xaml::Controls::{Border, Button, Frame, Grid, Page, StackPanel, TextBlock};

use crate::actions::{PageState, Refresh};
use crate::services::Services;
use crate::widgets;

pub use crate::widgets::Length;

/// 内容区宽度达到该值时左右分栏，否则上下堆叠（对偶 `PostPage.WideLayoutMinWidth`）。
/// `ActualWidth`/`NewSize.Width` 都是逻辑像素，与原版一样不靠 `AdaptiveTrigger`（其按物理像素度量）。
pub const WIDE_LAYOUT_MIN_WIDTH: f64 = 820.0;

/// 内容页边距（对偶 `Frame Padding="32,24"`）。
pub const CONTENT_PADDING: f64 = 32.0;

/// 页面级状态句柄：每次导航新建一份。
pub fn state<T>(value: T) -> PageState<T> {
    Arc::new(Mutex::new(value))
}

/// 把「状态写回控件」的闭包登记成动作层用的 `Refresh`。
pub fn refresh_of<F>(f: F) -> Refresh
where
    F: Fn() + Send + Sync + 'static,
{
    Arc::new(f)
}

/// 延迟绑定的刷新出口（见 [`through`]）。
pub type Slot = Arc<Mutex<Option<Refresh>>>;

/// 先交出一个 `Refresh`，真正的闭包等控件树建完再挂进 [`Slot`]。
///
/// 命令条之类的控件必须持有 `Refresh`，而 `Refresh` 又要引用这些控件——直接互相引用会编译不过。
/// 走一层插槽还顺带断开了引用环：`Controls → 按钮 → 插槽 → Refresh → Controls`，
/// 页面离开时把插槽清空，环即断开（否则每次导航都会漏一整棵控件树）。
pub fn through(slot: &Slot) -> Refresh {
    let slot = Arc::clone(slot);
    refresh_of(move || {
        // 取出来再调用：语句结束时锁已释放，刷新里若再碰插槽不会自锁
        let bound = slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        if let Some(refresh) = bound {
            refresh();
        }
    })
}

/// 绑定/解绑真正的刷新闭包（`None` 即解绑，页面离开时调用）。
pub fn bind(slot: &Slot, refresh: Option<Refresh>) {
    *slot
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = refresh;
}

/// 内容帧：承载页面，并给出与原版一致的内边距。
pub fn content_frame() -> WinResult<Frame> {
    let frame = Frame::new()?;
    frame.SetPadding(widgets::insets(CONTENT_PADDING, 24.0, CONTENT_PADDING, 24.0))?;
    Ok(frame)
}

/// 页头：左标题 + 右动作组（对偶各页顶部的 `Grid ColumnDefinitions="*,Auto"`）。
/// 标题文本块交给页面，刷新时可改写（博文页标题带文件名）。
pub struct PageHeader {
    pub root: Grid,
    pub title: TextBlock,
}

pub fn page_header(title: &str, actions: Option<&StackPanel>) -> WinResult<PageHeader> {
    let root = widgets::grid(
        &[widgets::Length::Auto],
        &[widgets::Length::Star(1.0), widgets::Length::Auto],
    )?;
    let heading = widgets::title_text(title)?;
    heading.SetVerticalAlignment(VerticalAlignment::Center)?;
    widgets::put(&root, &heading)?;

    if let Some(actions) = actions {
        widgets::put(&root, actions)?;
        widgets::place(actions, 0, 1)?;
    }

    Ok(PageHeader {
        root,
        title: heading,
    })
}

/// 卡片分节：标题 + 可选说明 + 竖排内容。
/// 对偶 `SettingsCard(ContentAlignment="Vertical")`——CommunityToolkit 控件不在 Rust 投影里，
/// 这里用「圆角卡片 + 小标题」近似，视觉层级一致。
pub fn section(title: &str, description: &str, content: &StackPanel) -> WinResult<Border> {
    let outer = widgets::vstack(8.0)?;

    let heading = widgets::section_text(title)?;
    widgets::add(&outer, &heading)?;

    if !description.is_empty() {
        let note = widgets::muted_text(description)?;
        widgets::add(&outer, &note)?;
    }

    widgets::add(&outer, content)?;
    widgets::card(&outer)
}

/// 页面骨架：页头（row 0）+ 可滚动内容（row 1）。
/// 正文页自己组多行布局，不走这里。
pub fn page_root(header: PageHeader, body: &StackPanel) -> WinResult<Grid> {
    let root = widgets::grid(&[widgets::Length::Auto, widgets::Length::Star(1.0)], &[])?;
    widgets::place(&header.root, 0, 0)?;
    widgets::put(&root, &header.root)?;

    let viewer = widgets::scroll_viewer(body)?;
    widgets::place(&viewer, 1, 0)?;
    widgets::put(&root, &viewer)?;

    Ok(root)
}

/// 订阅尺寸变化（对偶 XAML 的 `SizeChanged`）：回调收内容宽度（逻辑像素）。
///
/// 构建时 `ActualWidth` 还是 0，所以宽/窄布局只能在这里判定——与原版 `OnSizeChanged` 同理。
pub fn on_size_changed<F>(element: &FrameworkElement, handler: F) -> WinResult<()>
where
    F: Fn(f64) + Send + 'static,
{
    let sink = SizeChangedEventHandler::new(move |_, args| {
        if let Some(args) = args.as_ref() {
            if let Ok(size) = args.NewSize() {
                handler(f64::from(size.Width));
            }
        }
        Ok(())
    });
    element.SizeChanged(&sink).map(|_| ())
}

/// 博文两页共用的命令条：新建 / 打开 / 保存（对偶顶部三颗按钮）。
///
/// 三个动作都不需要额外输入：正文页的 `TextChanged` 已把正文实时写回编辑态，
/// 只有插图/导入才需要光标位置（那两个动作单独取）。
///
/// 返回结构体而不是一条面板：忙碌态要逐颗 `SetIsEnabled`——`Panel` 不是 `Control`，
/// 整条面板禁不掉。
pub struct PostActions {
    pub root: StackPanel,
    pub new_button: Button,
    pub open_button: Button,
    pub save_button: Button,
}

impl PostActions {
    /// 保存进行中禁用全部命令（对偶 `RelayCommand.CanExecuteChanged`）。
    pub fn set_busy(&self, busy: bool) -> WinResult<()> {
        let usable = !busy;
        widgets::enabled(&self.new_button, usable)?;
        widgets::enabled(&self.open_button, usable)?;
        widgets::enabled(&self.save_button, usable)
    }
}

pub fn post_actions(services: Arc<Services>, refresh: Refresh) -> WinResult<PostActions> {
    let row = widgets::hstack(8.0)?;

    let new_button = widgets::button("新建博文")?;
    let new_services = Arc::clone(&services);
    let new_refresh = Arc::clone(&refresh);
    widgets::on_click(&new_button, move || {
        crate::actions::post::new_post(Arc::clone(&new_services), Arc::clone(&new_refresh));
    })?;

    let open_button = widgets::button("打开已有博文")?;
    let open_services = Arc::clone(&services);
    let open_refresh = Arc::clone(&refresh);
    widgets::on_click(&open_button, move || {
        crate::actions::post::open(Arc::clone(&open_services), Arc::clone(&open_refresh));
    })?;

    let save_button = widgets::button("保存")?;
    let save_services = services;
    let save_refresh = refresh;
    widgets::on_click(&save_button, move || {
        crate::actions::post::save(Arc::clone(&save_services), Arc::clone(&save_refresh));
    })?;

    widgets::add(&row, &new_button)?;
    widgets::add(&row, &open_button)?;
    widgets::add(&row, &save_button)?;

    Ok(PostActions {
        root: row,
        new_button,
        open_button,
        save_button,
    })
}

/// 页面进入/离开的统一入口，好让 [`xaml_page`] 宏对五个页面写同一套转发。
pub trait PageLifecycle {
    /// 构建控件树并回填状态（对偶构造函数 + `OnLoaded`）。
    fn entered(&self, base: &Page) -> WinResult<()>;

    /// 离开页面：停掉页面自持的计时器等（默认无操作）。
    fn departed(&self) {}
}

/// 页面样板：`Activatable` + `XamlPageOverrides`。
///
/// `IPageOverrides` 是单个 vtable，三个回调都必须给出；我们的页面只在进入时构建、
/// 离开时收尾。另两个回调必须返回 `Ok`：上游示例的 `E_NOTIMPL` 写法只在「导航进、
/// 永不导航出」的单页场景成立——真离开页面时失败码会抛回 WinUI 导航机制，
/// 整个进程跟着崩（实测 0x80004001 未处理异常）。
#[macro_export]
macro_rules! xaml_page {
    ($name:ty) => {
        impl ::winui3::Activatable for $name {
            fn activate() -> ::windows_core::Result<::windows_core::IInspectable> {
                ::winui3::XamlPage::compose(<$name>::default()).map(Into::into)
            }
        }

        impl ::winui3::XamlPageOverrides for $name {
            fn OnNavigatedTo(
                &self,
                base: &::winui3::Microsoft::UI::Xaml::Controls::Page,
                _args: ::core::option::Option<
                    &::winui3::Microsoft::UI::Xaml::Navigation::NavigationEventArgs,
                >,
            ) -> ::windows_core::Result<()> {
                // 页面构建失败会被 XAML fail-fast（c000027b）吞掉细节，先落日志再上抛
                let result = $crate::pages::PageLifecycle::entered(self, base);
                if let Err(error) = &result {
                    $crate::runtime::append_crash_log(&format!(
                        "{}.entered 失败：{error:?}",
                        stringify!($name)
                    ));
                }
                result
            }

            fn OnNavigatedFrom(
                &self,
                _base: &::winui3::Microsoft::UI::Xaml::Controls::Page,
                _args: ::core::option::Option<
                    &::winui3::Microsoft::UI::Xaml::Navigation::NavigationEventArgs,
                >,
            ) -> ::windows_core::Result<()> {
                $crate::pages::PageLifecycle::departed(self);
                Ok(())
            }

            fn OnNavigatingFrom(
                &self,
                _base: &::winui3::Microsoft::UI::Xaml::Controls::Page,
                _args: ::core::option::Option<
                    &::winui3::Microsoft::UI::Xaml::Navigation::NavigatingCancelEventArgs,
                >,
            ) -> ::windows_core::Result<()> {
                Ok(())
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_layout_threshold_matches_original() {
        assert_eq!(WIDE_LAYOUT_MIN_WIDTH, 820.0);
    }

    #[test]
    fn page_state_is_fresh_per_call() {
        let first: PageState<crate::view_models::AuthorsPage> = state(Default::default());
        let second: PageState<crate::view_models::AuthorsPage> = state(Default::default());

        crate::actions::held(&first).draft.id = "cotes".to_owned();

        assert!(crate::actions::held(&second).draft.id.is_empty());
    }
}
