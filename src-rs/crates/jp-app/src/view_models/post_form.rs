//! 博文编辑状态（对偶 .NET `PostPageViewModel` 的可观察字段部分）。
//!
//! Rust 投影没有 `{x:Bind}`，所以这里不做属性通知：页面进入时把状态写进控件，
//! 执行动作前把控件值拉回状态，派生显示值（文件名预览、front matter 预览、
//! 图片目标目录、作者摘要）由本模块重算后交给页面刷新。

use std::path::PathBuf;

use chrono::{Local, NaiveDate, NaiveTime};
use jp_application::posts::{time_zone, LoadedPost, PostFormState};
use jp_domain::posts::{generate_slug, Post, PostDate};
use jp_infrastructure::posts::post_file_format;

pub const DEFAULT_PAGE_TITLE: &str = "博文";
pub const PAGE_TITLE_PREFIX: &str = "博文 - ";
pub const FILE_NAME_PLACEHOLDER: &str = "填写标题后生成文件名";
pub const NO_AUTHOR_DISPLAY: &str = "无作者";
pub const NO_PROJECT_DISPLAY: &str = "（未选项目）";

/// 新建 / 打开 / 保存共享的一份博文编辑态（对应 `App.Current.PostPageViewModel` 单例）。
#[derive(Debug, Clone)]
pub struct PostForm {
    pub state: PostFormState,
    pub body: String,
    /// 正文自上次落盘/加载后是否被本工具改过（FR-3.10 与 设计方案 v1.5 的接缝）：
    /// 没改过就不把正文交给保存，从而保留外部编辑器写入的最新正文。
    pub body_dirty: bool,
    /// `None` 表示尚未落盘的新建博文。
    pub original_file_path: Option<PathBuf>,
    pub original_content_hash: Option<String>,
    pub file_name_preview: String,
    pub front_matter_preview: String,
    pub page_title: String,
    /// FR-4.3：导入正文时替换而非追加。
    pub replace_body_on_import: bool,
    /// FR-6.5：本次插入所有图片共用的 alt 文本。
    pub alt_text: String,
    pub selected_author_ids: Vec<String>,
    /// 保存进行中（`CanSave`）。
    pub is_busy: bool,
    /// AI 提取进行中（`CanExtractKeywords`，FR-7.3）。
    pub is_extracting_keywords: bool,
}

impl PostForm {
    /// 新建博文的空白状态（FR-3.1：date 不预填）。
    pub fn empty(now: PostDate) -> Self {
        let mut form = Self {
            state: PostFormState::empty(now),
            body: String::new(),
            body_dirty: false,
            original_file_path: None,
            original_content_hash: None,
            file_name_preview: String::new(),
            front_matter_preview: String::new(),
            page_title: DEFAULT_PAGE_TITLE.to_owned(),
            replace_body_on_import: false,
            alt_text: String::new(),
            selected_author_ids: Vec::new(),
            is_busy: false,
            is_extracting_keywords: false,
        };
        form.update_preview();
        form
    }

    /// 本机当前时间（`DateTimeOffset.Now` 的对偶，供 `empty` 取默认时间/时区）。
    pub fn now() -> PostDate {
        Local::now().fixed_offset()
    }

    /// 新建：清空编辑态但保留时间与时区默认值。
    pub fn reset(&mut self, now: PostDate) {
        let replace_body_on_import = self.replace_body_on_import;
        let alt_text = std::mem::take(&mut self.alt_text);
        *self = Self {
            replace_body_on_import,
            alt_text,
            ..Self::empty(now)
        };
    }

    /// 改写正文的唯一入口：值真的变了才置脏。
    ///
    /// 页面把表单值推回控件时会重入 `TextChanged`，比对旧值可让这种回写保持幂等，
    /// 不至于「只是切了一页就把正文标成已编辑」。
    pub fn set_body(&mut self, body: String) {
        if self.body != body {
            self.body = body;
            self.body_dirty = true;
        }
    }

    /// 从磁盘读取结果回填编辑态（`LoadPostAsync` 的状态部分）。
    pub fn apply_loaded(&mut self, loaded: &LoadedPost) {
        let post = &loaded.post;
        let front_matter = post.front_matter();
        self.state = PostFormState::from_front_matter(front_matter);
        self.selected_author_ids = front_matter.authors.clone();
        self.body = post.body().to_owned();
        self.body_dirty = false;
        self.original_file_path = Some(post.file_path().to_path_buf());
        // 单次读盘：展示内容与基线哈希同源
        self.original_content_hash = Some(loaded.content_hash.clone());
        self.page_title = format!("{PAGE_TITLE_PREFIX}{}", post.file_name());
        self.update_preview();
    }

    /// 保存成功后改写「已落盘」身份。
    pub fn apply_saved(&mut self, file_path: PathBuf, content_hash: Option<String>) {
        // 走到这里说明正文已按本次保存的口径落盘，脏态归零
        self.body_dirty = false;
        self.page_title = format!(
            "{PAGE_TITLE_PREFIX}{}",
            jp_domain::common::paths::file_name(&file_path)
        );
        self.original_file_path = Some(file_path);
        self.original_content_hash = content_hash;
    }

    /// 当前 slug：优先从已保存文件名提取，否则按标题生成（ADR-006：中文原样保留）。
    pub fn current_slug(&self) -> String {
        if let Some(path) = &self.original_file_path {
            return Post::try_extract_slug(&jp_domain::common::paths::file_name(path));
        }
        generate_slug(&self.state.title)
            .map(|slug| slug.value().to_owned())
            .unwrap_or_default()
    }

    /// 图片资源目录的相对展示路径（FR-6.1）。
    pub fn target_directory(&self, project_selected: bool) -> String {
        if !project_selected {
            return NO_PROJECT_DISPLAY.to_owned();
        }
        format!("assets/img/{}/", self.current_slug())
    }

    /// 允许插入图片：已选项目且能确定 slug。
    pub fn can_insert_images(&self, project_selected: bool) -> bool {
        project_selected && !self.current_slug().trim().is_empty()
    }

    /// 作者选择摘要文本。
    pub fn authors_display(&self) -> String {
        if self.selected_author_ids.is_empty() {
            return NO_AUTHOR_DISPLAY.to_owned();
        }
        self.selected_author_ids.join(", ")
    }

    pub fn set_selected_authors(&mut self, ids: Vec<String>) {
        self.selected_author_ids = ids;
        self.state.selected_author_ids = self.selected_author_ids.clone();
    }

    /// 表单 → front matter（校验与转换规则都在 `PostFormState`）。
    pub fn to_front_matter(
        &self,
    ) -> jp_application::error::ApplicationResult<jp_domain::posts::FrontMatter> {
        let mut state = self.state.clone();
        state.selected_author_ids = self.selected_author_ids.clone();
        state.to_front_matter()
    }

    /// 防抖到期后重算文件名与 front matter 预览。
    ///
    /// 预览与落盘共用 `post_file_format::format`，所见即所得由构造保证。
    pub fn update_preview(&mut self) {
        let Ok(front_matter) = self.to_front_matter() else {
            self.file_name_preview = FILE_NAME_PLACEHOLDER.to_owned();
            self.front_matter_preview = String::new();
            return;
        };

        let Ok(slug) = generate_slug(&front_matter.title) else {
            self.file_name_preview = FILE_NAME_PLACEHOLDER.to_owned();
            self.front_matter_preview = String::new();
            return;
        };

        self.file_name_preview = match &front_matter.date {
            Some(date) => Post::build_file_name(date, &slug),
            None => format!("{}.md", slug.value()),
        };
        self.front_matter_preview = post_file_format::format(&front_matter, None);
    }

    /// 时间/时区输入框用的 `HH:mm` 文本。
    pub fn time_text(&self) -> String {
        self.state.selected_time.format("%H:%M").to_string()
    }

    pub fn set_time_text(&mut self, text: &str) {
        if let Some(time) = parse_time(text) {
            self.state.selected_time = time;
        }
    }

    pub fn date_text(&self) -> String {
        self.state
            .selected_date
            .map(|date| date.format("%Y-%m-%d").to_string())
            .unwrap_or_default()
    }

    pub fn set_date_text(&mut self, text: &str) {
        self.state.selected_date = NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok();
    }

    pub fn time_zone_options(&self) -> Vec<String> {
        time_zone::build_options()
    }

    /// 子分类输入框是否可用（对偶 `HasCategory1`）。
    pub fn has_category1(&self) -> bool {
        !self.state.category1.trim().is_empty()
    }

    /// 当前时区在下拉候选里的下标；`front matter` 里的偏移量不在候选表内时为 `None`（未选中）。
    pub fn time_zone_index(&self) -> Option<usize> {
        self.time_zone_options()
            .iter()
            .position(|option| option == &self.state.selected_time_zone)
    }

    /// 按下标写回时区；下标越界（含 `None`）保持原值，与「未选中不动」的 UI 语义一致。
    pub fn set_time_zone_index(&mut self, index: Option<usize>) {
        if let Some(option) = index.and_then(|index| time_zone::build_options().get(index).cloned())
        {
            self.state.selected_time_zone = option;
        }
    }
}

/// 接受 `9:30` / `09:30` / `09:30:00`；无法解析时保持原值（对应 .NET 侧 `TimeSpan` 解析失败即忽略）。
fn parse_time(text: &str) -> Option<NaiveTime> {
    let text = text.trim();
    const FORMATS: [&str; 2] = ["%H:%M:%S", "%H:%M"];

    if let Some(time) = FORMATS.iter().find_map(|format| NaiveTime::parse_from_str(text, format).ok()) {
        return Some(time);
    }

    // 单位数小时：chrono 的 %H 要求两位，补零后重试
    if text.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        let padded = format!("0{text}");
        return FORMATS
            .iter()
            .find_map(|format| NaiveTime::parse_from_str(&padded, format).ok());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    fn date_at(y: i32, m: u32, d: u32, h: u32, min: u32, offset_hours: i32) -> PostDate {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
            .and_local_timezone(FixedOffset::east_opt(offset_hours * 3600).unwrap())
            .single()
            .unwrap()
    }

    #[test]
    fn empty_defers_file_name_until_title_exists() {
        let form = PostForm::empty(date_at(2026, 7, 28, 9, 30, 8));

        assert_eq!(form.file_name_preview, FILE_NAME_PLACEHOLDER);
        assert_eq!(form.page_title, DEFAULT_PAGE_TITLE);
        assert_eq!(form.authors_display(), NO_AUTHOR_DISPLAY);
    }

    #[test]
    fn preview_without_date_shows_slug_only() {
        let mut form = PostForm::empty(date_at(2026, 7, 28, 9, 30, 8));
        form.state.title = "Hello World".to_owned();
        form.update_preview();

        assert_eq!(form.file_name_preview, "hello-world.md");
        assert!(form.front_matter_preview.contains("title: Hello World"));
        // date 未填时不落 date 行
        assert!(!form.front_matter_preview.contains("date:"));
    }

    #[test]
    fn preview_with_date_matches_file_name_on_disk() {
        let mut form = PostForm::empty(date_at(2026, 7, 28, 9, 30, 8));
        form.state.title = "你好世界".to_owned();
        form.set_date_text("2026-07-28");
        form.update_preview();

        assert_eq!(form.file_name_preview, "2026-07-28-你好世界.md");
        assert_eq!(form.current_slug(), "你好世界");
    }

    #[test]
    fn slug_prefers_saved_file_name_over_title() {
        let mut form = PostForm::empty(date_at(2026, 7, 28, 9, 30, 8));
        form.original_file_path = Some(PathBuf::from("2026-01-01-saved-slug.md"));
        form.state.title = "另一个标题".to_owned();

        assert_eq!(form.current_slug(), "saved-slug");
    }

    #[test]
    fn target_directory_requires_project() {
        let mut form = PostForm::empty(date_at(2026, 7, 28, 9, 30, 8));
        form.state.title = "Doc".to_owned();

        assert_eq!(form.target_directory(false), NO_PROJECT_DISPLAY);
        assert!(!form.can_insert_images(false));
        assert_eq!(form.target_directory(true), "assets/img/doc/");
        assert!(form.can_insert_images(true));
    }

    #[test]
    fn time_text_round_trip_accepts_short_hour() {
        let mut form = PostForm::empty(date_at(2026, 7, 28, 9, 30, 8));
        assert_eq!(form.time_text(), "09:30");

        form.set_time_text("9:05");
        assert_eq!(form.time_text(), "09:05");

        form.set_time_text("not a time");
        assert_eq!(form.time_text(), "09:05");
    }

    #[test]
    fn selected_authors_flow_into_front_matter() {
        let mut form = PostForm::empty(date_at(2026, 7, 28, 9, 30, 8));
        form.set_selected_authors(vec!["cotes".to_owned(), "admin".to_owned()]);

        assert_eq!(form.authors_display(), "cotes, admin");
        assert_eq!(form.to_front_matter().unwrap().authors, vec!["cotes", "admin"]);
    }

    #[test]
    fn body_marks_dirty_only_when_it_actually_changes() {
        let mut form = PostForm::empty(date_at(2026, 7, 28, 9, 30, 8));
        assert!(!form.body_dirty);

        // 表单值推回控件会重入 TextChanged，回写同值不该被当成「用户编辑过」
        form.set_body("正文".to_owned());
        assert!(form.body_dirty);

        form.apply_saved(PathBuf::from("C:\\blog\\_posts\\2026-07-28-a.md"), Some("hash".to_owned()));
        assert!(!form.body_dirty);

        form.set_body("正文".to_owned());
        assert!(!form.body_dirty);
    }

    #[test]
    fn reset_keeps_ui_preferences_only() {
        let mut form = PostForm::empty(date_at(2026, 7, 28, 9, 30, 8));
        form.state.title = "Old".to_owned();
        form.body = "text".to_owned();
        form.alt_text = "图1".to_owned();
        form.replace_body_on_import = true;

        form.reset(date_at(2026, 7, 29, 10, 0, 8));

        assert_eq!(form.state.title, "");
        assert_eq!(form.body, "");
        assert_eq!(form.alt_text, "图1");
        assert!(form.replace_body_on_import);
        assert_eq!(form.page_title, DEFAULT_PAGE_TITLE);
    }

    #[test]
    fn apply_saved_renames_post_identity() {
        let mut form = PostForm::empty(date_at(2026, 7, 28, 9, 30, 8));
        form.apply_saved(PathBuf::from("C:\\blog\\_posts\\2026-07-28-a.md"), Some("hash".to_owned()));

        assert_eq!(form.page_title, "博文 - 2026-07-28-a.md");
        assert_eq!(form.original_content_hash.as_deref(), Some("hash"));
    }

    #[test]
    fn front_matter_preview_is_byte_identical_to_disk() {
        let mut form = PostForm::empty(date_at(2026, 7, 28, 14, 10, 8));
        form.state.title = "Test".to_owned();
        form.set_date_text("2026-07-28");
        form.update_preview();

        let expected = post_file_format::format(&form.to_front_matter().unwrap(), None);
        assert_eq!(form.front_matter_preview, expected);
    }

    #[test]
    fn blank_front_matter_falls_back_to_placeholder() {
        let mut form = PostForm::empty(date_at(2026, 7, 28, 9, 30, 8));
        form.state.title = "   ".to_owned();
        form.update_preview();

        assert_eq!(form.file_name_preview, FILE_NAME_PLACEHOLDER);
        assert_eq!(form.front_matter_preview, "");
    }

    #[test]
    fn time_zone_index_round_trips_through_the_combo() {
        let mut form = PostForm::empty(date_at(2026, 7, 28, 9, 30, 8));
        assert_eq!(form.time_zone_index(), Some(40)); // +08:00 起点是 -12:00

        form.set_time_zone_index(Some(0));
        assert_eq!(form.state.selected_time_zone, "-12:00");

        // 未选中（下标 None）不改写已存值
        form.set_time_zone_index(None);
        assert_eq!(form.state.selected_time_zone, "-12:00");
    }

    #[test]
    fn subcategory_needs_a_main_category() {
        let mut form = PostForm::empty(date_at(2026, 7, 28, 9, 30, 8));
        assert!(!form.has_category1());

        form.state.category1 = "  ".to_owned();
        assert!(!form.has_category1());

        form.state.category1 = "技术".to_owned();
        assert!(form.has_category1());
    }

    #[test]
    fn form_mapping_drops_unknown_front_matter_fields() {
        // 记录既有行为：表单态 ↔ front matter 的映射不含未识别字段，
        // 经由表单保存会丢掉它们（见 README「与 .NET 版的差异」）。
        let mut form = PostForm::empty(date_at(2026, 7, 28, 9, 30, 8));
        form.state.title = "Doc".to_owned();

        assert!(form.to_front_matter().unwrap().unknown_fields.is_empty());

        form.update_preview();
        assert!(!form.front_matter_preview.contains("sidebar"));
    }
}
