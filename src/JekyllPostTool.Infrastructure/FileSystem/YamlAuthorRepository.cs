using System.Text;
using JekyllPostTool.Domain.Authors;
using YamlDotNet.Serialization;
using YamlDotNet.Serialization.NamingConventions;

namespace JekyllPostTool.Infrastructure.FileSystem;

/// <summary>
/// 基于 YamlDotNet 的 authors.yml 仓储实现。
/// 路径在每次调用时通过 pathResolver 解析，支持当前项目切换。
/// </summary>
public sealed class YamlAuthorRepository : IAuthorRepository
{
    private readonly Func<string?> _pathResolver;

    private static readonly ISerializer Serializer = new SerializerBuilder()
        .WithNamingConvention(CamelCaseNamingConvention.Instance)
        .ConfigureDefaultValuesHandling(DefaultValuesHandling.OmitNull)
        .Build();

    private static readonly IDeserializer Deserializer = new DeserializerBuilder()
        .WithNamingConvention(CamelCaseNamingConvention.Instance)
        .Build();

    public YamlAuthorRepository(Func<string?> pathResolver)
    {
        _pathResolver = pathResolver;
    }

    public async Task<IReadOnlyList<Author>> GetAllAsync(CancellationToken cancellationToken = default)
    {
        var filePath = _pathResolver();
        if (string.IsNullOrWhiteSpace(filePath) || !File.Exists(filePath))
        {
            return Array.Empty<Author>();
        }

        var yaml = await File.ReadAllTextAsync(filePath, Encoding.UTF8, cancellationToken);
        if (string.IsNullOrWhiteSpace(yaml))
        {
            return Array.Empty<Author>();
        }

        var dtos = Deserializer.Deserialize<Dictionary<string, AuthorDto>>(yaml);
        var authors = dtos
            .Where(kvp => kvp.Value is { Name: not null })
            .Select(kvp => new Author(kvp.Key, kvp.Value.Name!, kvp.Value.Twitter, kvp.Value.Url))
            .ToList();
        return authors;
    }

    public async Task SaveAsync(IReadOnlyList<Author> authors, CancellationToken cancellationToken = default)
    {
        var filePath = _pathResolver() ?? throw new InvalidOperationException("未选择博客项目，无法保存作者信息。");
        Directory.CreateDirectory(Path.GetDirectoryName(filePath)!);

        var dtos = authors.ToDictionary(
            a => a.Id,
            a => new AuthorDto { Name = a.Name, Twitter = a.Twitter, Url = a.Url });

        var yaml = Serializer.Serialize(dtos);
        // UTF-8 无 BOM（ADR-010）
        await File.WriteAllTextAsync(filePath, yaml, new UTF8Encoding(false), cancellationToken);
    }

    private sealed class AuthorDto
    {
        public string? Name { get; set; }
        public string? Twitter { get; set; }
        public string? Url { get; set; }
    }
}
