namespace JekyllPostTool.Domain.Common;

/// <summary>
/// 字段级校验错误。
/// </summary>
public sealed record ValidationError(string Field, string Message);
