using System.Security.Cryptography;
using System.Text;
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
    private readonly FilenameConflictResolver _conflictResolver;

    public PostEditUseCase(
        IPostRepository postRepository,
        IAuthorRepository authorRepository,
        FilenameConflictResolver conflictResolver)
    {
        _postRepository = postRepository;
        _authorRepository = authorRepository;
        _conflictResolver = conflictResolver;
    }

    public async Task<Post?> LoadAsync(string filePath, CancellationToken cancellationToken = default)
    {
        return await _postRepository.LoadAsync(filePath, cancellationToken);
    }

    public async Task<PostOperationResult> UpdateAsync(
        BlogProject project,
        string originalFilePath,
        FrontMatter frontMatter,
        string? originalContentHash = null,
        ConflictResolutionKind? conflictResolution = null,
        CancellationToken cancellationToken = default)
    {
        // 单次读盘：用于存在检查 + 外部修改检测 + body 提取（原实现读 3 次）
        var content = await _postRepository.ReadAllTextAsync(originalFilePath, cancellationToken);
        if (content is null)
        {
            return PostOperationResult.Failure(new[] { new ValidationError(string.Empty, "博文文件不存在") });
        }

        if (originalContentHash is not null)
        {
            var currentHash = ComputeHash(content);
            if (!string.Equals(currentHash, originalContentHash, StringComparison.Ordinal))
            {
                return PostOperationResult.ModifiedExternally();
            }
        }

        var authors = await _authorRepository.GetAllAsync(cancellationToken);
        var validationErrors = FrontMatterValidator.Validate(frontMatter, authors);

        if (validationErrors.Count > 0)
        {
            return PostOperationResult.Failure(validationErrors);
        }

        var slug = SlugGenerator.Generate(frontMatter.Title);
        var fileName = Post.BuildFileName(frontMatter.Date!.Value, slug);
        var newFilePath = System.IO.Path.Combine(project.PostsDirectory, fileName);

        if (!string.Equals(newFilePath, originalFilePath, StringComparison.OrdinalIgnoreCase))
        {
            var conflict = _conflictResolver.Check(project, fileName);

            if (conflict.HasConflict && conflictResolution is null)
            {
                return PostOperationResult.WithConflict(conflict);
            }

            newFilePath = conflict.ResolveFilePath(conflictResolution);
        }

        // 保留磁盘最新 body（FR-3.10），从已读取的全文切分
        MarkdownSplitter.TrySplit(content, out _, out var currentBody);
        var post = new Post(newFilePath, frontMatter, currentBody);

        await _postRepository.SaveAsync(post, cancellationToken);

        if (!string.Equals(originalFilePath, newFilePath, StringComparison.OrdinalIgnoreCase))
        {
            _postRepository.Delete(originalFilePath);
        }

        return PostOperationResult.Success(newFilePath);
    }

    public async Task<string> ComputeContentHashAsync(string filePath, CancellationToken cancellationToken = default)
    {
        var content = await _postRepository.ReadAllTextAsync(filePath, cancellationToken);
        if (content is null)
        {
            return string.Empty;
        }

        return ComputeHash(content);
    }

    private static string ComputeHash(string content)
    {
        var bytes = Encoding.UTF8.GetBytes(content);
        var hash = SHA256.HashData(bytes);
        return Convert.ToHexString(hash);
    }
}
