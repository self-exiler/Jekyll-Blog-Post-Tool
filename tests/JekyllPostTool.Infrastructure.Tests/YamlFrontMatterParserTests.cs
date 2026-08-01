using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Infrastructure.Yaml;

namespace JekyllPostTool.Infrastructure.Tests;

/// <summary>
/// YamlFrontMatterParser 解析测试。
/// </summary>
public class YamlFrontMatterParserTests
{
    [Fact]
    public void Parse_CompleteFrontMatter_ParsesAllKnownFields()
    {
        var yaml = """
title: Hello World
date: 2026-07-28 14:10:00 +08:00
categories:
  - Blogging
  - Tech
tags:
  - Jekyll
  - WinUI
authors:
  - cotes
description: A test post
""";

        var fm = YamlFrontMatterParser.Parse(yaml);

        Assert.Equal("Hello World", fm.Title);
        Assert.True(fm.Date.HasValue);
        Assert.Equal(2026, fm.Date.Value.Year);
        Assert.Equal(7, fm.Date.Value.Month);
        Assert.Equal(28, fm.Date.Value.Day);
        Assert.Equal(2, fm.Categories.Count);
        Assert.Equal("Blogging", fm.Categories[0].Value);
        Assert.Equal("Tech", fm.Categories[1].Value);
        Assert.Equal(2, fm.Tags.Count);
        Assert.Equal("Jekyll", fm.Tags[0].Value);
        Assert.Equal("WinUI", fm.Tags[1].Value);
        Assert.Single(fm.Authors);
        Assert.Equal("cotes", fm.Authors[0]);
        Assert.Equal("A test post", fm.Description);
    }

    [Fact]
    public void Parse_UnknownFields_PreservedInUnknownFields()
    {
        var yaml = """
title: Test
custom_field: value
another:
  - item1
  - item2
""";

        var fm = YamlFrontMatterParser.Parse(yaml);

        Assert.Equal("Test", fm.Title);
        Assert.Equal(2, fm.UnknownFields.Count);
        Assert.Contains("custom_field", fm.UnknownFields.Keys);
        Assert.Contains("another", fm.UnknownFields.Keys);
    }

    [Fact]
    public void Parse_EmptyYaml_ReturnsEmptyFrontMatter()
    {
        var fm = YamlFrontMatterParser.Parse("");

        Assert.Equal(string.Empty, fm.Title);
        Assert.Null(fm.Date);
        Assert.Empty(fm.Categories);
        Assert.Empty(fm.Tags);
        Assert.Empty(fm.Authors);
        Assert.Empty(fm.UnknownFields);
    }

    [Fact]
    public void Parse_WhitespaceYaml_ReturnsEmptyFrontMatter()
    {
        var fm = YamlFrontMatterParser.Parse("   \n  \n");
        Assert.Equal(string.Empty, fm.Title);
    }

    [Fact]
    public void Parse_DateOnly_ParsesCorrectly()
    {
        var fm = YamlFrontMatterParser.Parse("title: Test\ndate: 2026-07-28");
        Assert.True(fm.Date.HasValue);
        Assert.Equal(2026, fm.Date.Value.Year);
    }

    [Fact]
    public void Parse_DateWithTimezone_ParsesCorrectly()
    {
        var fm = YamlFrontMatterParser.Parse("title: Test\ndate: 2026-07-28 14:10:00 +08:00");
        Assert.True(fm.Date.HasValue);
        Assert.Equal(TimeSpan.FromHours(8), fm.Date.Value.Offset);
    }

    [Fact]
    public void Parse_DateWithoutTimezone_ParsesAsLocal()
    {
        var fm = YamlFrontMatterParser.Parse("title: Test\ndate: 2026-07-28 14:10:00");
        Assert.True(fm.Date.HasValue);
    }

    [Fact]
    public void Parse_CategoriesAsScalar_ConvertedToList()
    {
        var fm = YamlFrontMatterParser.Parse("title: Test\ncategories: Blogging");
        Assert.Single(fm.Categories);
        Assert.Equal("Blogging", fm.Categories[0].Value);
    }

    [Fact]
    public void Parse_AuthorField_MigratedToAuthors()
    {
        var fm = YamlFrontMatterParser.Parse("title: Test\nauthor: cotes");
        Assert.Single(fm.Authors);
        Assert.Equal("cotes", fm.Authors[0]);
    }

    [Fact]
    public void Parse_NoDescription_DescriptionIsNull()
    {
        var fm = YamlFrontMatterParser.Parse("title: Test");
        Assert.Null(fm.Description);
    }

    [Fact]
    public void Parse_UnknownFieldNestedMapping_PreservesStructure()
    {
        var yaml = """
title: Test
math:
  enable: true
  katex: false
""";

        var fm = YamlFrontMatterParser.Parse(yaml);

        Assert.Contains("math", fm.UnknownFields.Keys);
    }
}
