use std::sync::Arc;

use jp_domain::authors::{Author, AuthorRepository};

use crate::error::{ApplicationError, ApplicationResult};

/// 作者增删改用例（读全量 → 修改 → 回写）。冲突经错误类型上抛，由调用方展示。
pub struct AuthorCrudUseCase {
    repository: Arc<dyn AuthorRepository>,
}

impl AuthorCrudUseCase {
    pub fn new(repository: Arc<dyn AuthorRepository>) -> Self {
        Self { repository }
    }

    pub fn add(&self, author: &Author) -> ApplicationResult<()> {
        let mut authors = self.repository.get_all()?;

        if authors.iter().any(|existing| existing.id() == author.id()) {
            return Err(ApplicationError::AuthorAlreadyExists(author.id().to_owned()));
        }

        authors.push(author.clone());
        self.repository.save_all(&authors).map_err(ApplicationError::from)
    }

    pub fn update(&self, author: &Author) -> ApplicationResult<()> {
        let mut authors = self.repository.get_all()?;

        let index = authors
            .iter()
            .position(|existing| existing.id() == author.id())
            .ok_or_else(|| ApplicationError::AuthorNotFound(author.id().to_owned()))?;

        authors[index] = author.clone();
        self.repository.save_all(&authors).map_err(ApplicationError::from)
    }

    pub fn delete(&self, id: &str) -> ApplicationResult<()> {
        let mut authors = self.repository.get_all()?;

        let index = authors
            .iter()
            .position(|existing| existing.id() == id)
            .ok_or_else(|| ApplicationError::AuthorNotFound(id.to_owned()))?;

        authors.remove(index);
        self.repository.save_all(&authors).map_err(ApplicationError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryAuthorRepository {
        authors: Mutex<Vec<Author>>,
    }

    impl MemoryAuthorRepository {
        fn with(authors: Vec<Author>) -> Arc<Self> {
            Arc::new(Self { authors: Mutex::new(authors) })
        }

        fn snapshot(&self) -> Vec<Author> {
            self.authors.lock().unwrap().clone()
        }
    }

    impl AuthorRepository for MemoryAuthorRepository {
        fn get_all(&self) -> Result<Vec<Author>, jp_domain::common::error::DomainError> {
            Ok(self.authors.lock().unwrap().clone())
        }

        fn save_all(&self, authors: &[Author]) -> Result<(), jp_domain::common::error::DomainError> {
            *self.authors.lock().unwrap() = authors.to_vec();
            Ok(())
        }
    }

    fn author(id: &str, name: &str) -> Author {
        Author::new(id, name, None, None).unwrap()
    }

    #[test]
    fn add_appends_and_persists() {
        let repository = MemoryAuthorRepository::with(vec![author("cotes", "Cotes")]);
        let use_case = AuthorCrudUseCase::new(repository.clone());

        use_case.add(&author("dioha", "dioha")).unwrap();

        assert_eq!(repository.snapshot().len(), 2);
        assert_eq!(repository.snapshot()[1].id(), "dioha");
    }

    #[test]
    fn add_rejects_duplicate_id() {
        let repository = MemoryAuthorRepository::with(vec![author("cotes", "Cotes")]);
        let use_case = AuthorCrudUseCase::new(repository.clone());

        let error = use_case.add(&author("cotes", "Other")).unwrap_err();

        assert_eq!(error, ApplicationError::AuthorAlreadyExists("cotes".to_owned()));
        assert_eq!(error.to_string(), "作者 id 'cotes' 已存在");
        assert_eq!(repository.snapshot().len(), 1);
    }

    #[test]
    fn update_replaces_in_place() {
        let repository = MemoryAuthorRepository::with(vec![
            author("cotes", "Cotes"),
            author("admin", "Admin"),
        ]);
        let use_case = AuthorCrudUseCase::new(repository.clone());

        use_case.update(&author("cotes", "cotes2020")).unwrap();

        let authors = repository.snapshot();
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0].name(), "cotes2020");
    }

    #[test]
    fn update_missing_id_fails() {
        let repository = MemoryAuthorRepository::with(vec![author("cotes", "Cotes")]);
        let use_case = AuthorCrudUseCase::new(repository);

        let error = use_case.update(&author("ghost", "Ghost")).unwrap_err();

        assert!(matches!(error, ApplicationError::AuthorNotFound(_)));
    }

    #[test]
    fn delete_removes_only_matching_id() {
        let repository = MemoryAuthorRepository::with(vec![
            author("cotes", "Cotes"),
            author("admin", "Admin"),
        ]);
        let use_case = AuthorCrudUseCase::new(repository.clone());

        use_case.delete("cotes").unwrap();

        let authors = repository.snapshot();
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].id(), "admin");
    }

    #[test]
    fn delete_missing_id_fails() {
        let repository = MemoryAuthorRepository::with(vec![author("cotes", "Cotes")]);
        let use_case = AuthorCrudUseCase::new(repository);

        assert!(use_case.delete("ghost").is_err());
    }
}
