using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Infrastructure.Yaml;

namespace JekyllPostTool.Infrastructure.Tests;

/// <summary>
/// YamlFrontMatterSerializer 序列化测试。
/// </summary>
public class YamlFrontMatterSerializerTests
{
    [Fact]
    public void Serialize_AllFields_GeneratesCorrectYaml()
    {
        var fm = new FrontMatter
        {
            Title = "Hello World",
            Date = new DateTimeOffset(2026, 7, 28, 14, 10, 0, TimeSpan.FromHours(8)),
            Categories = new[] { new Category("Blogging"), new Category("Tech") },
            Tags = new[] { new Tag("Jekyll"), new Tag("WinUI") },
            Authors = new[] { "cotes" },
            Description = "A test post"
        };

        var yaml = YamlFrontMatterSerializer.Serialize(fm);

        Assert.Contains("title: Hello World", yaml);
        Assert.Contains("date: 2026-07-28 14:10:00", yaml);
        Assert.Contains("categories:", yaml);
        Assert.Contains("- Blogging", yaml);
        Assert.Contains("- Tech", yaml);
        Assert.Contains("tags:", yaml);
        Assert.Contains("- Jekyll", yaml);
        Assert.Contains("- WinUI", yaml);
        Assert.Contains("authors:", yaml);
        Assert.Contains("- cotes", yaml);
        Assert.Contains("description: A test post", yaml);
    }

    [Fact]
    public void Serialize_EmptyFrontMatter_OnlyTitleAndDate()
    {
        var fm = new FrontMatter
        {
            Title = "Test",
            Date = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero)
        };

        var yaml = YamlFrontMatterSerializer.Serialize(fm);

        Assert.Contains("title: Test", yaml);
        Assert.Contains("date: 2026-01-01", yaml);
        Assert.DoesNotContain("categories:", yaml);
        Assert.DoesNotContain("tags:", yaml);
        Assert.DoesNotContain("authors:", yaml);
        Assert.DoesNotContain("description:", yaml);
    }

    [Fact]
    public void Serialize_NoDate_OmitsDateField()
    {
        var fm = new FrontMatter { Title = "Test", Date = null };

        var yaml = YamlFrontMatterSerializer.Serialize(fm);

        Assert.DoesNotContain("date:", yaml);
    }

    [Fact]
    public void Serialize_EmptyDescription_OmitsDescriptionField()
    {
        var fm = new FrontMatter
        {
            Title = "Test",
            Date = DateTimeOffset.Now,
            Description = "  "
        };

        var yaml = YamlFrontMatterSerializer.Serialize(fm);

        Assert.DoesNotContain("description:", yaml);
    }

    [Fact]
    public void Serialize_UnknownFields_AppendedAfterKnownFields()
    {
        var fm = new FrontMatter
        {
            Title = "Test",
            Date = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero),
            UnknownFields = new OrderedDictionary<string, object?>
            {
                ["custom"] = "value",
                ["enabled"] = true
            }
        };

        var yaml = YamlFrontMatterSerializer.Serialize(fm);

        Assert.Contains("custom: value", yaml);
        Assert.Contains("enabled: true", yaml);
    }

    [Fact]
    public void RoundTrip_SerializeThenParse_PreservesKnownFields()
    {
        var original = new FrontMatter
        {
            Title = "Round Trip",
            Date = new DateTimeOffset(2026, 7, 28, 14, 10, 0, TimeSpan.FromHours(8)),
            Categories = new[] { new Category("A"), new Category("B") },
            Tags = new[] { new Tag("tag1"), new Tag("tag2") },
            Authors = new[] { "cotes", "admin" },
            Description = "Round trip test"
        };

        var yaml = YamlFrontMatterSerializer.Serialize(original);
        var parsed = YamlFrontMatterParser.Parse(yaml);

        Assert.Equal(original.Title, parsed.Title);
        Assert.Equal(original.Date, parsed.Date);
        Assert.Equal(original.Categories.Count, parsed.Categories.Count);
        Assert.Equal(original.Tags.Count, parsed.Tags.Count);
        Assert.Equal(original.Authors.Count, parsed.Authors.Count);
        Assert.Equal(original.Description, parsed.Description);
    }

    [Fact]
    public void RoundTrip_UnknownFields_Preserved()
    {
        var original = new FrontMatter
        {
            Title = "Test",
            Date = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero),
            UnknownFields = new OrderedDictionary<string, object?>
            {
                ["custom_field"] = "custom_value"
            }
        };

        var yaml = YamlFrontMatterSerializer.Serialize(original);
        var parsed = YamlFrontMatterParser.Parse(yaml);

        Assert.Contains("custom_field", parsed.UnknownFields.Keys);
        Assert.Equal("custom_value", parsed.UnknownFields["custom_field"]);
    }
}
