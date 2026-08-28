using JekyllPostTool.Domain.Common;

namespace JekyllPostTool.Application.Posts;

/// <summary>保存结果状态。</summary>
public enum PostOperationStatus
{
    Saved,
    ValidationFailed,
    Conflict,
    ModifiedExternally
}

/// <summary>
/// 博文保存操作结果。
/// </summary>
public sealed record PostOperationResult(
    PostOperationStatus Status,
    string? FilePath = null,
    IReadOnlyList<ValidationError>? Errors = null,
    ConflictResult? Conflict = null,
    IReadOnlyList<string>? Warnings = null)
{
    public bool IsSuccess => Status == PostOperationStatus.Saved;

    public bool IsConflict => Status == PostOperationStatus.Conflict;

    public bool IsModifiedExternally => Status == PostOperationStatus.ModifiedExternally;

    public static PostOperationResult Success(string filePath, IReadOnlyList<string>? warnings = null) =>
        new(PostOperationStatus.Saved, filePath, Warnings: warnings);

    public static PostOperationResult Failure(IReadOnlyList<ValidationError> errors) =>
        new(PostOperationStatus.ValidationFailed, Errors: errors);

    public static PostOperationResult WithConflict(ConflictResult conflict) =>
        new(PostOperationStatus.Conflict, Conflict: conflict);

    public static PostOperationResult ModifiedExternally() =>
        new(PostOperationStatus.ModifiedExternally);
}
