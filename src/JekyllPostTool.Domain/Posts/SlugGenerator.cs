using System.Globalization;
using System.Text;
using System.Text.RegularExpressions;

namespace JekyllPostTool.Domain.Posts;

/// <summary>
/// 将博文 title 转换为文件名安全 slug 的领域服务。
/// </summary>
public static partial class SlugGenerator
{
    /// <summary>
    /// 生成 slug：中文原样保留、英文小写、空格转 -、连续空格/符号压缩为单个 -、去除非法字符、去除首尾 -。
    /// </summary>
    public static Slug Generate(string title)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(title);

        var builder = new StringBuilder();

        foreach (var ch in title.Normalize(NormalizationForm.FormC).Trim())
        {
            if (IsCjk(ch))
            {
                // 中文字符原样保留（须在 IsLetter 之前判断，否则 CJK 会被当作普通字母进入小写化分支）
                builder.Append(ch);
            }
            else if (char.IsLetter(ch))
            {
                builder.Append(char.ToLowerInvariant(ch));
            }
            else if (char.IsDigit(ch))
            {
                builder.Append(ch);
            }
            else if (char.IsWhiteSpace(ch))
            {
                builder.Append('-');
            }
            else if (ch is '-' or '_')
            {
                builder.Append('-');
            }
            else
            {
                // 其他符号视为分隔符
                builder.Append('-');
            }
        }

        var slug = builder.ToString();

        // 连续 - 压缩为单个
        slug = MultipleHyphensRegex().Replace(slug, "-");

        // 去除首尾 -
        slug = slug.Trim('-');

        return string.IsNullOrEmpty(slug)
            ? new Slug("untitled")
            : new Slug(slug);
    }

    private static bool IsCjk(char ch)
    {
        // CJK Unified Ideographs, Extension A, Compatibility Ideographs
        var category = CharUnicodeInfo.GetUnicodeCategory(ch);
        return category == UnicodeCategory.OtherLetter &&
               ((ch >= '\u4E00' && ch <= '\u9FFF') ||
                (ch >= '\u3400' && ch <= '\u4DBF') ||
                (ch >= '\uF900' && ch <= '\uFAFF'));
    }

    [GeneratedRegex(@"-{2,}")]
    private static partial Regex MultipleHyphensRegex();
}
