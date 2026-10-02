use chrono::{FixedOffset, Local, Offset};

/// 时区偏移量格式化（`±hh:mm`）与候选列表生成。无状态纯函数。

/// 构建 -12:00 到 +14:00（步长 30 分钟）的时区候选列表。
pub fn build_options() -> Vec<String> {
    (-12 * 60..=14 * 60)
        .step_by(30)
        .map(|minutes| format_offset_seconds(minutes * 60))
        .collect()
}

/// 把偏移量格式化为 `±hh:mm`，符号与数字不受当前文化影响。
pub fn format_offset(offset: FixedOffset) -> String {
    format_offset_seconds(offset.local_minus_utc())
}

/// `±hh:mm` 文本形态的偏移量格式化，供候选列表与测试直接使用。
pub fn format_offset_seconds(seconds: i32) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let absolute = seconds.unsigned_abs();
    format!("{sign}{:02}:{:02}", absolute / 3600, (absolute % 3600) / 60)
}

/// 解析 `±hh:mm`；失败时回退到本地时区偏移。
pub fn parse_or_local(text: &str) -> FixedOffset {
    if let Some(offset) = parse(text) {
        return offset;
    }
    local_offset()
}

fn parse(text: &str) -> Option<FixedOffset> {
    let chars: Vec<char> = text.trim().chars().collect();
    if chars.len() != 6 {
        return None;
    }

    let sign = match chars[0] {
        '+' => 1,
        '-' => -1,
        _ => return None,
    };

    let digits = |range: std::ops::Range<usize>| -> Option<i32> {
        let mut value = 0;
        for index in range {
            let digit = chars[index].to_digit(10)?;
            value = value * 10 + digit as i32;
        }
        Some(value)
    };

    let hours = digits(1..3)?;
    if chars[3] != ':' {
        return None;
    }
    let minutes = digits(4..6)?;

    FixedOffset::east_opt(sign * (hours * 3600 + minutes * 60))
}

fn local_offset() -> FixedOffset {
    Local::now().offset().fix()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_options_covers_minus12_to_plus14_in_half_hour_steps() {
        let options = build_options();

        assert_eq!(options.len(), 53);
        assert_eq!(options.first().unwrap(), "-12:00");
        assert_eq!(options.last().unwrap(), "+14:00");
        assert!(options.contains(&"+08:00".to_owned()));
        assert!(options.contains(&"+00:00".to_owned()));
        assert!(options.contains(&"+05:30".to_owned()));
    }

    #[test]
    fn format_offset_is_invariant() {
        assert_eq!(format_offset_seconds(8 * 3600), "+08:00");
        assert_eq!(format_offset_seconds(0), "+00:00");
        assert_eq!(format_offset_seconds(-5 * 3600 - 30 * 60), "-05:30");
        assert_eq!(format_offset_seconds(14 * 3600), "+14:00");
    }

    #[test]
    fn parse_accepts_valid_offsets() {
        assert_eq!(parse("+08:00").unwrap().local_minus_utc(), 8 * 3600);
        assert_eq!(parse("-05:30").unwrap().local_minus_utc(), -(5 * 3600 + 30 * 60));
        assert_eq!(parse("+00:00").unwrap().local_minus_utc(), 0);
    }

    #[test]
    fn parse_rejects_malformed_text() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("   "), None);
        assert_eq!(parse("08:00"), None);
        assert_eq!(parse("+8:00"), None);
        assert_eq!(parse("+0800"), None);
        assert_eq!(parse("+08:00:00"), None);
    }

    #[test]
    fn parse_or_local_falls_back_to_local_offset() {
        let fallback = local_offset();

        assert_eq!(parse_or_local("").local_minus_utc(), fallback.local_minus_utc());
        assert_eq!(parse_or_local("nope").local_minus_utc(), fallback.local_minus_utc());
        assert_ne!(parse_or_local("+09:00").local_minus_utc(), i32::MIN);
    }
}
