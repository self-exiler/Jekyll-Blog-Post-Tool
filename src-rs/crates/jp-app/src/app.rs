//! 应用外壳：对偶 `App.xaml.cs` 的手写组合根与 `MainPage.xaml(.cs)` 的 NavigationView 导航。
//!
//! 与 XAML 版的三处有意差异，均见 `src-rs/README.md`：
//! 1. 导航项不放图标——`Icon` 要 `Symbol`/`FontIcon` 加主题字体，代码构建只能填字形码，
//!    收益不抵噪音，留空；
//! 2. 不用 `Tag`，也不设 `x:Name`：`Tag` 得再包一层 `IInspectable`，`Name`（即
//!    `IFrameworkElement::Name`）会把项登记进 namescope，非 ASCII 名还有校验风险。
//!    改为**比对项本身**——我们把 `NavigationViewItem` 直接放进 `MenuItems`，
//!    `InvokedItemContainer()` 回吐的就是那五颗项（对偶 `Tag` 的查表作用）。
//!    注意不用 `InvokedItem()`：WinUI 3 里它回吐的是项的 `Content`，不是项本身；
//! 3. 标题栏文案取产品名，原版字面量是程序集名 `JekyllPostTool.App`。

use windows::Foundation::TypedEventHandler;
use windows_core::{Error, HSTRING, Interface, Result as WinResult};

use winui3::Microsoft::UI::Xaml::{
    Application, LaunchActivatedEventArgs, Markup::IXamlType, Window,
};
use winui3::Microsoft::UI::Xaml::Controls::{
    Frame, NavigationView, NavigationViewBackButtonVisible, NavigationViewItem,
    NavigationViewItemInvokedEventArgs, NavigationViewPaneDisplayMode, XamlControlsResources,
};
use winui3::{XamlAppOverrides, XamlCustomType, xaml_typename};

use crate::pages::{
    advanced::AdvancedPage, authors::AuthorsPage, post_body::PostBodyPage,
    post_header::PostHeaderPage, project::ProjectPage,
};
use crate::{pages, widgets};

/// 页面类型名（对偶 XAML 的 `x:Class`）：`Frame` 导航与元数据解析共用同一组字符串。
const PROJECT_PAGE: &str = "ProjectPage";
const AUTHORS_PAGE: &str = "AuthorsPage";
const POST_HEADER_PAGE: &str = "PostHeaderPage";
const POST_BODY_PAGE: &str = "PostBodyPage";
const ADVANCED_PAGE: &str = "AdvancedPage";

/// 导航项：标签 → 页面（对偶 `MainPage.xaml` 的五颗 `NavigationViewItem` 与其 `Tag`）。
const NAV_ITEMS: [(&str, &str); 5] = [
    ("项目", PROJECT_PAGE),
    ("作者", AUTHORS_PAGE),
    ("博文头信息", POST_HEADER_PAGE),
    ("博文正文", POST_BODY_PAGE),
    ("高级功能", ADVANCED_PAGE),
];

/// 导航面板宽度（对偶 `OpenPaneLength="280"`）。
const PANE_WIDTH: f64 = 280.0;

/// 窗口标题。
const APP_TITLE: &str = "Jekyll 博文工具";

pub struct App;

impl XamlAppOverrides for App {
    fn OnLaunched(
        &self,
        base: &Application,
        _args: Option<&LaunchActivatedEventArgs>,
    ) -> WinResult<()> {
        // 控件主题字典：代码构建的 UI 同样依赖它（按钮、输入框的默认模板都在这里）
        base.Resources()?
            .MergedDictionaries()?
            .Append(&XamlControlsResources::new()?)?;

        // 先导航到首页，再装配导航容器（对偶 `MainPage` 构造完即 `Navigate(typeof(ProjectPage))`）
        let frame = pages::content_frame()?;
        frame.Navigate2(&xaml_typename(PROJECT_PAGE))?;

        let navigation = build_navigation(&frame)?;

        let window = Window::new()?;
        window.SetContent(&navigation)?;

        // 对话框与文件选择器都要挂到窗口上：先登记，再装配尺寸/背景
        crate::services::attach_window(&window);
        crate::window::configure(&window, APP_TITLE)?;

        window.Activate()
    }

    /// `Frame` 按类型名找页面：本项目那五页交给各自的 `Activatable`，其余回落控件元数据提供器。
    fn TryResolveXamlType(&self, full_name: &HSTRING) -> WinResult<IXamlType> {
        if full_name == PROJECT_PAGE {
            XamlCustomType::<ProjectPage>::for_page(full_name)
        } else if full_name == AUTHORS_PAGE {
            XamlCustomType::<AuthorsPage>::for_page(full_name)
        } else if full_name == POST_HEADER_PAGE {
            XamlCustomType::<PostHeaderPage>::for_page(full_name)
        } else if full_name == POST_BODY_PAGE {
            XamlCustomType::<PostBodyPage>::for_page(full_name)
        } else if full_name == ADVANCED_PAGE {
            XamlCustomType::<AdvancedPage>::for_page(full_name)
        } else {
            Err(Error::empty())
        }
    }
}

/// 导航容器：面板属性 + 五颗项 + 初始选中（对偶 `MainPage.xaml` 与 `OnLoaded`）。
fn build_navigation(frame: &Frame) -> WinResult<NavigationView> {
    let navigation = NavigationView::new()?;
    navigation.SetIsBackButtonVisible(NavigationViewBackButtonVisible::Collapsed)?;
    navigation.SetIsSettingsVisible(false)?;
    navigation.SetOpenPaneLength(PANE_WIDTH)?;
    navigation.SetPaneDisplayMode(NavigationViewPaneDisplayMode::Left)?;
    navigation.SetContent(frame)?;

    let items = navigation.MenuItems()?;
    let mut entries = Vec::with_capacity(NAV_ITEMS.len());
    for (label, page) in NAV_ITEMS {
        let item = build_item(label)?;
        items.Append(&item)?;
        entries.push((item, page));
    }

    // 与原版一致：首项高亮。注意这只改选中态，页面已由上面的 Navigate2 到位
    navigation.SetSelectedItem(&entries[0].0)?;

    wire_selection(&navigation, frame, entries)?;
    Ok(navigation)
}

/// 一颗导航项。`Content` 放 `TextBlock`（`SetContent` 收 `Param<IInspectable>`，
/// 直接塞 `HSTRING` 得先装箱，读回来还要再拆），面板渲染出来与 XAML 的字符串 Content 一致。
fn build_item(label: &str) -> WinResult<NavigationViewItem> {
    let item = NavigationViewItem::new()?;
    item.SetContent(&widgets::body_text(label)?)?;
    Ok(item)
}

/// 点击导航项 → 换页（对偶 `NavView_SelectionChanged` 的 `Tag` 查表）。
///
/// 与原版同样「不在当前页才导航」：`Frame.Navigate` 每次都新建页面实例，
/// 重复导航会把正在编辑的正文清空。
fn wire_selection(
    navigation: &NavigationView,
    frame: &Frame,
    entries: Vec<(NavigationViewItem, &'static str)>,
) -> WinResult<()> {
    let frame = frame.clone();

    // 泛型委托的回调参数要显式标注：`TypedEventHandler` 的泛型推不出来（args 是 `Ref<T>`，可空借用）
    let handler = TypedEventHandler::new(
        move |_: windows_core::Ref<NavigationView>, args: windows_core::Ref<NavigationViewItemInvokedEventArgs>| {
        // 设置项与页脚项不该出现在这里（入口已关），拿不到容器就当没发生。
        // 用 `InvokedItemContainer` 而非 `InvokedItem`：WinUI 3 里后者回吐的是项的
        // `Content`（我们的 TextBlock），前者才是那颗 `NavigationViewItem`。
        let Some(item) = args
            .as_ref()
            .and_then(|args| args.InvokedItemContainer().ok())
            .and_then(|container| container.cast::<NavigationViewItem>().ok())
        else {
            return Ok(());
        };

        let Some(page) = entries
            .iter()
            .find_map(|(entry, page)| (*entry == item).then_some(*page))
        else {
            return Ok(());
        };

        let already_there = frame
            .CurrentSourcePageType()
            .ok()
            .is_some_and(|current| current.Name == HSTRING::from(page));
        if already_there {
            return Ok(());
        }

        frame.Navigate2(&xaml_typename(page)).map(|_| ())
    });

    navigation.ItemInvoked(&handler).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 启动时写死了两件事：默认高亮首项、直接导航到 `PROJECT_PAGE`。
    /// 二者必须指同一页，否则一进来就是「高亮项目、显示别的页」。
    #[test]
    fn first_nav_item_is_the_startup_page() {
        assert_eq!(NAV_ITEMS[0], ("项目", PROJECT_PAGE));
    }
}
