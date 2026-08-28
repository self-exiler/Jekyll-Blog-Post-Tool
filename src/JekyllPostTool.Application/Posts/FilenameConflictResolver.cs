using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Domain.Projects;

namespace JekyllPostTool.Application.Posts;

/// <summary>
/// 检测博文文件名冲突并提供候选方案。
/// </summary>
public sealed class FilenameConflictResolver(IPostRepository postRepository)
{
    public ConflictResult Check(BlogProject project, string fileName)
    {
        var filePath = Path.Combine(project.PostsDirectory, fileName);
        return postRepository.Exists(filePath)
            ? new ConflictResult(filePath, FindNextAvailableSuffix(project, fileName))
            : new ConflictResult(filePath, null);
    }

    private int FindNextAvailableSuffix(BlogProject project, string fileName)
    {
        var nameWithoutExtension = Path.GetFileNameWithoutExtension(fileName);
        var extension = Path.GetExtension(fileName);
        var suffix = 1;

        while (postRepository.Exists(Path.Combine(
                   project.PostsDirectory, $"{nameWithoutExtension}-{suffix}{extension}")))
        {
            suffix++;
        }

        return suffix;
    }
}

/// <summary>
/// 冲突检测结果：<paramref name="autoSuffix"/> 为冲突时首个可用序号，无冲突时为 null。
/// </summary>
public sealed record ConflictResult(string FilePath, int? AutoSuffix)
{
    public bool HasConflict => AutoSuffix.HasValue;

    public string ResolveFilePath(ConflictResolutionKind? resolution) => (resolution, HasConflict) switch
    {
        (_, false) => FilePath,
        (ConflictResolutionKind.AutoSuffix, true) => AppendSuffix(FilePath, AutoSuffix!.Value),
        (ConflictResolutionKind.Overwrite, _) => FilePath,
        (null, _) => throw new InvalidOperationException("需要选择冲突处理方式"),
        _ => throw new InvalidOperationException("不支持的冲突处理方式")
    };

    private static string AppendSuffix(string filePath, int suffix)
    {
        var directory = Path.GetDirectoryName(filePath)!;
        var nameWithoutExtension = Path.GetFileNameWithoutExtension(filePath);
        var extension = Path.GetExtension(filePath);
        return Path.Combine(directory, $"{nameWithoutExtension}-{suffix}{extension}");
    }
}

public enum ConflictResolutionKind
{
    AutoSuffix,
    Overwrite
}
