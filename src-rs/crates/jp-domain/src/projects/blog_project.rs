use std::path::{Path, PathBuf};

/// 博客项目聚合根：无状态的文件夹引用，不携带项目级配置（ADR-005）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlogProject {
    path: PathBuf,
}

impl BlogProject {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn posts_directory(&self) -> PathBuf {
        self.path.join("_posts")
    }

    pub fn authors_file_path(&self) -> PathBuf {
        self.path.join("_data").join("authors.yml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_chirpy_paths() {
        let project = BlogProject::new("C:/blog");

        assert_eq!(project.posts_directory(), PathBuf::from("C:/blog/_posts"));
        assert_eq!(
            project.authors_file_path(),
            PathBuf::from("C:/blog/_data/authors.yml")
        );
    }
}
