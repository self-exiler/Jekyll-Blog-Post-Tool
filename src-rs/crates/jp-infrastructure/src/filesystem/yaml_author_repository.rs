//! `authors.yml` 仓储：根映射的键为作者 id，值为 `{name, twitter?, url?}`。
//! 路径在每次调用时经 `path_resolver` 解析，支持运行期切换项目（ADR-005）。

use std::path::PathBuf;
use std::sync::Arc;

use jp_domain::authors::{Author, AuthorRepository};
use jp_domain::common::error::{DomainError, DomainResult};
use jp_domain::posts::UnknownValue;
use jp_domain::projects::BlogProject;
use serde_yaml::Value;

use crate::yaml::parser::{mapping_of, text_of};
use crate::yaml::write_map;

pub type PathResolver = Arc<dyn Fn() -> Option<PathBuf> + Send + Sync>;

pub struct YamlAuthorRepository {
    path_resolver: PathResolver,
}

impl YamlAuthorRepository {
    pub fn new(path_resolver: PathResolver) -> Self {
        Self { path_resolver }
    }

    /// 便捷构造：作者文件固定跟随某个项目。
    pub fn for_project(project: &BlogProject) -> Self {
        let path = project.authors_file_path();
        Self::new(Arc::new(move || Some(path.clone())))
    }

    fn resolved_path(&self) -> DomainResult<PathBuf> {
        (self.path_resolver)().ok_or_else(|| {
            DomainError::validation("未选择博客项目，无法保存作者信息。")
        })
    }
}

impl AuthorRepository for YamlAuthorRepository {
    fn get_all(&self) -> DomainResult<Vec<Author>> {
        let Some(path) = (self.path_resolver)() else {
            return Ok(Vec::new());
        };
        if !path.is_file() {
            return Ok(Vec::new());
        }

        let text = std::fs::read_to_string(&path)
            .map_err(|error| DomainError::io(path.display().to_string(), &error))?;
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        let root: Value = serde_yaml::from_str(&text)
            .map_err(|error| DomainError::validation(format!("authors.yml 解析失败：{error}")))?;

        // 仅含注释等无根节点的文件视作空列表（与 .NET 反序列化出 null 的分支一致）
        let Some(root) = mapping_of(&root) else {
            return Ok(Vec::new());
        };

        let mut authors = Vec::new();
        for (key, value) in root.iter() {
            let (Some(id), Some(fields)) = (text_of(key), mapping_of(value)) else {
                continue;
            };
            // 缺少 name 的条目不是有效作者，跳过而非报错（.NET 侧 `Name: not null` 过滤）
            let Some(name) = scalar_text(&fields, "name") else {
                continue;
            };

            authors.push(Author::new(
                &id,
                &name,
                scalar_text(&fields, "twitter").as_deref(),
                scalar_text(&fields, "url").as_deref(),
            )?);
        }

        Ok(authors)
    }

    fn save_all(&self, authors: &[Author]) -> DomainResult<()> {
        let path = self.resolved_path()?;
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory)
                .map_err(|error| DomainError::io(directory.display().to_string(), &error))?;
        }

        let entries = authors
            .iter()
            .map(|author| {
                let mut fields = vec![("name".to_owned(), UnknownValue::scalar(author.name()))];
                // 空字段整个省略（.NET 侧 DefaultValuesHandling.OmitNull）
                for (key, value) in [("twitter", author.twitter()), ("url", author.url())] {
                    if let Some(value) = value {
                        fields.push((key.to_owned(), UnknownValue::scalar(value)));
                    }
                }
                (author.id().to_owned(), UnknownValue::Map(fields))
            })
            .collect::<Vec<_>>();

        let mut yaml = String::new();
        write_map(&mut yaml, &entries, 0);

        // UTF-8 无 BOM（ADR-010）
        std::fs::write(&path, yaml)
            .map_err(|error| DomainError::io(path.display().to_string(), &error))
    }
}

fn scalar_text(fields: &serde_yaml::Mapping, key: &str) -> Option<String> {
    let key = Value::from(key);
    fields
        .get(&key)
        .and_then(text_of)
        .filter(|text| !text.trim().is_empty())
        .map(|text| text.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("jp-yaml-authors-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn repo_for(dir: &Path) -> YamlAuthorRepository {
        let path = dir.join("_data").join("authors.yml");
        YamlAuthorRepository::new(Arc::new(move || Some(path.clone())))
    }

    #[test]
    fn get_all_parses_mapping_of_authors() {
        let dir = temp_dir("parse");
        let file = dir.join("_data").join("authors.yml");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(
            &file,
            "cotes:\n  name: Cotes\n  twitter: cotes201\nadmin:\n  name: Admin\nghost:\n  twitter: x\n",
        )
        .unwrap();

        let authors = repo_for(&dir).get_all().unwrap();

        assert_eq!(authors.len(), 2, "缺少 name 的条目应被跳过");
        assert_eq!(authors[0].id(), "cotes");
        assert_eq!(authors[0].twitter(), Some("cotes201"));
        assert_eq!(authors[1].id(), "admin");
        assert_eq!(authors[1].twitter(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_project_or_file_yields_empty_list() {
        let empty_path: PathResolver = Arc::new(|| None);
        assert!(YamlAuthorRepository::new(empty_path).get_all().unwrap().is_empty());

        let dir = temp_dir("no-file");
        assert!(repo_for(&dir).get_all().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn comment_only_file_yields_empty_list() {
        let dir = temp_dir("comments");
        let file = dir.join("_data").join("authors.yml");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "# 还没有作者\n").unwrap();

        assert!(repo_for(&dir).get_all().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_all_writes_id_keyed_mapping_and_omits_blank_fields() {
        let dir = temp_dir("save");
        let authors = vec![
            Author::new("cotes", "Cotes", Some("cotes201"), None).unwrap(),
            Author::new("admin", "Admin", None, Some("https://a.test")).unwrap(),
        ];

        repo_for(&dir).save_all(&authors).unwrap();

        let written = std::fs::read_to_string(dir.join("_data").join("authors.yml")).unwrap();
        assert_eq!(written, "cotes:\n  name: Cotes\n  twitter: cotes201\nadmin:\n  name: Admin\n  url: https://a.test\n");
        assert!(!written.starts_with('\u{feff}'), "must be BOM-free");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_all_without_project_reports_validation_error() {
        let repo = YamlAuthorRepository::new(Arc::new(|| None));

        let error = repo.save_all(&[]).unwrap_err();

        assert!(matches!(error, DomainError::Validation(_)), "{error}");
        assert!(error.to_string().contains("未选择博客项目"));
    }

    #[test]
    fn round_trip_preserves_order_and_fields() {
        let dir = temp_dir("roundtrip");
        let repo = repo_for(&dir);
        let authors = vec![
            Author::new("zeta", "Zeta", None, None).unwrap(),
            Author::new("alpha", "Alpha", Some("@alpha"), None).unwrap(),
        ];

        repo.save_all(&authors).unwrap();
        let loaded = repo.get_all().unwrap();

        assert_eq!(loaded, authors);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn for_project_uses_authors_file_of_project() {
        let dir = temp_dir("project");
        let repo = YamlAuthorRepository::for_project(&jp_domain::projects::BlogProject::new(&dir));

        repo.save_all(&[Author::new("cotes", "Cotes", None, None).unwrap()]).unwrap();

        assert!(dir.join("_data").join("authors.yml").is_file());
        assert!(repo.get_all().unwrap()[0].name() == "Cotes");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
