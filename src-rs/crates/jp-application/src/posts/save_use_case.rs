use std::path::Path;
use std::sync::Arc;

use jp_domain::authors::AuthorRepository;
use jp_domain::common::paths;
use jp_domain::common::validation::ValidationError;
use jp_domain::posts::{compute_hash, generate_slug, try_split, FrontMatter, Post, PostRepository};
use jp_domain::projects::BlogProject;

use crate::error::{ApplicationError, ApplicationResult};
use crate::posts::conflict_resolver::{ConflictResolutionKind, ConflictResult, FilenameConflictResolver};
use crate::posts::operation_result::PostOperationResult;
use crate::posts::validator;

/// 保存过程的用户提问 seam：生产实现挂 WinUI 对话框，测试注入 fake 穷举冲突序列。
///
/// 同步签名：实现方把提问投递到 UI 线程并阻塞当前工作线程直至得到答案。
pub trait SavePrompts {
    /// 文件名冲突时询问处理方式；返回 `None` 表示放弃本次保存。
    fn resolve_conflict(&self, conflict: &ConflictResult) -> Option<ConflictResolutionKind>;

    /// 选择覆盖现有文件后的二次确认。
    fn confirm_overwrite(&self, file_name: &str) -> bool;
}

/// 单次读盘的打开结果：展示内容与基线哈希同源。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedPost {
    pub post: Post,
    pub content_hash: String,
}

/// 博文保存用例：新建（`original_file_path` 为 `None`）与更新统一编排——
/// 校验 → slug → 文件名 → 冲突重试循环 → 外部修改检测 → 改名后删除旧文件。
pub struct PostSaveUseCase {
    post_repository: Arc<dyn PostRepository>,
    author_repository: Arc<dyn AuthorRepository>,
    conflict_resolver: FilenameConflictResolver,
}

impl PostSaveUseCase {
    pub fn new(
        post_repository: Arc<dyn PostRepository>,
        author_repository: Arc<dyn AuthorRepository>,
        conflict_resolver: FilenameConflictResolver,
    ) -> Self {
        Self {
            post_repository,
            author_repository,
            conflict_resolver,
        }
    }

    pub fn load(&self, file_path: &Path) -> ApplicationResult<Option<LoadedPost>> {
        Ok(self
            .post_repository
            .read(file_path)?
            .map(|read| LoadedPost {
                content_hash: compute_hash(&read.content),
                post: read.to_post(),
            }))
    }

    /// 读取博文当前内容哈希（保存后刷新基线用）。
    pub fn content_hash(&self, file_path: &Path) -> ApplicationResult<Option<String>> {
        Ok(self
            .post_repository
            .read(file_path)?
            .map(|read| compute_hash(&read.content)))
    }

    /// 保存博文。文件名冲突时经 `prompts` 询问并循环重试；
    /// 外部修改检测返回 `PostOperationResult::ModifiedExternally`，由调用方确认后置空基线哈希重存。
    ///
    /// `body`：`Some` 表示调用方（表单/正文页）对正文有主张，新建与更新都写这份正文；
    /// `None` 表示不碰正文——新建落空正文，更新保留磁盘上的最新 body（FR-3.10 的「未编辑则不被覆盖」）。
    pub fn save(
        &self,
        project: &BlogProject,
        front_matter: &FrontMatter,
        body: Option<&str>,
        prompts: &dyn SavePrompts,
        original_file_path: Option<&Path>,
        original_content_hash: Option<&str>,
    ) -> ApplicationResult<PostOperationResult> {
        let mut resolution = None;

        loop {
            let result = match original_file_path {
                None => self.create_core(project, front_matter, body, resolution)?,
                Some(path) => self.update_core(
                    project,
                    path,
                    front_matter,
                    body,
                    original_content_hash,
                    resolution,
                )?,
            };

            let conflict = match &result {
                PostOperationResult::Conflict { conflict } => conflict.clone(),
                _ => return Ok(result),
            };

            let Some(kind) = prompts.resolve_conflict(&conflict) else {
                // 用户取消冲突处理
                return Ok(result);
            };

            if kind == ConflictResolutionKind::Overwrite
                && !prompts.confirm_overwrite(&conflict.file_name())
            {
                return Ok(result);
            }

            resolution = Some(kind);
        }
    }

    fn create_core(
        &self,
        project: &BlogProject,
        front_matter: &FrontMatter,
        body: Option<&str>,
        resolution: Option<ConflictResolutionKind>,
    ) -> ApplicationResult<PostOperationResult> {
        let errors = self.validate(front_matter)?;
        if !errors.is_empty() {
            return Ok(PostOperationResult::failure(errors));
        }

        let conflict = self.conflict_resolver.check(project, &self.build_file_name(front_matter)?);
        if conflict.has_conflict() && resolution.is_none() {
            return Ok(PostOperationResult::with_conflict(conflict));
        }

        let file_path = conflict.resolve(resolution)?;
        let post = Post::new(&file_path, front_matter.clone(), body.unwrap_or_default())?;
        self.post_repository.save(&post)?;

        Ok(PostOperationResult::success(file_path))
    }

    #[allow(clippy::too_many_arguments)]
    fn update_core(
        &self,
        project: &BlogProject,
        original_file_path: &Path,
        front_matter: &FrontMatter,
        body: Option<&str>,
        original_content_hash: Option<&str>,
        resolution: Option<ConflictResolutionKind>,
    ) -> ApplicationResult<PostOperationResult> {
        // 单次读盘：存在检查 + 外部修改检测 + body 提取共用同一份内容
        let Some(content) = self.post_repository.read_all_text(original_file_path)? else {
            return Ok(PostOperationResult::failure(vec![ValidationError::new(
                "",
                "博文文件不存在",
            )]));
        };

        if let Some(expected) = original_content_hash {
            if compute_hash(&content) != expected {
                return Ok(PostOperationResult::modified_externally());
            }
        }

        let errors = self.validate(front_matter)?;
        if !errors.is_empty() {
            return Ok(PostOperationResult::failure(errors));
        }

        let mut new_file_path = project
            .posts_directory()
            .join(self.build_file_name(front_matter)?);

        let renamed = !paths::eq_ignore_ascii_case(
            &new_file_path.to_string_lossy(),
            &original_file_path.to_string_lossy(),
        );

        if renamed {
            let conflict = self
                .conflict_resolver
                .check(project, &paths::file_name(&new_file_path));
            if conflict.has_conflict() && resolution.is_none() {
                return Ok(PostOperationResult::with_conflict(conflict));
            }

            new_file_path = conflict.resolve(resolution)?;
        }

        // FR-3.10：调用方未编辑过正文（`body` 为 `None`）时保留磁盘最新，从已读取的全文切分
        let current_body = match body {
            Some(body) => body.to_owned(),
            None => try_split(&content)
                .map(|(_, body)| body)
                .unwrap_or_else(|| content.clone()),
        };

        let post = Post::new(&new_file_path, front_matter.clone(), current_body)?;
        self.post_repository.save(&post)?;

        if renamed {
            if let Err(error) = self.post_repository.delete(original_file_path) {
                // 新文件已写入但旧文件未能删除：明确警告，避免新旧两份内容分叉而不自知
                return Ok(PostOperationResult::success_with_warnings(
                    new_file_path,
                    vec![format!(
                        "旧文件 {} 删除失败（{error}），磁盘上可能残留旧博文",
                        paths::file_name(original_file_path)
                    )],
                ));
            }
        }

        Ok(PostOperationResult::success(new_file_path))
    }

    fn validate(&self, front_matter: &FrontMatter) -> ApplicationResult<Vec<ValidationError>> {
        Ok(validator::validate(front_matter, &self.author_repository.get_all()?))
    }

    fn build_file_name(&self, front_matter: &FrontMatter) -> ApplicationResult<String> {
        let slug = generate_slug(&front_matter.title)?;
        let date = front_matter
            .date
            .as_ref()
            .ok_or_else(|| ApplicationError::validation("date 为必填项"))?;

        Ok(Post::build_file_name(date, &slug))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, NaiveDate};
    use jp_domain::authors::Author;
    use jp_domain::posts::PostRead;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};

    fn project() -> &'static BlogProject {
        static PROJECT: OnceLock<BlogProject> = OnceLock::new();
        PROJECT.get_or_init(|| BlogProject::new(std::env::temp_dir()))
    }

    /// 记录 save 写入内容的 stub，模拟磁盘状态。
    struct StubPostRepository {
        files: Mutex<HashMap<String, String>>,
        lock_deletes: Mutex<bool>,
    }

    impl StubPostRepository {
        fn new() -> Self {
            Self {
                files: Mutex::new(HashMap::new()),
                lock_deletes: Mutex::new(false),
            }
        }

        fn add_existing_file(&self, path: &Path, content: &str) {
            self.files
                .lock()
                .unwrap()
                .insert(key(path), content.to_owned());
        }

        fn read_text(&self, path: &Path) -> Option<String> {
            self.files.lock().unwrap().get(&key(path)).cloned()
        }

        fn set_lock_deletes(&self, value: bool) {
            *self.lock_deletes.lock().unwrap() = value;
        }
    }

    impl PostRepository for StubPostRepository {
        fn exists(&self, file_path: &Path) -> bool {
            self.files.lock().unwrap().contains_key(&key(file_path))
        }

        fn delete(&self, file_path: &Path) -> Result<(), jp_domain::common::error::DomainError> {
            if *self.lock_deletes.lock().unwrap() {
                return Err(jp_domain::common::error::DomainError::Io {
                    path: file_path.display().to_string(),
                    message: "文件被其他程序占用".to_owned(),
                });
            }
            self.files.lock().unwrap().remove(&key(file_path));
            Ok(())
        }

        fn read(&self, file_path: &Path) -> Result<Option<PostRead>, jp_domain::common::error::DomainError> {
            let Some(content) = self.read_text(file_path) else {
                return Ok(None);
            };
            let (yaml, body) = try_split(&content).unwrap_or((String::new(), content.clone()));
            let mut front_matter = FrontMatter::new();
            for line in yaml.lines() {
                if let Some(value) = line.split_once(':') {
                    if value.0.trim() == "title" {
                        front_matter.title = value.1.trim().to_owned();
                    }
                }
            }
            Ok(Some(PostRead {
                file_path: file_path.to_path_buf(),
                content,
                front_matter,
                body,
            }))
        }

        fn save(&self, post: &Post) -> Result<(), jp_domain::common::error::DomainError> {
            self.files.lock().unwrap().insert(
                key(post.file_path()),
                format!("---\ntitle: {}\n---\n{}", post.front_matter().title, post.body()),
            );
            Ok(())
        }

        fn read_all_text(&self, file_path: &Path) -> Result<Option<String>, jp_domain::common::error::DomainError> {
            Ok(self.read_text(file_path))
        }
    }

    struct StubAuthorRepository(Vec<Author>);

    impl AuthorRepository for StubAuthorRepository {
        fn get_all(&self) -> Result<Vec<Author>, jp_domain::common::error::DomainError> {
            Ok(self.0.clone())
        }

        fn save_all(&self, _: &[Author]) -> Result<(), jp_domain::common::error::DomainError> {
            Ok(())
        }
    }

    struct FakePrompts {
        conflict_answer: Option<ConflictResolutionKind>,
        confirm_overwrite: bool,
        asked: Mutex<Vec<ConflictResult>>,
    }

    impl FakePrompts {
        fn new(conflict_answer: Option<ConflictResolutionKind>) -> Self {
            Self {
                conflict_answer,
                confirm_overwrite: true,
                asked: Mutex::new(Vec::new()),
            }
        }

        fn asked_count(&self) -> usize {
            self.asked.lock().unwrap().len()
        }
    }

    impl SavePrompts for FakePrompts {
        fn resolve_conflict(&self, conflict: &ConflictResult) -> Option<ConflictResolutionKind> {
            self.asked.lock().unwrap().push(conflict.clone());
            self.conflict_answer
        }

        fn confirm_overwrite(&self, _file_name: &str) -> bool {
            self.confirm_overwrite
        }
    }

    fn key(path: &Path) -> String {
        path.to_string_lossy().to_ascii_lowercase()
    }

    fn valid_front_matter() -> FrontMatter {
        FrontMatter {
            title: "Hello World".to_owned(),
            date: Some(date(2026, 7, 28)),
            authors: vec!["cotes".to_owned()],
            ..FrontMatter::new()
        }
    }

    fn use_case(post_repo: Arc<StubPostRepository>) -> (PostSaveUseCase, Arc<StubPostRepository>) {
        let repository: Arc<dyn PostRepository> = post_repo.clone();
        let authors: Arc<dyn AuthorRepository> =
            Arc::new(StubAuthorRepository(vec![Author::new("cotes", "Cotes", None, None).unwrap()]));
        let use_case = PostSaveUseCase::new(
            repository,
            authors,
            FilenameConflictResolver::new(post_repo.clone() as Arc<dyn PostRepository>),
        );
        (use_case, post_repo)
    }

    fn posts_path(file_name: &str) -> PathBuf {
        project().posts_directory().join(file_name)
    }

    #[test]
    fn create_with_valid_input_saves_and_returns_path() {
        let (use_case, repo) = use_case(Arc::new(StubPostRepository::new()));

        let result = use_case
            .save(project(), &valid_front_matter(), Some("body"), &FakePrompts::new(None), None, None)
            .unwrap();

        assert!(result.is_success());
        assert!(result.file_path().unwrap().ends_with("2026-07-28-hello-world.md"));
        assert!(repo.exists(&posts_path("2026-07-28-hello-world.md")));
    }

    #[test]
    fn create_with_blank_title_returns_validation_failure() {
        let (use_case, _) = use_case(Arc::new(StubPostRepository::new()));
        let mut front_matter = valid_front_matter();
        front_matter.title = String::new();

        let result = use_case
            .save(project(), &front_matter, None, &FakePrompts::new(None), None, None)
            .unwrap();

        assert!(!result.is_success());
        assert!(matches!(result, PostOperationResult::ValidationFailed { .. }));
        assert!(!result.errors().is_empty());
    }

    #[test]
    fn create_conflict_without_answer_returns_conflict_with_suffix() {
        let repo = Arc::new(StubPostRepository::new());
        repo.add_existing_file(&posts_path("2026-07-28-hello-world.md"), "old");
        let (use_case, _) = use_case(repo);
        let prompts = FakePrompts::new(None);

        let result = use_case
            .save(project(), &valid_front_matter(), None, &prompts, None, None)
            .unwrap();

        assert!(result.is_conflict());
        assert_eq!(prompts.asked_count(), 1);
        assert_eq!(result.conflict().unwrap().auto_suffix(), Some(1));
    }

    #[test]
    fn create_conflict_cancel_stops_loop() {
        let repo = Arc::new(StubPostRepository::new());
        repo.add_existing_file(&posts_path("2026-07-28-hello-world.md"), "old");
        let (use_case, _) = use_case(Arc::clone(&repo));
        let prompts = FakePrompts::new(None);

        let result = use_case
            .save(project(), &valid_front_matter(), None, &prompts, None, None)
            .unwrap();

        assert!(result.is_conflict());
        assert_eq!(prompts.asked_count(), 1);
        assert!(!repo.exists(&posts_path("2026-07-28-hello-world-1.md")));
    }

    #[test]
    fn create_conflict_overwrite_not_confirmed_stops_loop() {
        let repo = Arc::new(StubPostRepository::new());
        let existing = posts_path("2026-07-28-hello-world.md");
        repo.add_existing_file(&existing, "external-content");
        let (use_case, _) = use_case(Arc::clone(&repo));
        let mut prompts = FakePrompts::new(Some(ConflictResolutionKind::Overwrite));
        prompts.confirm_overwrite = false;

        let result = use_case
            .save(project(), &valid_front_matter(), None, &prompts, None, None)
            .unwrap();

        assert!(result.is_conflict());
        assert_eq!(repo.read_text(&existing).unwrap(), "external-content");
    }

    #[test]
    fn create_conflict_overwrite_confirmed_saves_original_path() {
        let repo = Arc::new(StubPostRepository::new());
        repo.add_existing_file(&posts_path("2026-07-28-hello-world.md"), "old");
        let (use_case, _) = use_case(repo);

        let result = use_case
            .save(
                project(),
                &valid_front_matter(),
                None,
                &FakePrompts::new(Some(ConflictResolutionKind::Overwrite)),
                None,
                None,
            )
            .unwrap();

        assert!(result.is_success());
        assert!(result.file_path().unwrap().ends_with("2026-07-28-hello-world.md"));
    }

    #[test]
    fn create_without_conflict_never_asks() {
        let (use_case, _) = use_case(Arc::new(StubPostRepository::new()));
        let prompts = FakePrompts::new(Some(ConflictResolutionKind::AutoSuffix));

        let result = use_case
            .save(project(), &valid_front_matter(), None, &prompts, None, None)
            .unwrap();

        assert!(result.is_success());
        assert_eq!(prompts.asked_count(), 0);
    }

    #[test]
    fn update_with_matching_hash_saves_in_place_and_keeps_disk_body() {
        let repo = Arc::new(StubPostRepository::new());
        let original = posts_path("2026-01-01-old-title.md");
        repo.add_existing_file(&original, "---\ntitle: Old Title\n---\noriginal body");
        let (use_case, _) = use_case(Arc::clone(&repo));
        let hash = use_case.content_hash(&original).unwrap().unwrap();

        let mut front_matter = valid_front_matter();
        front_matter.title = "Old Title".to_owned();
        front_matter.date = Some(date(2026, 1, 1));

        let result = use_case
            .save(project(), &front_matter, None, &FakePrompts::new(None), Some(&original), Some(&hash))
            .unwrap();

        assert!(result.is_success());
        assert_eq!(result.file_path().unwrap(), &original);
        assert!(repo.read_text(&original).unwrap().contains("original body"));
    }

    #[test]
    fn update_with_body_from_form_overwrites_the_disk_body() {
        let repo = Arc::new(StubPostRepository::new());
        let original = posts_path("2026-01-01-old-title.md");
        repo.add_existing_file(&original, "---\ntitle: Old Title\n---\noriginal body");
        let (use_case, _) = use_case(Arc::clone(&repo));
        let hash = use_case.content_hash(&original).unwrap().unwrap();

        let mut front_matter = valid_front_matter();
        front_matter.title = "Old Title".to_owned();
        front_matter.date = Some(date(2026, 1, 1));

        let result = use_case
            .save(
                project(),
                &front_matter,
                Some("edited body"),
                &FakePrompts::new(None),
                Some(&original),
                Some(&hash),
            )
            .unwrap();

        assert!(result.is_success());
        let content = repo.read_text(&original).unwrap();
        assert!(content.ends_with("edited body"), "实际内容: {content}");
        assert!(!content.contains("original body"));
    }

    #[test]
    fn update_detects_external_modification() {
        let repo = Arc::new(StubPostRepository::new());
        let original = posts_path("2026-07-28-hello-world.md");
        repo.add_existing_file(&original, "---\ntitle: External Edit\n---\nchanged");
        let (use_case, _) = use_case(repo);

        let result = use_case
            .save(
                project(),
                &valid_front_matter(),
                None,
                &FakePrompts::new(None),
                Some(&original),
                Some("deadbeef"),
            )
            .unwrap();

        assert!(result.is_modified_externally());
        assert!(!result.is_success());
    }

    #[test]
    fn update_missing_file_returns_validation_failure() {
        let (use_case, _) = use_case(Arc::new(StubPostRepository::new()));

        let result = use_case
            .save(
                project(),
                &valid_front_matter(),
                None,
                &FakePrompts::new(None),
                Some(&posts_path("ghost.md")),
                None,
            )
            .unwrap();

        assert!(!result.is_success());
        assert!(result.errors().iter().any(|e| e.message.contains("不存在")));
    }

    #[test]
    fn update_rename_removes_old_file_and_keeps_body() {
        let repo = Arc::new(StubPostRepository::new());
        let old = posts_path("2026-01-01-old-title.md");
        repo.add_existing_file(&old, "---\ntitle: Old Title\n---\nkeep body");
        let (use_case, _) = use_case(Arc::clone(&repo));
        let hash = use_case.content_hash(&old).unwrap().unwrap();

        let result = use_case
            .save(project(), &valid_front_matter(), None, &FakePrompts::new(None), Some(&old), Some(&hash))
            .unwrap();

        assert!(result.is_success());
        assert!(result.file_path().unwrap().ends_with("2026-07-28-hello-world.md"));
        assert!(!repo.exists(&old));
        assert!(repo.read_text(result.file_path().unwrap()).unwrap().contains("keep body"));
    }

    #[test]
    fn update_rename_delete_failure_warns_but_succeeds() {
        let repo = Arc::new(StubPostRepository::new());
        let old = posts_path("2026-01-01-old-title.md");
        repo.add_existing_file(&old, "---\ntitle: Old Title\n---\nkeep body");
        repo.set_lock_deletes(true);
        let (use_case, _) = use_case(Arc::clone(&repo));
        let hash = use_case.content_hash(&old).unwrap().unwrap();

        let result = use_case
            .save(project(), &valid_front_matter(), None, &FakePrompts::new(None), Some(&old), Some(&hash))
            .unwrap();

        assert!(result.is_success());
        assert!(result.warnings().iter().any(|w| w.contains("删除失败")));
        assert!(repo.exists(&old));
    }

    #[test]
    fn update_rename_conflict_resolves_with_auto_suffix() {
        let repo = Arc::new(StubPostRepository::new());
        let old = posts_path("2026-01-01-old-title.md");
        repo.add_existing_file(&old, "---\ntitle: Old Title\n---\nbody");
        repo.add_existing_file(&posts_path("2026-07-28-hello-world.md"), "existing target");
        let (use_case, _) = use_case(Arc::clone(&repo));
        let hash = use_case.content_hash(&old).unwrap().unwrap();

        let result = use_case
            .save(
                project(),
                &valid_front_matter(),
                None,
                &FakePrompts::new(Some(ConflictResolutionKind::AutoSuffix)),
                Some(&old),
                Some(&hash),
            )
            .unwrap();

        assert!(result.is_success());
        assert!(result.file_path().unwrap().ends_with("2026-07-28-hello-world-1.md"));
        assert!(!repo.exists(&old));
    }

    fn date(year: i32, month: u32, day: u32) -> jp_domain::posts::PostDate {
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(FixedOffset::east_opt(0).unwrap())
            .single()
            .unwrap()
    }
}
