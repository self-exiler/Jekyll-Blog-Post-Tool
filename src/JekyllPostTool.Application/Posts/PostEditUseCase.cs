using JekyllPostTool.Domain.Authors;
using JekyllPostTool.Domain.Common;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Domain.Projects;

namespace JekyllPostTool.Application.Posts;

/// <summary>
/// 编辑已有博文 front matter 用例。
/// </summary>
public sealed class PostEditUseCase
{
    private readonly IPostRepository _postRepository;
    private readonly IAuthorRepository _authorRepository;
    private readonly FrontMatterValidator _validator;
    private readonly FilenameConflictResolver _conflictResolver;

    public PostEditUseCase(
        IPostRepository postRepository,
        IAuthorRepository authorRepository,
        FrontMatterValidator validator,
        FilenameConflictResolver conflictResolver)
    {
        _postRepository = postRepository;
        _authorRepository = authorRepository;
        _validator = validator;
        _conflictResolver = conflictResolver;
    }

    public async Task<LoadResult> LoadAsync(string filePath, CancellationToken cancellationToken = default)
    {
        var post = await _postRepository.LoadAsync(filePath, cancellationToken);

        if (post is null)
        {
            return LoadResult.NotFound();
        }

        return LoadResult.Success(post);
    }

    public async Task<PostOperationResult> UpdateAsync(
        BlogProject project,
        string originalFilePath,
        FrontMatter frontMatter,
        string? originalContentHash = null,
        ConflictResolutionKind? conflictResolution = null,
        CancellationToken cancellationToken = default)
    {
        var existing = await _postRepository.LoadAsync(originalFilePath, cancellationToken);

        if (existing is null)
        {
            return PostOperationResult.Failure(new ValidationError(string.Empty, "博文文件不存在"));
        }

        if (originalContentHash is not null)
        {
            var currentHash = await ComputeContentHashAsync(originalFilePath, cancellationToken);
            if (!string.Equals(currentHash, originalContentHash, StringComparison.Ordinal))
            {
                return PostOperationResult.ModifiedExternally();
            }
        }

        var authors = await _authorRepository.GetAllAsync(cancellationToken);
        var validationErrors = _validator.Validate(frontMatter, authors);

        if (validationErrors.Count > 0)
        {
            return PostOperationResult.Failure(validationErrors);
        }

        if (!frontMatter.Date.HasValue)
        {
            return PostOperationResult.Failure(new ValidationError(nameof(FrontMatter.Date), "date 为必填项"));
        }

        var slug = SlugGenerator.Generate(frontMatter.Title);
        var fileName = Post.BuildFileName(frontMatter.Date.Value, slug);
        var newFilePath = System.IO.Path.Combine(project.PostsDirectory, fileName);

        // 若文件名发生变化，需处理冲突
        if (!string.Equals(newFilePath, originalFilePath, StringComparison.OrdinalIgnoreCase))
        {
            var conflict = _conflictResolver.Check(project, fileName);

            if (conflict.HasConflict && conflictResolution is null)
            {
                return PostOperationResult.WithConflict(conflict);
            }

            newFilePath = ResolveFilePath(conflict, conflictResolution);
        }

        // 保存时取磁盘最新 body
        var currentBody = await _postRepository.ReadBodyAsync(originalFilePath, cancellationToken);
        var post = new Post(newFilePath, frontMatter, currentBody);

        await _postRepository.SaveAsync(post, cancellationToken);

        // 若文件名变更，删除原文件
        if (!string.Equals(originalFilePath, newFilePath, StringComparison.OrdinalIgnoreCase))
        {
            File.Delete(originalFilePath);
        }

        return PostOperationResult.Success(newFilePath);
    }

    public async Task<string> ComputeContentHashAsync(string filePath, CancellationToken cancellationToken = default)
    {
        var body = await _postRepository.ReadBodyAsync(filePath, cancellationToken);
        var bytes = System.Text.Encoding.UTF8.GetBytes(body);
        var hash = System.Security.Cryptography.SHA256.HashData(bytes);
        return Convert.ToHexString(hash);
    }

    private static string ResolveFilePath(ConflictResult conflict, ConflictResolutionKind? resolution)
    {
        if (!conflict.HasConflict)
        {
            return conflict.FilePath;
        }

        return resolution switch
        {
            ConflictResolutionKind.AutoSuffix => AppendSuffix(conflict.FilePath, conflict.Resolutions.First(r => r.Kind == ConflictResolutionKind.AutoSuffix).Suffix!.Value),
            ConflictResolutionKind.Overwrite => conflict.FilePath,
            ConflictResolutionKind.RenameTitle or null => throw new InvalidOperationException("需要选择冲突处理方式"),
            _ => throw new InvalidOperationException("不支持的冲突处理方式")
        };
    }

    private static string AppendSuffix(string filePath, int suffix)
    {
        var directory = System.IO.Path.GetDirectoryName(filePath)!;
        var nameWithoutExtension = System.IO.Path.GetFileNameWithoutExtension(filePath);
        var extension = System.IO.Path.GetExtension(filePath);
        return System.IO.Path.Combine(directory, $"{nameWithoutExtension}-{suffix}{extension}");
    }
}

public sealed class LoadResult
{
    public bool IsSuccess { get; }

    public Post? Post { get; }

    private LoadResult(bool isSuccess, Post? post)
    {
        IsSuccess = isSuccess;
        Post = post;
    }

    public static LoadResult Success(Post post) => new(true, post);

    public static LoadResult NotFound() => new(false, null);
}
