use sha2::{Digest, Sha256};

/// 博文全文内容哈希（SHA-256 大写十六进制），用于外部修改检测。
///
/// 大写与 .NET `Convert.ToHexString` 一致，两侧哈希需可互相比对。
pub fn compute_hash(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02X}"));
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_known_sha256_vector() {
        assert_eq!(
            compute_hash("abc"),
            "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
        );
    }

    #[test]
    fn empty_content_has_stable_hash() {
        assert_eq!(
            compute_hash(""),
            "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855"
        );
    }

    #[test]
    fn chinese_content_hashes_utf8_bytes() {
        assert_eq!(compute_hash("中文"), compute_hash("中文"));
        assert_ne!(compute_hash("中文"), compute_hash("English"));
    }
}
