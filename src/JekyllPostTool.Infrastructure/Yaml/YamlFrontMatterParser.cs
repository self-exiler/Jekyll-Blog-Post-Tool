using System.Collections;
using System.Globalization;
using JekyllPostTool.Domain.Posts;
using YamlDotNet.Core;
using YamlDotNet.RepresentationModel;

namespace JekyllPostTool.Infrastructure.Yaml;

/// <summary>
/// 将 front matter YAML 文本解析为 FrontMatter 对象。
/// </summary>
public static class YamlFrontMatterParser
{
    private static readonly string[] DateFormats =
    {
        "yyyy-MM-dd HH:mm:ss zzz",
        "yyyy-MM-dd HH:mm:ss",
        "yyyy-MM-dd"
    };

    public static FrontMatter Parse(string yaml)
    {
        var frontMatter = new FrontMatter();
        var unknownFields = new Dictionary<string, object?>();

        if (string.IsNullOrWhiteSpace(yaml))
        {
            frontMatter.UnknownFields = unknownFields;
            return frontMatter;
        }

        var yamlStream = new YamlStream();
        yamlStream.Load(new StringReader(yaml));

        if (yamlStream.Documents.Count == 0)
        {
            frontMatter.UnknownFields = unknownFields;
            return frontMatter;
        }

        var mapping = yamlStream.Documents[0].RootNode as YamlMappingNode;
        if (mapping is null)
        {
            frontMatter.UnknownFields = unknownFields;
            return frontMatter;
        }

        foreach (var entry in mapping.Children)
        {
            var key = ((YamlScalarNode)entry.Key).Value!;
            var value = entry.Value;

            switch (key)
            {
                case "title":
                    frontMatter.Title = GetScalarString(value) ?? string.Empty;
                    break;

                case "date":
                    frontMatter.Date = ParseDate(value);
                    break;

                case "categories":
                    frontMatter.Categories = GetStringList(value).Select(v => new Category(v)).ToList();
                    break;

                case "tags":
                    frontMatter.Tags = GetStringList(value).Select(v => new Tag(v)).ToList();
                    break;

                case "authors":
                    frontMatter.Authors = GetStringList(value);
                    break;

                case "author":
                    // 单数字段迁移为 authors
                    frontMatter.Authors = GetStringList(value);
                    break;

                case "description":
                    frontMatter.Description = GetScalarString(value);
                    break;

                default:
                    unknownFields[key] = ConvertYamlNode(value);
                    break;
            }
        }

        frontMatter.UnknownFields = unknownFields;
        return frontMatter;
    }

    private static string? GetScalarString(YamlNode node)
    {
        if (node is YamlScalarNode scalar)
        {
            return scalar.Value;
        }

        return null;
    }

    private static IReadOnlyList<string> GetStringList(YamlNode node)
    {
        if (node is YamlSequenceNode sequence)
        {
            return sequence.Children
                .OfType<YamlScalarNode>()
                .Select(n => n.Value ?? string.Empty)
                .Where(v => !string.IsNullOrEmpty(v))
                .ToList();
        }

        if (node is YamlScalarNode scalar && !string.IsNullOrEmpty(scalar.Value))
        {
            return new[] { scalar.Value };
        }

        return Array.Empty<string>();
    }

    private static DateTimeOffset? ParseDate(YamlNode node)
    {
        var text = GetScalarString(node);

        if (string.IsNullOrWhiteSpace(text))
        {
            return null;
        }

        if (DateTimeOffset.TryParseExact(text, DateFormats, CultureInfo.InvariantCulture, DateTimeStyles.None, out var dto))
        {
            return dto;
        }

        if (DateTimeOffset.TryParse(text, CultureInfo.InvariantCulture, out dto))
        {
            return dto;
        }

        return null;
    }

    private static object? ConvertYamlNode(YamlNode node)
    {
        return node switch
        {
            YamlScalarNode scalar => scalar.Value,
            YamlSequenceNode sequence => sequence.Children.Select(ConvertYamlNode).ToList(),
            YamlMappingNode mapping => mapping.Children.ToDictionary(
                kvp => ((YamlScalarNode)kvp.Key).Value!,
                kvp => ConvertYamlNode(kvp.Value)),
            _ => null
        };
    }
}
