//! 构建期把 exe 自身的图标编进 Win32 资源（RT_GROUP_ICON）。
//!
//! 运行时的 `WM_SETICON`（`src/icon.rs`）只管窗口与任务栏的活动图标，
//! 资源管理器、固定到任务栏/开始菜单、Alt+Tab 缩略图这些看的是 exe 里那份资源，
//! 只能在这里挂——rustc 没有内建图标支持。
//!
//! 用的是 `assets/AppIcon.ico`（DIB 编码，见 `scripts/gen-icon-asset.py`）而不是
//! .NET 那份 PNG 编码的原件：`rc.exe` 与图标资源都按老格式最稳。

fn main() {
    println!("cargo:rerun-if-changed=assets/AppIcon.ico");
    println!("cargo:rerun-if-changed=Cargo.toml");

    let mut resource = winresource::WindowsResource::new();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let icon = std::path::Path::new(&manifest_dir)
        .join("assets")
        .join("AppIcon.ico");

    resource
        .set_icon(icon.to_str().expect("图标路径应为合法 UTF-8"))
        .compile()
        .expect("图标资源编译失败");
}
