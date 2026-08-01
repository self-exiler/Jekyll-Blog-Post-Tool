using JekyllPostTool.Domain.Authors;
using JekyllPostTool.Domain.Common;
using JekyllPostTool.Domain.Posts;

namespace JekyllPostTool.Application.Posts;

/// <summary>
/// 校验 front matter 字段是否满足 SRS 约束。无状态，静态调用。
/// </summary>
public static class FrontMatterValidator
{
    public static IReadOnlyList<ValidationError> Validate(FrontMatter frontMatter, IReadOnlyList<Author> authors)
    {
        var errors = new List<ValidationError>();

        if (string.IsNullOrWhiteSpace(frontMatter.Title))
        {
            errors.Add(new ValidationError(nameof(FrontMatter.Title), "title 为必填项"));
        }

        if (!frontMatter.Date.HasValue)
        {
            errors.Add(new ValidationError(nameof(FrontMatter.Date), "date 为必填项"));
        }

        if (frontMatter.Categories.Count > 2)
        {
            errors.Add(new ValidationError(nameof(FrontMatter.Categories), "categories 最多 2 个"));
        }

        // FR-3.8 (v1.2)：authors 可留空；非空时校验 id 是否存在
        if (frontMatter.Authors.Count > 0)
        {
            var validIds = new HashSet<string>(authors.Select(a => a.Id), StringComparer.Ordinal);
            var invalidIds = frontMatter.Authors.Where(id => !validIds.Contains(id)).ToList();
            if (invalidIds.Count > 0)
            {
                errors.Add(new ValidationError(
                    nameof(FrontMatter.Authors),
                    $"以下作者不存在：{string.Join(", ", invalidIds)}"));
            }
        }

        return errors;
    }
}
