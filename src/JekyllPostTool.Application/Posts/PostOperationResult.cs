using JekyllPostTool.Domain.Common;

namespace JekyllPostTool.Application.Posts;

/// <summary>
/// 博文保存操作结果。
/// </summary>
public sealed class PostOperationResult
{
    public bool IsSuccess { get; }

    public bool IsConflict { get; }

    public bool IsModifiedExternally { get; }

    public string? FilePath { get; }

    public IReadOnlyList<ValidationError> Errors { get; }

    public ConflictResult? Conflict { get; }

    private PostOperationResult(
        bool isSuccess,
        bool isConflict,
        bool isModifiedExternally,
        string? filePath,
        IReadOnlyList<ValidationError>? errors,
        ConflictResult? conflict)
    {
        IsSuccess = isSuccess;
        IsConflict = isConflict;
        IsModifiedExternally = isModifiedExternally;
        FilePath = filePath;
        Errors = errors ?? Array.Empty<ValidationError>();
        Conflict = conflict;
    }

    public static PostOperationResult Success(string filePath) =>
        new(true, false, false, filePath, null, null);

    public static PostOperationResult Failure(IReadOnlyList<ValidationError> errors) =>
        new(false, false, false, null, errors, null);

    public static PostOperationResult WithConflict(ConflictResult conflict) =>
        new(false, true, false, null, null, conflict);

    public static PostOperationResult ModifiedExternally() =>
        new(false, false, true, null, null, null);
}
