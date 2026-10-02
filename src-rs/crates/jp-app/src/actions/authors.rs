//! 作者管理命令（对偶 `AuthorsPageViewModel` 的五个 RelayCommand）。
//!
//! 列表与编辑态都在页面的 [`PageState<AuthorsPage>`] 里；动作只在工作线程改它，
//! 改完经 [`notify`] 回 UI 线程把值写回控件（Rust 侧没有双向绑定）。
//! 校验（`Author::new`）在持锁期间做，但只做纯计算，IO 一律出锁。

use std::sync::Arc;

use jp_domain::authors::Author;

use super::{PageState, Refresh, held, notify};
use crate::runtime::spawn_work;
use crate::services::{dialogs, Services};
use crate::view_models::AuthorsPage;
/// 作者管理页的列表加载（对偶 `LoadAuthorsAsync`）。
pub fn load(services: Arc<Services>, state: PageState<AuthorsPage>, refresh: Refresh) {
    spawn_work(
        move || read_list(&services, &state),
        move |_: ()| notify(&refresh),
    )
}

/// 博文页作者多选用的候选列表（对偶 `PostPageViewModel` 的 `LoadAuthorsAsync`）。
///
/// 与作者管理页分开：这里只要 `Vec<Author>`，编辑态在共享的 `PostForm` 里，
/// 所以复用那份页面状态会把两个页面的字段搅在一起。
pub fn available(services: Arc<Services>, state: PageState<Vec<Author>>, refresh: Refresh) {
    spawn_work(
        move || {
            held(&state).clear();
            if services.project.current().is_none() {
                return;
            }
            match services.author_repository.get_all() {
                Ok(authors) => *held(&state) = authors,
                Err(error) => dialogs::info("加载作者失败", &error.to_string()),
            }
        },
        move |_: ()| notify(&refresh),
    )
}

/// 列表选中项变化（对偶 `OnSelectedAuthorChanged`）。
///
/// 与 .NET 侧一致：只在选中实体时回填字段，取消选中不动编辑态。
/// 纯状态操作，由 UI 线程的事件处理器直接调用。
pub fn apply_selection(state: &PageState<AuthorsPage>, author: Option<&Author>) {
    if let Some(author) = author {
        held(state).draft.select(author);
    }
}

/// 新增作者：进入可编辑空表单（对偶 `AddAuthor`）。
pub fn begin_new(state: PageState<AuthorsPage>, refresh: Refresh) {
    held(&state).draft.begin_new();
    refresh();
}

/// 取消编辑，回到未选中态（对偶 `Cancel`）。
pub fn cancel(state: PageState<AuthorsPage>, refresh: Refresh) {
    held(&state).draft.clear();
    refresh();
}

/// 保存（新增或更新）作者（对偶 `SaveAuthorAsync`）。
pub fn save(services: Arc<Services>, state: PageState<AuthorsPage>, refresh: Refresh) {
    spawn_work(
        move || {
            if services.project.current().is_none() {
                dialogs::info("未选择项目", "请先选择一个博客项目。");
                return;
            }

            let Some((author, is_new)) = draft_of(&state) else {
                return;
            };

            let outcome = if is_new {
                services.author_use_case.add(&author)
            } else {
                services.author_use_case.update(&author)
            };

            // 保存失败要说得出原因（id 重复 / 作者不存在 / 写盘失败）
            if let Err(error) = outcome {
                dialogs::info("保存失败", &error.to_string());
                return;
            }

            // 成功后重读列表：与 .NET 相同，编辑态一并清空
            read_list(&services, &state);
        },
        move |_: ()| notify(&refresh),
    )
}

/// 删除当前选中作者（对偶 `DeleteAuthorAsync`）。
pub fn delete(services: Arc<Services>, state: PageState<AuthorsPage>, refresh: Refresh) {
    spawn_work(
        move || {
            let Some(id) = held(&state).draft.selected_id.clone() else {
                return;
            };

            if !dialogs::confirm_with("删除作者", &delete_message(&id), "删除", "取消") {
                return;
            }

            if let Err(error) = services.author_use_case.delete(&id) {
                dialogs::info("删除失败", &error.to_string());
                return;
            }

            read_list(&services, &state);
        },
        move |_: ()| notify(&refresh),
    )
}

/// 取本次提交用的作者。判定顺序与 .NET 一致：先构造实体（校验），再看新增还是更新，
/// 两者都不是（未选中且非新增）才静默空操作。校验失败就地弹「输入无效」。
///
/// 编辑态先克隆再离开锁区：对话框要阻塞等用户点击，绝不能带着锁等。
fn draft_of(state: &PageState<AuthorsPage>) -> Option<(Author, bool)> {
    let draft = held(state).draft.clone();

    let author = match draft.to_author() {
        Ok(author) => author,
        Err(error) => {
            dialogs::info("输入无效", &error.to_string());
            return None;
        }
    };

    (draft.is_new || draft.selected_id.is_some()).then_some((author, draft.is_new))
}

/// 读列表：失败只提示、不清选择（`authors.yml` 损坏时作者页不会永远空白无提示）。
/// 未选项目时按 .NET 语义只清空列表、保留编辑态。
fn read_list(services: &Services, state: &PageState<AuthorsPage>) {
    held(state).authors.clear();

    if services.project.current().is_none() {
        return;
    }

    match services.author_repository.get_all() {
        Ok(authors) => held(state).authors = authors,
        Err(error) => dialogs::info("加载作者失败", &error.to_string()),
    }

    // 与 .NET 相同：读取流程收尾必定清空编辑态
    held(state).draft.clear();
}

fn delete_message(id: &str) -> String {
    format!("确定要删除作者 {id} 吗？此操作不可撤销。")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use crate::view_models::AuthorsState;

    fn author(id: &str, name: &str) -> Author {
        Author::new(id, name, None, None).unwrap()
    }

    #[test]
    fn delete_message_names_the_author_id() {
        assert_eq!(
            delete_message("cotes"),
            "确定要删除作者 cotes 吗？此操作不可撤销。"
        );
    }

    #[test]
    fn draft_of_requires_selection_or_new_flag() {
        let state: PageState<AuthorsPage> = Arc::new(Mutex::default());
        assert!(draft_of(&state).is_none());

        held(&state).draft.id = "cotes".to_owned();
        held(&state).draft.name = "Cotes".to_owned();
        // 既非新增也无选中项：提交是空操作（对话框在无 UI 线程时会被丢弃，不阻塞）
        assert!(draft_of(&state).is_none());

        held(&state).draft.is_new = true;
        let (author, is_new) = draft_of(&state).unwrap();
        assert!(is_new);
        assert_eq!(author.id(), "cotes");
    }

    #[test]
    fn existing_draft_updates_with_locked_id() {
        let state: PageState<AuthorsPage> = Arc::new(Mutex::default());
        {
            let mut page = held(&state);
            page.draft = AuthorsState {
                selected_id: Some("cotes".to_owned()),
                id: "cotes".to_owned(),
                name: "Cotes 2020".to_owned(),
                ..Default::default()
            };
        }

        let (author, is_new) = draft_of(&state).unwrap();
        assert!(!is_new);
        assert_eq!(author.name(), "Cotes 2020");
    }

    #[test]
    fn apply_selection_ignores_deselection() {
        let state: PageState<AuthorsPage> = Arc::new(Mutex::default());
        held(&state).draft.id = "typed".to_owned();

        apply_selection(&state, None);
        assert_eq!(held(&state).draft.id, "typed");

        apply_selection(&state, Some(&author("cotes", "Cotes")));
        assert_eq!(held(&state).draft.id, "cotes");
        assert!(held(&state).draft.id_locked());
    }
}
