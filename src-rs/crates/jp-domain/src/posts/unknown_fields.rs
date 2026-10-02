/// 工具不认识的 front matter 字段值。递归形态与 YAML 节点一一对应，
/// 使未知字段能在不引入 YAML 库类型的前提下原样 round-trip。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnknownValue {
    Null,
    Scalar(String),
    Seq(Vec<UnknownValue>),
    Map(Vec<(String, UnknownValue)>),
}

impl UnknownValue {
    pub fn scalar(value: impl Into<String>) -> Self {
        Self::Scalar(value.into())
    }

    pub fn seq(values: impl IntoIterator<Item = Self>) -> Self {
        Self::Seq(values.into_iter().collect())
    }

    pub fn as_scalar(&self) -> Option<&str> {
        match self {
            Self::Scalar(value) => Some(value),
            _ => None,
        }
    }
}

/// 保序的未知字段集合：`Vec<(key, value)>` 保留磁盘上的原始书写顺序。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnknownFields {
    entries: Vec<(String, UnknownValue)>,
}

impl UnknownFields {
    pub fn new() -> Self {
        Self::default()
    }

    /// 同键重复出现时保留首个位置、覆盖取值。
    pub fn insert(&mut self, key: &str, value: UnknownValue) {
        match self.entries.iter_mut().find(|(existing, _)| existing == key) {
            Some(slot) => slot.1 = value,
            None => self.entries.push((key.to_owned(), value)),
        }
    }

    pub fn get(&self, key: &str) -> Option<&UnknownValue> {
        self.entries
            .iter()
            .find(|(existing, _)| existing == key)
            .map(|(_, value)| value)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn keys(&self) -> Vec<&str> {
        self.entries.iter().map(|(key, _)| key.as_str()).collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &UnknownValue)> {
        self.entries.iter().map(|(key, value)| (key, value))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_preserves_written_order() {
        let mut fields = UnknownFields::new();
        fields.insert("image", UnknownValue::scalar("/assets/img/a.png"));
        fields.insert("pin", UnknownValue::scalar("true"));
        fields.insert(
            "math",
            UnknownValue::Map(vec![("enable".to_owned(), UnknownValue::scalar("true"))]),
        );

        assert_eq!(fields.keys(), vec!["image", "pin", "math"]);
        assert!(fields.contains_key("math"));
        assert!(!fields.contains_key("mermaid"));
    }

    #[test]
    fn insert_same_key_keeps_first_position() {
        let mut fields = UnknownFields::new();
        fields.insert("a", UnknownValue::scalar("1"));
        fields.insert("b", UnknownValue::scalar("2"));
        fields.insert("a", UnknownValue::scalar("3"));

        assert_eq!(fields.keys(), vec!["a", "b"]);
        assert_eq!(fields.get("a").and_then(UnknownValue::as_scalar), Some("3"));
    }

    #[test]
    fn seq_roundtrips_items() {
        let value = UnknownValue::seq([UnknownValue::scalar("a"), UnknownValue::scalar("b")]);

        assert_eq!(
            value,
            UnknownValue::Seq(vec![UnknownValue::scalar("a"), UnknownValue::scalar("b")])
        );
        assert_eq!(value.as_scalar(), None);
    }
}
