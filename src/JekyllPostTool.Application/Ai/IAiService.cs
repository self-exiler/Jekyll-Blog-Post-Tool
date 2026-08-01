namespace JekyllPostTool.Application.Ai;

/// <summary>
/// AI 服务抽象（FR-7.2）。
/// </summary>
public interface IAiService
{
    /// <summary>
    /// 从博文正文提取关键字（FR-7.3）。
    /// </summary>
    /// <param name="body">博文正文 markdown。</param>
    /// <param name="maxCount">最大关键字数量，默认 5。</param>
    /// <returns>提取的关键字列表（已去重、去空白）。</returns>
    Task<IReadOnlyList<string>> ExtractKeywordsAsync(string body, int maxCount = 5, CancellationToken cancellationToken = default);
}
