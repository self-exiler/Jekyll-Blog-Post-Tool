using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Infrastructure.Yaml;

namespace JekyllPostTool.Infrastructure.Posts;

/// <summary>
/// 博文文件格式（`---` 包裹的 YAML front matter + 正文）的唯一权威实现（ADR-007/010）：
/// 定界符切分、YAML 解析/序列化与 LF 归一只在这里发生。Parse 与 Format 互为逆变换，
/// 预览与落盘共用 Format，由构造保证所见即所得。
/// </summary>
public static class PostFileFormat
{
    /// <summary>解析博文全文；无有效 front matter 块时正文为原文。</summary>
    public static (FrontMatter FrontMatter, string Body) Parse(string content)
    {
        MarkdownSplitter.TrySplit(content, out var yaml, out var body);
        return (YamlFrontMatterParser.Parse(yaml), body);
    }

    /// <summary>
    /// 序列化为最终落盘文本：front matter 块，body 非空时另起一段追加。统一 LF（ADR-010）。
    /// </summary>
    public static string Format(FrontMatter frontMatter, string? body = null)
    {
        var yaml = YamlFrontMatterSerializer.Serialize(frontMatter);
        var text = $"---\n{yaml}\n---\n";
        if (!string.IsNullOrEmpty(body))
        {
            text += body;
        }

        // YAML 序列化器在 Windows 上输出 CRLF，整体归一
        return NormalizeNewLines(text);
    }

    /// <summary>CRLF 归一为 LF。</summary>
    public static string NormalizeNewLines(string text) => text.Replace("\r\n", "\n");
}
