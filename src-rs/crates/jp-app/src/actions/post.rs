//! 博文命令：新建、打开、保存、导入正文、插入图片、AI 提取关键字
//! （对偶 `PostPageViewModel.Posts.cs` / `.Body.cs` 的全部 RelayCommand）。
//!
//! 调用方（页面）负责两件事：
//! 1. 调用前把控件值拉回 `PostForm`（Rust 投影没有双向绑定）；
//! 2. 传入 `refresh` 闭包，动作结束时经 [`notify`] 回 UI 线程把状态写回控件。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use jp_application::posts::PostOperationResult;
use jp_domain::posts::{FrontMatter, insert_at_cursor, utf16_index_to_char_index};
use jp_domain::projects::BlogProject;
use jp_infrastructure::filesystem::image_inserter;
use jp_infrastructure::posts::post_file_format;

use super::{IMAGE_EXTENSIONS, MAX_KEYWORDS, Refresh, notify, relative_display};
use crate::runtime::spawn_work;
use crate::services::dialogs::UiSavePrompts;
use crate::services::{dialogs, pickers, Services};
use crate::view_models::PostForm;

/// 新建博文（FR-3.1：date 不预填）。纯状态操作，无 IO，直接在 UI 线程完成。
pub fn new_post(services: Arc<Services>, refresh: Refresh) {
    services.edit_form(|form| form.reset(PostForm::now()));
    refresh();
}

/// 打开 `_posts/` 下的博文（对偶 `OpenPostAsync`）。
pub fn open(services: Arc<Services>, refresh: Refresh) {
    spawn_work(
        move || open_blocking(&services),
        move |_: ()| notify(&refresh),
    )
}

fn open_blocking(services: &Services) {
    let Some(project) = services.project.current() else {
        dialogs::info("未选择项目", "请先选择一个博客项目。");
        return;
    };

    let Some(path) = pickers::pick_markdown_file() else {
        return;
    };
    if !in_posts_directory(&path, &project.posts_directory()) {
        dialogs::info("路径无效", "只能打开当前项目 _posts/ 目录下的 .md 文件。");
        return;
    }

    load_into_form(services, &path);
}

/// 读盘并回填编辑态：`OpenPostAsync` 与「取消并刷新」共用（单次读盘，展示内容与基线哈希同源）。
fn load_into_form(services: &Services, path: &Path) {
    match services.save_use_case.load(path) {
        Ok(Some(loaded)) => {
            services.edit_form(|form| form.apply_loaded(&loaded));
        }
        Ok(None) => dialogs::info("打开失败", "无法读取博文文件。"),
        Err(error) => dialogs::info("打开失败", &error.to_string()),
    }
}

/// 保存（对偶 `SavePostAsync`）：忙碌态在派发前置位，防止连点触发第二次对话框。
pub fn save(services: Arc<Services>, refresh: Refresh) {
    if services.form().is_busy {
        return;
    }
    services.edit_form(|form| form.is_busy = true);
    refresh();

    let finishing = Arc::clone(&services);
    spawn_work(
        move || save_blocking(&services),
        move |_: ()| {
            finishing.edit_form(|form| form.is_busy = false);
            notify(&refresh);
        },
    )
}

fn save_blocking(services: &Services) {
    let form = services.form();

    let Some(project) = services.project.current() else {
        dialogs::info("未选择项目", "请先选择一个博客项目。");
        return;
    };

    let front_matter = match form.to_front_matter() {
        Ok(front_matter) => front_matter,
        Err(error) => {
            dialogs::info("输入无效", &error.to_string());
            return;
        }
    };

    match run_save(services, &project, &form, &front_matter) {
        SaveFlow::Cancelled => {}
        SaveFlow::Reload(path) => load_into_form(services, &path),
        SaveFlow::Failed(message) => dialogs::info("保存失败", &message),
        SaveFlow::Saved {
            file_path,
            warnings,
        } => {
            let hash = services.save_use_case.content_hash(&file_path).ok().flatten();
            services.edit_form(|form| form.apply_saved(file_path.clone(), hash));

            if !warnings.is_empty() {
                dialogs::info("已保存（有警告）", &warnings.join("\r\n"));
            }
            dialogs::info(
                "保存成功",
                &format!(
                    "博文已保存到 {}",
                    relative_display(project.path(), &file_path)
                ),
            );
        }
    }
}

/// 冲突重试循环在用例内，这里只编排「外部修改 → 二次确认 → 重存」这一层外层重试。
fn run_save(
    services: &Services,
    project: &BlogProject,
    form: &PostForm,
    front_matter: &FrontMatter,
) -> SaveFlow {
    let prompts = UiSavePrompts;
    let original = form.original_file_path.as_deref();
    // 正文没被本工具碰过就不交给保存：更新路径据此保留磁盘最新 body（FR-3.10），
    // 新建路径没有磁盘正文可保留，恒写表单值。
    let body = (original.is_none() || form.body_dirty).then_some(form.body.as_str());

    let result = match services.save_use_case.save(
        project,
        front_matter,
        body,
        &prompts,
        original,
        form.original_content_hash.as_deref(),
    ) {
        Ok(result) => result,
        Err(error) => return SaveFlow::Failed(error.to_string()),
    };

    if !result.is_modified_externally() {
        return classify(result);
    }

    let keep_going = dialogs::confirm_with(
        "文件已被外部修改",
        "该博文在磁盘上已被其他程序修改。继续保存将覆盖外部修改。建议选择“取消并刷新”以加载最新内容。",
        "继续保存",
        "取消并刷新",
    );
    if !keep_going {
        return match original {
            Some(path) => SaveFlow::Reload(path.to_path_buf()),
            None => SaveFlow::Cancelled,
        };
    }

    // 覆盖外部修改：置空基线哈希后重存
    match services.save_use_case.save(
        project,
        front_matter,
        body,
        &prompts,
        original,
        None,
    ) {
        Ok(result) => classify(result),
        Err(error) => SaveFlow::Failed(error.to_string()),
    }
}

fn classify(result: PostOperationResult) -> SaveFlow {
    if result.is_success() {
        return match result.file_path() {
            Some(path) => SaveFlow::Saved {
                file_path: path.to_path_buf(),
                warnings: result.warnings().to_vec(),
            },
            None => SaveFlow::Cancelled,
        };
    }

    // 用户在冲突对话框里选了「取消」，或放弃了覆盖确认——不再额外提示
    if result.is_conflict() {
        return SaveFlow::Cancelled;
    }

    SaveFlow::Failed(
        result
            .errors()
            .iter()
            .map(|error| error.message.clone())
            .collect::<Vec<_>>()
            .join("\r\n"),
    )
}

/// 导入 markdown 正文（FR-4.3：默认追加到光标处，勾选后整体替换）。
///
/// `caret` 是 `TextBox.SelectionStart`，即 **UTF-16 码元** 计数，这里负责换算成 char 索引。
pub fn import_body(services: Arc<Services>, caret: Option<usize>, refresh: Refresh) {
    spawn_work(
        move || {
            let Some(path) = pickers::pick_markdown_file() else {
                return;
            };
            let Ok(content) = std::fs::read_to_string(&path) else {
                dialogs::info("导入失败", "文件不存在、被占用或不是文本文件。");
                return;
            };
            let Ok((_, imported)) = post_file_format::parse(&content) else {
                dialogs::info("导入失败", "无法解析该 markdown 文件。");
                return;
            };

            services.edit_form(|form| {
                let body = if form.replace_body_on_import {
                    imported
                } else {
                    insert_at_caret(&form.body, &imported, caret)
                };
                form.set_body(body);
            });
        },
        move |_: ()| notify(&refresh),
    )
}

/// 插入图片（FR-6.x）：复制到 `assets/img/{slug}/`，markdown 引用插到光标处。
pub fn insert_images(services: Arc<Services>, caret: Option<usize>, refresh: Refresh) {
    spawn_work(
        move || {
            let Some(project) = services.project.current() else {
                return;
            };
            let form = services.form();
            let slug = form.current_slug();
            if slug.trim().is_empty() {
                return;
            }

            let picked = pickers::pick_images(&IMAGE_EXTENSIONS);
            if picked.is_empty() {
                return;
            }

            // 复制图片是文件 IO：只读快照里的 alt，不持锁
            let markdown = match image_inserter::insert(&project, &slug, &picked, &form.alt_text)
            {
                Ok(markdown) if !markdown.is_empty() => markdown,
                Ok(_) => {
                    dialogs::info("未插入", "未找到有效的图片文件。");
                    return;
                }
                Err(error) => {
                    dialogs::info("插入失败", &error.to_string());
                    return;
                }
            };

            services.edit_form(|form| {
                form.set_body(insert_at_caret(&form.body, &markdown, caret));
            });
            dialogs::info("插入成功", "markdown 引用已插入到光标处。");
        },
        move |_: ()| notify(&refresh),
    )
}

/// AI 提取关键字（FR-7.x）：结果直接替换标签输入（FR-7.4）。
pub fn extract_keywords(services: Arc<Services>, refresh: Refresh) {
    if services.form().is_extracting_keywords {
        return;
    }
    services.edit_form(|form| form.is_extracting_keywords = true);
    refresh();

    let finishing = Arc::clone(&services);
    spawn_work(
        move || extract_blocking(&services),
        move |_: ()| {
            finishing.edit_form(|form| form.is_extracting_keywords = false);
            notify(&refresh);
        },
    )
}

fn extract_blocking(services: &Services) {
    let body = services.form().body;
    if body.trim().is_empty() {
        dialogs::info("正文为空", "请先填写或导入正文后再提取关键字。");
        return;
    }

    match services.ai.extract_keywords(&body, MAX_KEYWORDS) {
        Ok(keywords) if !keywords.is_empty() => {
            services.edit_form(|form| {
                form.state.tags = keywords.join(" ");
                form.update_preview();
            });
        }
        Ok(_) => dialogs::info(
            "未提取到关键字",
            "AI 未返回有效关键字，请检查正文或稍后重试。",
        ),
        Err(error) => dialogs::info("提取失败", &error.to_string()),
    }
}

fn insert_at_caret(body: &str, markdown: &str, caret: Option<usize>) -> String {
    let cursor = caret.and_then(|units| utf16_index_to_char_index(body, units));
    insert_at_cursor(body, markdown, cursor)
}

/// 文件是否落在项目 `_posts/` 目录下（对偶 `Path.GetFullPath` + 不区分大小写比较）。
///
/// 两侧都能被文件系统解析时用 `canonicalize`（顺带处理符号链接与 8.3 短名）；
/// 任一侧解析失败（目录刚被删除等）退回按字面比较，宁可多问一次。
fn in_posts_directory(file_path: &Path, posts_directory: &Path) -> bool {
    let Some(parent) = file_path.parent() else {
        return false;
    };
    same_directory(parent, posts_directory)
}

fn same_directory(candidate: &Path, expected: &Path) -> bool {
    match (std::fs::canonicalize(candidate), std::fs::canonicalize(expected)) {
        (Ok(candidate), Ok(expected)) => candidate == expected,
        _ => trim_separators(candidate).eq_ignore_ascii_case(&trim_separators(expected)),
    }
}

fn trim_separators(path: &Path) -> String {
    path.to_string_lossy().trim_end_matches(['\\', '/']).to_owned()
}

enum SaveFlow {
    Cancelled,
    Reload(PathBuf),
    Failed(String),
    Saved {
        file_path: PathBuf,
        warnings: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("jp-app-post-{tag}"));
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn file_inside_posts_directory_is_accepted() {
        let posts = temp_dir("inside");
        let file = posts.join("2026-07-28-a.md");

        assert!(in_posts_directory(&file, &posts));
    }

    #[test]
    fn file_in_sibling_directory_is_rejected() {
        let posts = temp_dir("sibling-a");
        let other = temp_dir("sibling-b");

        assert!(!in_posts_directory(&other.join("a.md"), &posts));
    }

    #[test]
    fn comparison_ignores_case() {
        let posts = temp_dir("casing");
        let shuffled = PathBuf::from(posts.to_string_lossy().to_uppercase());

        // 目录真实存在的一侧走 canonicalize（大小写已被文件系统归一），
        // 另一侧不存在时走字面比较，两条路径都必须判定为同一目录
        assert!(same_directory(&shuffled, &posts));
    }

    #[test]
    fn trailing_separators_do_not_matter() {
        assert_eq!(
            trim_separators(Path::new("C:\\blog\\_posts\\")),
            "C:\\blog\\_posts"
        );
    }

    #[test]
    fn caret_conversion_falls_back_to_append_when_out_of_range() {
        // UTF-16 码元超出正文长度时按「追加到末尾」处理（与用例的 None 分支一致）
        assert_eq!(
            insert_at_caret("正文", "![a](/b.png)", Some(99)),
            "正文\r\n![a](/b.png)"
        );
    }

    #[test]
    fn caret_conversion_inserts_at_utf16_position() {
        // "😀" 占 2 个 UTF-16 码元（位 3、4）：码元位 5 即 emoji 之后，对应 char 位 4
        let body = "abc😀def";

        assert_eq!(insert_at_caret(body, "|", Some(5)), "abc😀\r\n|\r\ndef");
        // 落在代理对内部（码元位 4）无法换算：退回追加到末尾
        assert_eq!(insert_at_caret(body, "|", Some(4)), "abc😀def\r\n|");
    }
}
