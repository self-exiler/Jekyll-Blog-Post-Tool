using JekyllPostTool.Application.Posts;
using JekyllPostTool.Domain.Authors;
using JekyllPostTool.Domain.Posts;

namespace JekyllPostTool.Application.Tests;

/// <summary>
/// FrontMatterValidator 单元测试。
/// </summary>
public class FrontMatterValidatorTests
{
    private static readonly Author[] SampleAuthors =
    {
        new("cotes", "Cotes"),
        new("admin", "Admin")
    };

    private static FrontMatter ValidFrontMatter() => new()
    {
        Title = "Test Post",
        Date = DateTimeOffset.Now,
        Categories = new[] { new Category("Blogging") },
        Authors = new[] { "cotes" }
    };

    [Fact]
    public void Validate_AllFieldsValid_ReturnsNoErrors()
    {
        var fm = ValidFrontMatter();
        var errors = FrontMatterValidator.Validate(fm, SampleAuthors);
        Assert.Empty(errors);
    }

    [Fact]
    public void Validate_EmptyTitle_ReturnsError()
    {
        var fm = ValidFrontMatter();
        fm.Title = "  ";
        var errors = FrontMatterValidator.Validate(fm, SampleAuthors);
        Assert.Contains(errors, e => e.Field == nameof(FrontMatter.Title));
    }

    [Fact]
    public void Validate_NullDate_ReturnsError()
    {
        var fm = ValidFrontMatter();
        fm.Date = null;
        var errors = FrontMatterValidator.Validate(fm, SampleAuthors);
        Assert.Contains(errors, e => e.Field == nameof(FrontMatter.Date));
    }

    [Fact]
    public void Validate_MoreThanTwoCategories_ReturnsError()
    {
        var fm = ValidFrontMatter();
        fm.Categories = new[] { new Category("A"), new Category("B"), new Category("C") };
        var errors = FrontMatterValidator.Validate(fm, SampleAuthors);
        Assert.Contains(errors, e => e.Field == nameof(FrontMatter.Categories));
    }

    [Fact]
    public void Validate_ExactlyTwoCategories_NoError()
    {
        var fm = ValidFrontMatter();
        fm.Categories = new[] { new Category("A"), new Category("B") };
        var errors = FrontMatterValidator.Validate(fm, SampleAuthors);
        Assert.DoesNotContain(errors, e => e.Field == nameof(FrontMatter.Categories));
    }

    [Fact]
    public void Validate_NoAuthors_ReturnsNoError()
    {
        // FR-3.8 (v1.2)：authors 可留空
        var fm = ValidFrontMatter();
        fm.Authors = [];
        var errors = FrontMatterValidator.Validate(fm, SampleAuthors);
        Assert.DoesNotContain(errors, e => e.Field == nameof(FrontMatter.Authors));
    }

    [Fact]
    public void Validate_InvalidAuthorId_ReturnsError()
    {
        var fm = ValidFrontMatter();
        fm.Authors = new[] { "cotes", "ghost" };
        var errors = FrontMatterValidator.Validate(fm, SampleAuthors);
        Assert.Contains(errors, e => e.Field == nameof(FrontMatter.Authors));
    }

    [Fact]
    public void Validate_AllFieldsMissing_ReturnsMultipleErrors()
    {
        var fm = new FrontMatter();
        var errors = FrontMatterValidator.Validate(fm, SampleAuthors);
        // title + date 至少 2 个错误（authors 为空不再报错）
        Assert.True(errors.Count >= 2);
    }
}
