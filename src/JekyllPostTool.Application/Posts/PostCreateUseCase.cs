using JekyllPostTool.Domain.Authors;
using JekyllPostTool.Domain.Common;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Domain.Projects;

namespace JekyllPostTool.Application.Posts;

/// <summary>
/// 新建博文用例。
/// </summary>
public sealed class PostCreateUseCase
{
    private readonly IPostRepository _postRepository;
    private readonly IAuthorRepository _authorRepository;
    private readonly FrontMatterValidator _validator;
    private readonly FilenameConflictResolver _conflictResolver;

    public PostCreateUseCase(
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

    public async Task<PostOperationResult> CreateAsync(
        BlogProject project,
        FrontMatter frontMatter,
        string? body = null,
        ConflictResolutionKind? conflictResolution = null,
        CancellationToken cancellationToken = default)
    {
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
        var conflict = _conflictResolver.Check(project, fileName);

        if (conflict.HasConflict && conflictResolution is null)
        {
            return PostOperationResult.WithConflict(conflict);
        }

        var filePath = ResolveFilePath(conflict, conflictResolution);
        var post = new Post(filePath, frontMatter, body ?? string.Empty);

        await _postRepository.SaveAsync(post, cancellationToken);
        return PostOperationResult.Success(filePath);
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
