using JekyllPostTool.Domain.Authors;
using JekyllPostTool.Domain.Common;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Domain.Projects;

namespace JekyllPostTool.Application.Posts;

/// <summary>
/// 保存过程的用户提问 seam：生产实现挂 WinUI 对话框，测试注入 fake 穷举冲突序列。
/// </summary>
/// <param name="ResolveConflict">文件名冲突时询问处理方式；返回 null 表示放弃本次保存。</param>
/// <param name="ConfirmOverwrite">选择覆盖现有文件后的二次确认。</param>
public sealed record SavePrompts(
    Func<ConflictResult, Task<ConflictResolutionKind?>> ResolveConflict,
    Func<string, Task<bool>> ConfirmOverwrite);

/// <summary>
/// 博文保存用例：新建（<paramref name="originalFilePath"/> 为 null）与更新统一编排——
/// 校验 → slug → 文件名 → 冲突重试循环 → 外部修改检测 → 改名后删除旧文件。
/// 保存语义（验证顺序、哈希策略、重试策略）集中于此一处。
/// </summary>
public sealed class PostSaveUseCase(
    IPostRepository postRepository,
    IAuthorRepository authorRepository,
    FilenameConflictResolver conflictResolver)
{
    /// <summary>单次读盘的打开结果：展示内容与基线哈希同源。</summary>
    public sealed record LoadedPost(Post Post, string ContentHash);

    public async Task<LoadedPost?> LoadAsync(string filePath, CancellationToken cancellationToken = default)
    {
        var read = await postRepository.ReadAsync(filePath, cancellationToken);
        return read is null ? null : new LoadedPost(read.ToPost(), PostContentHash.Compute(read.Content));
    }

    /// <summary>读取博文当前内容哈希（保存后刷新基线用）。</summary>
    public async Task<string?> GetContentHashAsync(string filePath, CancellationToken cancellationToken = default)
    {
        var read = await postRepository.ReadAsync(filePath, cancellationToken);
        return read is null ? null : PostContentHash.Compute(read.Content);
    }

    /// <summary>
    /// 保存博文。文件名冲突时经 <paramref name="prompts"/> 询问并循环重试；
    /// 外部修改检测返回 <see cref="PostOperationStatus.ModifiedExternally"/>，由调用方确认后续空基线哈希重存。
    /// </summary>
    public async Task<PostOperationResult> SaveAsync(
        BlogProject project,
        FrontMatter frontMatter,
        string? body,
        SavePrompts prompts,
        string? originalFilePath = null,
        string? originalContentHash = null,
        CancellationToken cancellationToken = default)
    {
        var resolution = default(ConflictResolutionKind?);
        while (true)
        {
            var result = originalFilePath is null
                ? await CreateCoreAsync(project, frontMatter, body, resolution, cancellationToken)
                : await UpdateCoreAsync(project, originalFilePath, frontMatter, originalContentHash, resolution, cancellationToken);

            if (!result.IsConflict || result.Conflict is null)
            {
                return result;
            }

            var kind = await prompts.ResolveConflict(result.Conflict);
            if (kind is null)
            {
                // 用户取消冲突处理
                return result;
            }

            if (kind == ConflictResolutionKind.Overwrite
                && !await prompts.ConfirmOverwrite(Path.GetFileName(result.Conflict.FilePath)))
            {
                return result;
            }

            resolution = kind;
        }
    }

    private async Task<PostOperationResult> CreateCoreAsync(
        BlogProject project,
        FrontMatter frontMatter,
        string? body,
        ConflictResolutionKind? conflictResolution,
        CancellationToken cancellationToken)
    {
        var validationErrors = await ValidateAsync(frontMatter, cancellationToken);
        if (validationErrors.Count > 0)
        {
            return PostOperationResult.Failure(validationErrors);
        }

        var conflict = conflictResolver.Check(project, BuildFileName(frontMatter));
        if (conflict.HasConflict && conflictResolution is null)
        {
            return PostOperationResult.WithConflict(conflict);
        }

        var filePath = conflict.ResolveFilePath(conflictResolution);
        await postRepository.SaveAsync(new Post(filePath, frontMatter, body ?? string.Empty), cancellationToken);
        return PostOperationResult.Success(filePath);
    }

    private async Task<PostOperationResult> UpdateCoreAsync(
        BlogProject project,
        string originalFilePath,
        FrontMatter frontMatter,
        string? originalContentHash,
        ConflictResolutionKind? conflictResolution,
        CancellationToken cancellationToken)
    {
        // 单次读盘：存在检查 + 外部修改检测 + body 提取共用同一份内容
        var content = await postRepository.ReadAllTextAsync(originalFilePath, cancellationToken);
        if (content is null)
        {
            return PostOperationResult.Failure([new ValidationError(string.Empty, "博文文件不存在")]);
        }

        if (originalContentHash is not null
            && !string.Equals(PostContentHash.Compute(content), originalContentHash, StringComparison.Ordinal))
        {
            return PostOperationResult.ModifiedExternally();
        }

        var validationErrors = await ValidateAsync(frontMatter, cancellationToken);
        if (validationErrors.Count > 0)
        {
            return PostOperationResult.Failure(validationErrors);
        }

        var newFilePath = Path.Combine(project.PostsDirectory, BuildFileName(frontMatter));
        var renamed = !string.Equals(newFilePath, originalFilePath, StringComparison.OrdinalIgnoreCase);
        if (renamed)
        {
            var conflict = conflictResolver.Check(project, Path.GetFileName(newFilePath));
            if (conflict.HasConflict && conflictResolution is null)
            {
                return PostOperationResult.WithConflict(conflict);
            }

            newFilePath = conflict.ResolveFilePath(conflictResolution);
        }

        // FR-3.10：保留磁盘最新 body，从已读取的全文切分
        MarkdownSplitter.TrySplit(content, out _, out var currentBody);
        await postRepository.SaveAsync(new Post(newFilePath, frontMatter, currentBody), cancellationToken);

        if (renamed)
        {
            try
            {
                postRepository.Delete(originalFilePath);
            }
            catch (Exception ex)
            {
                // 新文件已写入但旧文件未能删除：明确警告，避免新旧两份内容分叉而不自知
                return PostOperationResult.Success(
                    newFilePath,
                    [$"旧文件 {Path.GetFileName(originalFilePath)} 删除失败（{ex.Message}），磁盘上可能残留旧博文"]);
            }
        }

        return PostOperationResult.Success(newFilePath);
    }

    private async Task<IReadOnlyList<ValidationError>> ValidateAsync(FrontMatter frontMatter, CancellationToken cancellationToken)
    {
        var authors = await authorRepository.GetAllAsync(cancellationToken);
        return FrontMatterValidator.Validate(frontMatter, authors);
    }

    private static string BuildFileName(FrontMatter frontMatter)
    {
        var slug = SlugGenerator.Generate(frontMatter.Title);
        return Post.BuildFileName(frontMatter.Date!.Value, slug);
    }
}
