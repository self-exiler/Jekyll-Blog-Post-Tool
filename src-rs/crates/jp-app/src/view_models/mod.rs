//! 页面级状态（对偶 .NET 侧 `*PageViewModel` 的可观察字段部分）。
//!
//! Rust 投影没有 `{x:Bind}`，页面进入时把这里的状态写进控件、执行动作前把控件值拉回状态；
//! 因此这些结构体只做「存储 + 派生判定」，不做属性通知。

use jp_application::ai::AiSettings;
use jp_domain::authors::Author;
use jp_domain::common::error::DomainResult;

pub mod post_form;

pub use post_form::PostForm;

/// 项目页路径占位文案（对应 `ProjectPageViewModel.ProjectPath` 初值）。
pub const PROJECT_PATH_PLACEHOLDER: &str = "未选择项目";

/// 博文头信息页的作者多选项（对偶 `AuthorOption`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorOption {
    pub id: String,
    /// 列表展示文本：`"{id} ({name})"`。
    pub display_name: String,
    pub is_selected: bool,
}

impl AuthorOption {
    pub fn from_author(author: &Author, selected: &[String]) -> Self {
        Self {
            id: author.id().to_owned(),
            display_name: format!("{} ({})", author.id(), author.name()),
            is_selected: selected.iter().any(|id| id == author.id()),
        }
    }
}

/// 作者管理页的编辑态（对偶 `AuthorsPageViewModel`）。
///
/// 与 .NET 侧一致：`selected_id` 非空即「选中既有作者」，此时 id 锁定、删除可用。
#[derive(Debug, Clone, Default)]
pub struct AuthorsState {
    pub selected_id: Option<String>,
    pub is_new: bool,
    pub id: String,
    pub name: String,
    pub twitter: String,
    pub url: String,
}

impl AuthorsState {
    /// `HasSelection`：列表里有选中项，或正处于新增流程。
    /// 暂只被测试引用：保留对偶 `AuthorsPageViewModel.HasSelection` 的完整语义。
    #[allow(dead_code)]
    pub fn has_selection(&self) -> bool {
        self.selected_id.is_some() || self.is_new
    }

    /// .NET 侧的 `IsReadOnly`：id 输入框只读 + 删除按钮可用，两处共用同一判定。
    pub fn id_locked(&self) -> bool {
        !self.is_new && self.selected_id.is_some()
    }

    /// 新增作者：清空字段但进入可编辑态。
    pub fn begin_new(&mut self) {
        self.clear();
        self.is_new = true;
    }

    pub fn select(&mut self, author: &Author) {
        self.selected_id = Some(author.id().to_owned());
        self.is_new = false;
        self.id = author.id().to_owned();
        self.name = author.name().to_owned();
        self.twitter = author.twitter().unwrap_or_default().to_owned();
        self.url = author.url().unwrap_or_default().to_owned();
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn to_author(&self) -> DomainResult<Author> {
        Author::new(&self.id, &self.name, Some(&self.twitter), Some(&self.url))
    }
}

/// 作者管理页状态（对偶 `AuthorsPageViewModel` 的全部字段）：
/// 列表 + 编辑态。`clear` 只清编辑态，列表由 `authors` 自己管。
#[derive(Debug, Clone, Default)]
pub struct AuthorsPage {
    pub authors: Vec<Author>,
    pub draft: AuthorsState,
}

/// 高级页的 AI 配置表单（对偶 `AdvancedPageViewModel`）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AiForm {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl AiForm {
    pub fn apply_settings(&mut self, settings: &AiSettings) {
        self.base_url = settings.base_url.clone();
        self.api_key = settings.api_key.clone();
        self.model = settings.model.clone();
    }

    /// 保存前统一 trim（与 .NET 侧 `AiBaseUrl.Trim()` 一致）。
    pub fn to_settings(&self) -> AiSettings {
        AiSettings {
            base_url: self.base_url.trim().to_owned(),
            api_key: self.api_key.trim().to_owned(),
            model: self.model.trim().to_owned(),
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn author(id: &str, name: &str, twitter: Option<&str>) -> Author {
        Author::new(id, name, twitter, Some(" http://x.test ")).unwrap()
    }

    #[test]
    fn author_option_uses_id_and_name_display() {
        let option = AuthorOption::from_author(
            &author("cotes", "Cotes", None),
            &["cotes".to_owned(), "admin".to_owned()],
        );

        assert_eq!(option.display_name, "cotes (Cotes)");
        assert!(option.is_selected);
    }

    #[test]
    fn author_option_unselected_when_absent() {
        let option = AuthorOption::from_author(&author("dio", "Dio", None), &["cotes".to_owned()]);
        assert!(!option.is_selected);
    }

    #[test]
    fn selecting_author_locks_id_and_enables_delete() {
        let mut state = AuthorsState::default();
        assert!(!state.has_selection());
        assert!(!state.id_locked());

        state.select(&author("cotes", "Cotes", Some(" cotes201 ")));

        assert_eq!(state.selected_id.as_deref(), Some("cotes"));
        assert_eq!(state.twitter, "cotes201");
        assert_eq!(state.url, "http://x.test");
        assert!(state.has_selection());
        assert!(state.id_locked());
    }

    #[test]
    fn new_author_is_editable_with_unlocked_id() {
        let mut state = AuthorsState::default();
        state.select(&author("cotes", "Cotes", None));

        state.begin_new();

        assert!(state.has_selection());
        assert!(!state.id_locked());
        assert!(state.id.is_empty());
    }

    #[test]
    fn draft_trims_and_blanks_through_author_entity() {
        let mut state = AuthorsState::default();
        state.id = " cotes ".to_owned();
        state.name = " Cotes ".to_owned();
        state.twitter = "   ".to_owned();

        let author = state.to_author().unwrap();
        assert_eq!(author.id(), "cotes");
        assert_eq!(author.twitter(), None);

        state.name.clear();
        assert!(state.to_author().is_err());
    }

    #[test]
    fn ai_form_trims_on_save_and_clears_on_reset() {
        let mut form = AiForm {
            base_url: "  https://api.test/v1 ".to_owned(),
            api_key: "  key ".to_owned(),
            model: "  gpt ".to_owned(),
        };

        let settings = form.to_settings();
        assert_eq!(settings.base_url, "https://api.test/v1");
        assert_eq!(settings.api_key, "key");
        assert_eq!(settings.model, "gpt");

        form.apply_settings(&AiSettings::default());
        assert_eq!(form, AiForm::default());
    }
}
