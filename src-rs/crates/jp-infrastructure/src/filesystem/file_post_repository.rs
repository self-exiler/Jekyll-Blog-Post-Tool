//! 基于文件系统的博文仓储：纯文件 IO，格式细节全部委托 `post_file_format`。

use std::path::{Path, PathBuf};

use jp_domain::common::error::{DomainError, DomainResult};
use jp_domain::posts::{Post, PostRead, PostRepository};

use crate::posts::post_file_format;

#[derive(Debug, Default, Clone, Copy)]
pub struct FilePostRepository;

impl FilePostRepository {
    pub fn new() -> Self {
        Self
    }
}

impl PostRepository for FilePostRepository {
    fn exists(&self, file_path: &Path) -> bool {
        file_path.is_file()
    }

    fn delete(&self, file_path: &Path) -> DomainResult<()> {
        if !file_path.exists() {
            return Ok(());
        }

        std::fs::remove_file(file_path).map_err(|error| DomainError::io(display_path(file_path), &error))
    }

    fn read(&self, file_path: &Path) -> DomainResult<Option<PostRead>> {
        let Some(content) = self.read_all_text(file_path)? else {
            return Ok(None);
        };

        let (front_matter, body) = post_file_format::parse(&content)?;
        Ok(Some(PostRead {
            file_path: file_path.to_path_buf(),
            content,
            front_matter,
            body,
        }))
    }

    fn save(&self, post: &Post) -> DomainResult<()> {
        let file_path = post.file_path();
        if let Some(directory) = file_path.parent() {
            std::fs::create_dir_all(directory)
                .map_err(|error| DomainError::io(display_path(directory), &error))?;
        }

        let content = post_file_format::format(post.front_matter(), Some(post.body()));

        // 先写临时文件再原子替换：写一半崩溃不会损坏已有博文
        let temp_path = temp_path_of(file_path);
        match write_and_replace(&temp_path, file_path, &content) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = std::fs::remove_file(&temp_path);
                Err(error)
            }
        }
    }

    fn read_all_text(&self, file_path: &Path) -> DomainResult<Option<String>> {
        if !file_path.is_file() {
            return Ok(None);
        }

        let bytes = std::fs::read(file_path).map_err(|error| DomainError::io(display_path(file_path), &error))?;
        Ok(Some(decode_utf8(&bytes)))
    }
}

/// 与 .NET `File.ReadAllText(path, Encoding.UTF8)` 对齐：剥除 BOM，非法字节按替换字符处理，
/// 不因单个坏字节让整篇博文读失败。
fn decode_utf8(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    String::from_utf8_lossy(bytes).into_owned()
}

fn write_and_replace(temp_path: &Path, file_path: &Path, content: &str) -> DomainResult<()> {
    std::fs::write(temp_path, content.as_bytes())
        .map_err(|error| DomainError::io(display_path(temp_path), &error))?;

    // Windows 下 rename 目标已存在时由 std 附加 MOVEFILE_REPLACE_EXISTING，语义等同 File.Move(overwrite: true)
    std::fs::rename(temp_path, file_path)
        .map_err(|error| DomainError::io(display_path(file_path), &error))
}

fn temp_path_of(file_path: &Path) -> PathBuf {
    let mut name = file_path.as_os_str().to_os_string();
    name.push(".tmp");
    PathBuf::from(name)
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, NaiveDate};
    use jp_domain::posts::{Category, FrontMatter};

    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "jp-file-post-{}-{tag}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn post_at(file_path: &Path) -> Post {
        Post::new(
            file_path,
            FrontMatter {
                title: "Hello World".to_owned(),
                date: Some(
                    NaiveDate::from_ymd_opt(2026, 7, 28)
                        .unwrap()
                        .and_hms_opt(14, 10, 0)
                        .unwrap()
                        .and_local_timezone(FixedOffset::east_opt(8 * 3600).unwrap())
                        .single()
                        .unwrap(),
                ),
                categories: vec![Category::new("Blogging").unwrap()],
                ..FrontMatter::new()
            },
            "正文第一行\n第二行",
        )
        .unwrap()
    }

    #[test]
    fn save_creates_missing_directories_and_read_roundtrips() {
        let repo = FilePostRepository::new();
        let dir = temp_dir("roundtrip");
        let file_path = dir.join("_posts").join("2026-07-28-hello-world.md");

        repo.save(&post_at(&file_path)).unwrap();

        assert!(file_path.is_file());
        assert!(repo.exists(&file_path));
        assert!(!dir.join("_posts").join("2026-07-28-hello-world.md.tmp").exists());

        let read = repo.read(&file_path).unwrap().unwrap();
        assert_eq!(read.front_matter.title, "Hello World");
        assert_eq!(read.front_matter.categories[0].value(), "Blogging");
        assert_eq!(read.body, "正文第一行\n第二行");
        assert_eq!(read.content, post_file_format::format(&read.front_matter, Some(read.body.as_str())));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_overwrites_existing_file() {
        let repo = FilePostRepository::new();
        let dir = temp_dir("overwrite");
        let file_path = dir.join("a.md");

        let mut fm = post_at(&file_path).front_matter().clone();
        fm.title = "First".to_owned();
        repo.save(&Post::new(&file_path, fm.clone(), "one").unwrap()).unwrap();

        fm.title = "Second".to_owned();
        repo.save(&Post::new(&file_path, fm, "two").unwrap()).unwrap();

        assert_eq!(
            repo.read_all_text(&file_path).unwrap().unwrap(),
            "---\ntitle: Second\ndate: 2026-07-28 14:10:00 +08:00\ncategories:\n- Blogging\n---\ntwo"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_missing_file_yields_none() {
        let repo = FilePostRepository::new();
        let dir = temp_dir("missing");

        assert!(repo.read(&dir.join("ghost.md")).unwrap().is_none());
        assert!(repo.read_all_text(&dir.join("ghost.md")).unwrap().is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_is_idempotent() {
        let repo = FilePostRepository::new();
        let dir = temp_dir("delete");
        let file_path = dir.join("a.md");
        repo.save(&post_at(&file_path)).unwrap();

        repo.delete(&file_path).unwrap();
        repo.delete(&file_path).unwrap();

        assert!(!file_path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn utf8_bom_is_stripped_before_parsing() {
        let dir = temp_dir("bom");
        let file_path = dir.join("bom.md");
        std::fs::write(&file_path, [&[0xEF, 0xBB, 0xBF], "---\ntitle: BOM\n---\nbody".as_bytes()].concat())
            .unwrap();

        let read = FilePostRepository::new().read(&file_path).unwrap().unwrap();

        assert_eq!(read.front_matter.title, "BOM");
        assert_eq!(read.body, "body");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
