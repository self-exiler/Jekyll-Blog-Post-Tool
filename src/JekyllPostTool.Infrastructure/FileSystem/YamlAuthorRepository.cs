using System.Text;
using JekyllPostTool.Domain.Authors;
using YamlDotNet.Core;
using YamlDotNet.RepresentationModel;
using YamlDotNet.Serialization;
using YamlDotNet.Serialization.NamingConventions;

namespace JekyllPostTool.Infrastructure.FileSystem;

/// <summary>
/// 基于 YamlDotNet 的 authors.yml 仓储实现。
/// </summary>
public sealed class YamlAuthorRepository : IAuthorRepository
{
    private readonly string _filePath;

    private static readonly ISerializer Serializer = new SerializerBuilder()
        .WithNamingConvention(CamelCaseNamingConvention.Instance)
        .ConfigureDefaultValuesHandling(DefaultValuesHandling.OmitNull)
        .Build();

    private static readonly IDeserializer Deserializer = new DeserializerBuilder()
        .WithNamingConvention(CamelCaseNamingConvention.Instance)
        .Build();

    public YamlAuthorRepository(string filePath)
    {
        _filePath = filePath;
    }

    public Task<IReadOnlyList<Author>> GetAllAsync(CancellationToken cancellationToken = default)
    {
        if (!File.Exists(_filePath))
        {
            return Task.FromResult<IReadOnlyList<Author>>(Array.Empty<Author>());
        }

        var yaml = File.ReadAllText(_filePath, Encoding.UTF8);

        if (string.IsNullOrWhiteSpace(yaml))
        {
            return Task.FromResult<IReadOnlyList<Author>>(Array.Empty<Author>());
        }

        var dtos = Deserializer.Deserialize<Dictionary<string, AuthorDto>>(yaml);
        var authors = dtos.Select(kvp => new Author(kvp.Key, kvp.Value.Name, kvp.Value.Twitter, kvp.Value.Url)).ToList();
        return Task.FromResult<IReadOnlyList<Author>>(authors);
    }

    public async Task<Author?> FindByIdAsync(string id, CancellationToken cancellationToken = default)
    {
        var authors = await GetAllAsync(cancellationToken);
        return authors.FirstOrDefault(a => a.Id.Equals(id, StringComparison.Ordinal));
    }

    public Task SaveAsync(IReadOnlyList<Author> authors, CancellationToken cancellationToken = default)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(_filePath)!);

        var dtos = authors.ToDictionary(
            a => a.Id,
            a => new AuthorDto(a.Name, a.Twitter, a.Url));

        var yaml = Serializer.Serialize(dtos);
        File.WriteAllText(_filePath, yaml, Encoding.UTF8);

        return Task.CompletedTask;
    }

    private sealed record AuthorDto(string Name, string? Twitter, string? Url);
}
