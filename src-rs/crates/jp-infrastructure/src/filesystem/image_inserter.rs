//! 将本地图片复制到博文对应的资源目录，并生成 markdown 引用插入正文（内存，保存时统一写入）。
//! 依据：FR-6.2~6.6、设计方案 §4.4.1。

use std::path::{Path, PathBuf};

use jp_domain::common::error::{DomainError, DomainResult};
use jp_domain::common::paths;
use jp_domain::projects::BlogProject;

/// 与 .NET `StringBuilder.AppendLine` 在 Windows 上一致的行尾。
const NEWLINE: &str = "\r\n";

/// 把图片复制到 `{project}/assets/img/{post_slug}/`，返回 markdown 引用文本（换行分隔）；
/// 无成功复制时为空字符串。
pub fn insert(
    project: &BlogProject,
    post_slug: &str,
    source_image_paths: &[PathBuf],
    alt: &str,
) -> DomainResult<String> {
    if post_slug.trim().is_empty() {
        return Err(DomainError::validation_blank("postSlug"));
    }

    let target_dir = project.path().join("assets").join("img").join(post_slug);
    std::fs::create_dir_all(&target_dir)
        .map_err(|error| DomainError::io(target_dir.display().to_string(), &error))?;

    let mut markdown = String::new();
    for source_path in source_image_paths {
        if !source_path.is_file() {
            continue;
        }

        let file_name = paths::file_name(source_path);
        // FR-6.4：文件名冲突自动追加 -1, -2, ... 直到可用
        let dest_path = resolve_available(&target_dir.join(&file_name));

        std::fs::copy(source_path, &dest_path)
            .map_err(|error| DomainError::io(source_path.display().to_string(), &error))?;

        let dest_file_name = paths::file_name(&dest_path);
        markdown.push_str(&format!("![{}](/assets/img/{post_slug}/{dest_file_name}){NEWLINE}", escape_alt(alt)));
    }

    Ok(markdown)
}

fn resolve_available(dest_path: &Path) -> PathBuf {
    if !dest_path.exists() {
        return dest_path.to_path_buf();
    }

    let mut index = 1;
    loop {
        let candidate = paths::append_suffix(dest_path, index);
        if !candidate.exists() {
            return candidate;
        }
        index += 1;
    }
}

/// 转义 alt 文本中的 markdown 链接语法字符，避免生成断裂的引用。
fn escape_alt(alt: &str) -> String {
    alt.replace('[', "\\[").replace(']', "\\]").replace('(', "\\(").replace(')', "\\)")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("jp-image-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn image(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn copies_images_and_emits_references() {
        let project_dir = temp_dir("copy");
        let scratch = temp_dir("copy-src");
        let first = image(&scratch, "a.png", b"1");
        let second = image(&scratch, "b.jpg", b"2");

        let markdown = insert(
            &BlogProject::new(&project_dir),
            "hello-world",
            &[first, second],
            "配图",
        )
        .unwrap();

        assert_eq!(markdown, "![配图](/assets/img/hello-world/a.png)\r\n![配图](/assets/img/hello-world/b.jpg)\r\n");
        assert_eq!(std::fs::read(project_dir.join("assets/img/hello-world/a.png")).unwrap(), b"1");
        assert!(project_dir.join("assets/img/hello-world/b.jpg").is_file());

        let _ = std::fs::remove_dir_all(&project_dir);
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn name_conflict_gets_incrementing_suffix() {
        let project_dir = temp_dir("conflict");
        let scratch = temp_dir("conflict-src");
        let source = image(&scratch, "shot.png", b"x");

        for expected in ["shot.png", "shot-1.png", "shot-2.png"] {
            let markdown = insert(&BlogProject::new(&project_dir), "s", &[source.clone()], "").unwrap();

            assert!(markdown.contains(&format!("](/assets/img/s/{expected})")), "{markdown}");
        }

        assert!(project_dir.join("assets/img/s/shot-2.png").is_file());
        let _ = std::fs::remove_dir_all(&project_dir);
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn missing_sources_are_skipped() {
        let project_dir = temp_dir("missing");

        let markdown = insert(
            &BlogProject::new(&project_dir),
            "s",
            &[project_dir.join("gone.png")],
            "",
        )
        .unwrap();

        assert_eq!(markdown, "");
        let _ = std::fs::remove_dir_all(&project_dir);
    }

    #[test]
    fn alt_syntax_characters_are_escaped() {
        assert_eq!(escape_alt("[a](b)"), "\\[a\\]\\(b\\)");
    }

    #[test]
    fn blank_slug_is_rejected() {
        let error = insert(&BlogProject::new("C:/blog"), "   ", &[], "").unwrap_err();

        assert!(matches!(error, DomainError::Validation(_)));
    }
}
