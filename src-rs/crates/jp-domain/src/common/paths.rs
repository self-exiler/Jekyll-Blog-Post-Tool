use std::path::{Path, PathBuf};

/// .NET `Path` 语义的本地实现：仓储与用例都以文件名为键做冲突判断，
/// 因此这里必须与 `GetFileName` / `GetExtension` 的行为逐一对齐。

pub fn last_segment(text: &str) -> &str {
    let trimmed = text.trim_end_matches(['/', '\\']);
    match trimmed.rfind(['/', '\\']) {
        Some(index) => &trimmed[index + 1..],
        None => trimmed,
    }
}

/// `Path.GetFileName`
pub fn file_name(path: &Path) -> String {
    last_segment(&path.to_string_lossy()).to_owned()
}

/// `Path.GetExtension`：名字首字符的点不算扩展名。
pub fn split_extension(name: &str) -> (&str, &str) {
    if let Some(index) = name.rfind('.') {
        if index > 0 {
            return (&name[..index], &name[index..]);
        }
    }
    (name, "")
}

pub fn extension(path: &Path) -> String {
    split_extension(&file_name(path)).1.to_owned()
}

pub fn file_name_without_extension(path: &Path) -> String {
    split_extension(&file_name(path)).0.to_owned()
}

pub fn file_name_without_extension_str(name: &str) -> String {
    split_extension(last_segment(name)).0.to_owned()
}

/// `Path.GetDirectoryName`，无父目录时返回空路径。
pub fn parent_directory(path: &Path) -> PathBuf {
    let name = file_name(path);
    let text = path.to_string_lossy();
    let trimmed_len = text.len() - text.trim_end_matches(['/', '\\']).len();
    let head = &text[..text.len() - trimmed_len - name.len()];
    let head = head.trim_end_matches(['/', '\\']);
    if head.is_empty() {
        PathBuf::new()
    } else {
        PathBuf::from(head)
    }
}

/// 在文件名尾部（扩展名之前）插入 `-suffix`，对应 `$"{name}-{suffix}{ext}"`。
pub fn append_suffix(path: &Path, suffix: i64) -> PathBuf {
    let name = file_name(path);
    let (stem, ext) = split_extension(&name);
    let candidate = format!("{stem}-{suffix}{ext}");
    let directory = parent_directory(path);
    if directory.as_os_str().is_empty() {
        PathBuf::from(candidate)
    } else {
        directory.join(candidate)
    }
}

/// `string.Equals(a, b, StringComparison.OrdinalIgnoreCase)`：仅折叠 ASCII 大小写，
/// 与 .NET 一致地不做分隔符与规范化归一。
pub fn eq_ignore_ascii_case(left: &str, right: &str) -> bool {
    left.len() == right.len()
        && left
            .chars()
            .zip(right.chars())
            .all(|(a, b)| a == b || (a.is_ascii() && b.is_ascii() && a.eq_ignore_ascii_case(&b)))
}
