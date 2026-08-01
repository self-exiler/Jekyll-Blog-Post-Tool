using JekyllPostTool.Domain.Authors;
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
    private readonly FilenameConflictResolver _conflictResolver;

    public PostCreateUseCase(
        IPostRepository postRepository,
        IAuthorRepository authorRepository,
        FilenameConflictResolver conflictResolver)
    {
        _postRepository = postRepository;
        _authorRepository = authorRepository;
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
        var validationErrors = FrontMatterValidator.Validate(frontMatter, authors);

        if (validationErrors.Count > 0)
        {
            return PostOperationResult.Failure(validationErrors);
        }

        var slug = SlugGenerator.Generate(frontMatter.Title);
        var fileName = Post.BuildFileName(frontMatter.Date!.Value, slug);
        var conflict = _conflictResolver.Check(project, fileName);

        if (conflict.HasConflict && conflictResolution is null)
        {
            return PostOperationResult.WithConflict(conflict);
        }

        var filePath = conflict.ResolveFilePath(conflictResolution);
        var post = new Post(filePath, frontMatter, body ?? string.Empty);

        await _postRepository.SaveAsync(post, cancellationToken);
        return PostOperationResult.Success(filePath);
    }
}
