using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Domain.Projects;

namespace JekyllPostTool.Application.Posts;

/// <summary>
/// 检测博文文件名冲突并提供候选方案。
/// </summary>
public sealed class FilenameConflictResolver
{
    private readonly IPostRepository _postRepository;

    public FilenameConflictResolver(IPostRepository postRepository)
    {
        _postRepository = postRepository;
    }

    public ConflictResult Check(BlogProject project, string fileName)
    {
        var filePath = System.IO.Path.Combine(project.PostsDirectory, fileName);
        var exists = _postRepository.ExistsAsync(filePath).GetAwaiter().GetResult();

        if (!exists)
        {
            return ConflictResult.NoConflict(filePath);
        }

        var candidates = new List<ConflictResolution>
        {
            ConflictResolution.RenameTitle,
            ConflictResolution.AutoSuffix(FindNextAvailableSuffix(project, fileName)),
            ConflictResolution.Overwrite
        };

        return ConflictResult.Conflict(filePath, candidates);
    }

    private int FindNextAvailableSuffix(BlogProject project, string fileName)
    {
        var nameWithoutExtension = System.IO.Path.GetFileNameWithoutExtension(fileName);
        var extension = System.IO.Path.GetExtension(fileName);
        var suffix = 1;

        while (true)
        {
            var candidate = $"{nameWithoutExtension}-{suffix}{extension}";
            var candidatePath = System.IO.Path.Combine(project.PostsDirectory, candidate);
            var exists = _postRepository.ExistsAsync(candidatePath).GetAwaiter().GetResult();

            if (!exists)
            {
                return suffix;
            }

            suffix++;
        }
    }
}

public sealed class ConflictResult
{
    public bool HasConflict { get; }

    public string FilePath { get; }

    public IReadOnlyList<ConflictResolution> Resolutions { get; }

    private ConflictResult(bool hasConflict, string filePath, IReadOnlyList<ConflictResolution> resolutions)
    {
        HasConflict = hasConflict;
        FilePath = filePath;
        Resolutions = resolutions;
    }

    public static ConflictResult NoConflict(string filePath) => new(false, filePath, Array.Empty<ConflictResolution>());

    public static ConflictResult Conflict(string filePath, IReadOnlyList<ConflictResolution> resolutions) =>
        new(true, filePath, resolutions);
}

public sealed class ConflictResolution
{
    public ConflictResolutionKind Kind { get; }

    public int? Suffix { get; }

    private ConflictResolution(ConflictResolutionKind kind, int? suffix = null)
    {
        Kind = kind;
        Suffix = suffix;
    }

    public static ConflictResolution RenameTitle => new(ConflictResolutionKind.RenameTitle);

    public static ConflictResolution AutoSuffix(int suffix) => new(ConflictResolutionKind.AutoSuffix, suffix);

    public static ConflictResolution Overwrite => new(ConflictResolutionKind.Overwrite);
}

public enum ConflictResolutionKind
{
    RenameTitle,
    AutoSuffix,
    Overwrite
}
