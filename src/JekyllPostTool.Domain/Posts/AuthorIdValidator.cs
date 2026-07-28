using JekyllPostTool.Domain.Authors;

namespace JekyllPostTool.Domain.Posts;

/// <summary>
/// 校验博文中引用的作者是否都存在于 authors.yml 的领域服务。
/// </summary>
public static class AuthorIdValidator
{
    public static IReadOnlyList<string> FindInvalidIds(IEnumerable<string> authorIds, IEnumerable<Author> authors)
    {
        var validIds = new HashSet<string>(authors.Select(a => a.Id), StringComparer.Ordinal);
        return authorIds.Where(id => !validIds.Contains(id)).ToList();
    }
}
