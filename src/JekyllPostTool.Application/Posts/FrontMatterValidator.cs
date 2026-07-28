using JekyllPostTool.Domain.Authors;
using JekyllPostTool.Domain.Common;
using JekyllPostTool.Domain.Posts;

namespace JekyllPostTool.Application.Posts;

/// <summary>
/// 校验 front matter 字段是否满足 SRS 约束。
/// </summary>
public sealed class FrontMatterValidator
{
    public IReadOnlyList<ValidationError> Validate(FrontMatter frontMatter, IReadOnlyList<Author> authors)
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

        if (frontMatter.Authors.Count == 0)
        {
            errors.Add(new ValidationError(nameof(FrontMatter.Authors), "authors 至少选择 1 个"));
        }
        else
        {
            var invalidIds = AuthorIdValidator.FindInvalidIds(frontMatter.Authors, authors);
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
