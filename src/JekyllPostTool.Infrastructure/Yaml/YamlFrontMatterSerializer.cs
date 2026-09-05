using System.Collections;
using System.Globalization;
using JekyllPostTool.Domain.Posts;
using YamlDotNet.Core;
using YamlDotNet.RepresentationModel;

namespace JekyllPostTool.Infrastructure.Yaml;

/// <summary>
/// 将 FrontMatter 对象序列化为 front matter YAML 文本。
/// </summary>
public static class YamlFrontMatterSerializer
{
    private const string DateFormat = "yyyy-MM-dd HH:mm:ss zzz";

    public static string Serialize(FrontMatter frontMatter)
    {
        var mapping = new YamlMappingNode();

        AddKnownField(mapping, "title", frontMatter.Title);

        if (frontMatter.Date.HasValue)
        {
            AddKnownField(mapping, "date", frontMatter.Date.Value.ToString(DateFormat, CultureInfo.InvariantCulture));
        }

        if (frontMatter.Categories.Count > 0)
        {
            AddKnownField(mapping, "categories", frontMatter.Categories.Select(c => c.Value));
        }

        if (frontMatter.Tags.Count > 0)
        {
            AddKnownField(mapping, "tags", frontMatter.Tags.Select(t => t.Value));
        }

        if (frontMatter.Authors.Count > 0)
        {
            AddKnownField(mapping, "authors", frontMatter.Authors);
        }

        if (!string.IsNullOrWhiteSpace(frontMatter.Description))
        {
            AddKnownField(mapping, "description", frontMatter.Description);
        }

        foreach (var unknown in frontMatter.UnknownFields)
        {
            mapping.Add(unknown.Key, ConvertValue(unknown.Value));
        }

        var document = new YamlDocument(mapping);
        var yamlStream = new YamlStream(document);

        using var writer = new StringWriter();
        yamlStream.Save(writer, false);

        return writer.ToString().TrimEnd();
    }

    private static void AddKnownField(YamlMappingNode mapping, string key, string value)
    {
        mapping.Add(key, new YamlScalarNode(value));
    }

    private static void AddKnownField(YamlMappingNode mapping, string key, IEnumerable<string> values)
    {
        var sequence = new YamlSequenceNode();
        foreach (var value in values)
        {
            sequence.Add(new YamlScalarNode(value));
        }

        mapping.Add(key, sequence);
    }

    // 解析器产出的未知字段只会是 string / List<object?> / OrderedDictionary 三种，
    // 其余类型经 ToString 兜底为标量
    private static YamlNode ConvertValue(object? value)
    {
        return value switch
        {
            null => new YamlScalarNode(""),
            string s => new YamlScalarNode(s),
            IDictionary dictionary => ConvertMapping(dictionary),
            IEnumerable enumerable and not string => ConvertSequence(enumerable),
            _ => new YamlScalarNode(value.ToString())
        };
    }

    private static YamlNode ConvertMapping(IDictionary dictionary)
    {
        var mapping = new YamlMappingNode();
        foreach (var key in dictionary.Keys)
        {
            mapping.Add(key.ToString()!, ConvertValue(dictionary[key]));
        }

        return mapping;
    }

    private static YamlNode ConvertSequence(IEnumerable enumerable)
    {
        var sequence = new YamlSequenceNode();
        foreach (var item in enumerable)
        {
            sequence.Add(ConvertValue(item));
        }

        return sequence;
    }
}

