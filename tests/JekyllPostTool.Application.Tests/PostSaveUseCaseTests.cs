using JekyllPostTool.Application.Posts;
using JekyllPostTool.Domain.Authors;
using JekyllPostTool.Domain.Common;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Domain.Projects;

namespace JekyllPostTool.Application.Tests;

/// <summary>
/// PostSaveUseCase 单元测试（新建与更新统一编排），使用 stub 仓储与 fake 提问 seam。
/// </summary>
public class PostSaveUseCaseTests
{
    private static readonly BlogProject Project = new(Path.GetTempPath());

    /// <summary>记录 SaveAsync 写入内容的 stub，模拟磁盘状态。</summary>
    private sealed class StubPostRepository : IPostRepository
    {
        private readonly Dictionary<string, string> _files = new(StringComparer.OrdinalIgnoreCase);

        /// <summary>模拟旧文件被占用导致删除失败。</summary>
        public bool LockDeletes { get; set; }

        public void AddExistingFile(string path, string content = "old") => _files[path] = content;

        public bool Exists(string filePath) => _files.ContainsKey(filePath);

        public void Delete(string filePath)
        {
            if (LockDeletes)
            {
                throw new IOException("文件被其他程序占用");
            }

            _files.Remove(filePath);
        }

        public Task<PostRead?> ReadAsync(string filePath, CancellationToken ct = default)
        {
            if (!_files.TryGetValue(filePath, out var content))
            {
                return Task.FromResult<PostRead?>(null);
            }

            MarkdownSplitter.TrySplit(content, out var yaml, out var body);
            return Task.FromResult<PostRead?>(new PostRead(filePath, content, YamlStubParse(yaml), body));
        }

        public Task SaveAsync(Post post, CancellationToken ct = default)
        {
            _files[post.FilePath] = $"---\ntitle: {post.FrontMatter.Title}\n---\n{post.Body}";
            return Task.CompletedTask;
        }

        public Task<string?> ReadAllTextAsync(string filePath, CancellationToken ct = default)
            => Task.FromResult(_files.TryGetValue(filePath, out var content) ? content : null);

        private static FrontMatter YamlStubParse(string yaml)
        {
            var fm = new FrontMatter();
            foreach (var line in yaml.Split('\n'))
            {
                var idx = line.IndexOf(':');
                if (idx <= 0)
                {
                    continue;
                }

                var key = line[..idx].Trim();
                var value = line[(idx + 1)..].Trim();
                if (key == "title")
                {
                    fm.Title = value;
                }
            }

            return fm;
        }
    }

    private sealed class StubAuthorRepository(IReadOnlyList<Author> authors) : IAuthorRepository
    {
        public Task<IReadOnlyList<Author>> GetAllAsync(CancellationToken ct = default) => Task.FromResult(authors);

        public Task SaveAsync(IReadOnlyList<Author> authors, CancellationToken ct = default) => Task.CompletedTask;
    }

    private static FrontMatter ValidFrontMatter() => new()
    {
        Title = "Hello World",
        Date = new DateTimeOffset(2026, 7, 28, 0, 0, 0, TimeSpan.Zero),
        Authors = new[] { "cotes" }
    };

    private static PostSaveUseCase CreateUseCase(StubPostRepository postRepo) =>
        new(postRepo, new StubAuthorRepository([new Author("cotes", "Cotes")]), new FilenameConflictResolver(postRepo));

    private static SavePrompts Prompts(
        ConflictResolutionKind? conflictAnswer,
        bool confirmOverwrite = true,
        List<ConflictResult>? asked = null) =>
        new(
            ResolveConflict: conflict =>
            {
                asked?.Add(conflict);
                return Task.FromResult(conflictAnswer);
            },
            ConfirmOverwrite: _ => Task.FromResult(confirmOverwrite));

    // ---------- 新建 ----------

    [Fact]
    public async Task SaveAsync_CreateValidInput_SavesAndReturnsSuccess()
    {
        var postRepo = new StubPostRepository();
        var useCase = CreateUseCase(postRepo);

        var result = await useCase.SaveAsync(Project, ValidFrontMatter(), "body", Prompts(null));

        Assert.True(result.IsSuccess);
        Assert.EndsWith("2026-07-28-hello-world.md", result.FilePath);
    }

    [Fact]
    public async Task SaveAsync_CreateInvalidTitle_ReturnsValidationFailure()
    {
        var postRepo = new StubPostRepository();
        var useCase = CreateUseCase(postRepo);

        var fm = ValidFrontMatter();
        fm.Title = "";
        var result = await useCase.SaveAsync(Project, fm, null, Prompts(null));

        Assert.False(result.IsSuccess);
        Assert.Equal(PostOperationStatus.ValidationFailed, result.Status);
        Assert.NotEmpty(result.Errors);
    }

    [Fact]
    public async Task SaveAsync_CreateConflictWithoutAnswer_ReturnsConflict()
    {
        var postRepo = new StubPostRepository();
        postRepo.AddExistingFile(Path.Combine(Project.PostsDirectory, "2026-07-28-hello-world.md"));
        var useCase = CreateUseCase(postRepo);
        var asked = new List<ConflictResult>();

        var result = await useCase.SaveAsync(Project, ValidFrontMatter(), null, Prompts(null, asked: asked));

        Assert.True(result.IsConflict);
        Assert.NotNull(result.Conflict);
        Assert.Single(asked);
        Assert.True(asked[0].AutoSuffix.HasValue);
    }

    [Fact]
    public async Task SaveAsync_CreateConflictCancelStopsLoop()
    {
        var postRepo = new StubPostRepository();
        postRepo.AddExistingFile(Path.Combine(Project.PostsDirectory, "2026-07-28-hello-world.md"));
        var useCase = CreateUseCase(postRepo);
        var asked = new List<ConflictResult>();

        var result = await useCase.SaveAsync(Project, ValidFrontMatter(), null, Prompts(null, asked: asked));

        Assert.True(result.IsConflict);
        // 只询问一次，取消后不再重试
        Assert.Single(asked);
        Assert.False(postRepo.Exists(Path.Combine(Project.PostsDirectory, "2026-07-28-hello-world-1.md")));
    }

    [Fact]
    public async Task SaveAsync_CreateConflictWithOverwriteNotConfirmed_StopsLoop()
    {
        var postRepo = new StubPostRepository();
        var existingPath = Path.Combine(Project.PostsDirectory, "2026-07-28-hello-world.md");
        postRepo.AddExistingFile(existingPath, "external-content");
        var useCase = CreateUseCase(postRepo);

        var result = await useCase.SaveAsync(Project, ValidFrontMatter(), null, Prompts(ConflictResolutionKind.Overwrite, confirmOverwrite: false));

        Assert.True(result.IsConflict);
        Assert.Equal("external-content", (await postRepo.ReadAllTextAsync(existingPath))!);
    }

    [Fact]
    public async Task SaveAsync_CreateConflictWithOverwriteConfirmed_SavesToOriginalPath()
    {
        var postRepo = new StubPostRepository();
        postRepo.AddExistingFile(Path.Combine(Project.PostsDirectory, "2026-07-28-hello-world.md"));
        var useCase = CreateUseCase(postRepo);

        var result = await useCase.SaveAsync(Project, ValidFrontMatter(), null, Prompts(ConflictResolutionKind.Overwrite));

        Assert.True(result.IsSuccess);
        Assert.EndsWith("2026-07-28-hello-world.md", result.FilePath);
    }

    [Fact]
    public async Task SaveAsync_NoConflict_DoesNotAskPrompt()
    {
        var postRepo = new StubPostRepository();
        var useCase = CreateUseCase(postRepo);
        var asked = new List<ConflictResult>();

        var result = await useCase.SaveAsync(Project, ValidFrontMatter(), null, Prompts(ConflictResolutionKind.AutoSuffix, asked: asked));

        Assert.True(result.IsSuccess);
        Assert.Empty(asked);
    }

    // ---------- 更新 ----------

    [Fact]
    public async Task SaveAsync_UpdateUnchangedHash_SavesInPlace()
    {
        var postRepo = new StubPostRepository();
        var originalPath = Path.Combine(Project.PostsDirectory, "2026-01-01-old-title.md");
        var content = "---\ntitle: Old Title\n---\noriginal body";
        postRepo.AddExistingFile(originalPath, content);
        var useCase = CreateUseCase(postRepo);
        var hash = (await useCase.GetContentHashAsync(originalPath))!;

        var fm = ValidFrontMatter();
        fm.Date = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
        fm.Title = "Old Title"; // 同名保存：不改名
        var result = await useCase.SaveAsync(Project, fm, null, Prompts(null), originalFilePath: originalPath, originalContentHash: hash);

        Assert.True(result.IsSuccess);
        Assert.Equal(originalPath, result.FilePath);
        var saved = await postRepo.ReadAllTextAsync(originalPath);
        Assert.Contains("original body", saved); // FR-3.10：保留磁盘最新 body
    }

    [Fact]
    public async Task SaveAsync_UpdateModifiedExternally_ReturnsStatus()
    {
        var postRepo = new StubPostRepository();
        var originalPath = Path.Combine(Project.PostsDirectory, "2026-07-28-hello-world.md");
        postRepo.AddExistingFile(originalPath, "---\ntitle: External Edit\n---\nchanged");
        var useCase = CreateUseCase(postRepo);

        var result = await useCase.SaveAsync(
            Project, ValidFrontMatter(), null, Prompts(null),
            originalFilePath: originalPath, originalContentHash: "deadbeef");

        Assert.True(result.IsModifiedExternally);
        Assert.False(result.IsSuccess);
    }

    [Fact]
    public async Task SaveAsync_UpdateMissingFile_ReturnsFailure()
    {
        var postRepo = new StubPostRepository();
        var useCase = CreateUseCase(postRepo);

        var result = await useCase.SaveAsync(
            Project, ValidFrontMatter(), null, Prompts(null),
            originalFilePath: Path.Combine(Project.PostsDirectory, "ghost.md"));

        Assert.False(result.IsSuccess);
        Assert.Contains(result.Errors, e => e.Message.Contains("不存在"));
    }

    [Fact]
    public async Task SaveAsync_UpdateRename_RemovesOldFile()
    {
        var postRepo = new StubPostRepository();
        var oldPath = Path.Combine(Project.PostsDirectory, "2026-01-01-old-title.md");
        postRepo.AddExistingFile(oldPath, "---\ntitle: Old Title\n---\nkeep body");
        var useCase = CreateUseCase(postRepo);
        var hash = (await useCase.GetContentHashAsync(oldPath))!;

        var result = await useCase.SaveAsync(
            Project, ValidFrontMatter(), null, Prompts(null),
            originalFilePath: oldPath, originalContentHash: hash);

        Assert.True(result.IsSuccess);
        Assert.EndsWith("2026-07-28-hello-world.md", result.FilePath);
        Assert.False(postRepo.Exists(oldPath), "改名保存后旧文件应被删除");
        var saved = await postRepo.ReadAllTextAsync(result.FilePath!);
        Assert.Contains("keep body", saved);
    }

    [Fact]
    public async Task SaveAsync_UpdateRenameDeleteFails_ReturnsWarningButSucceeds()
    {
        var postRepo = new StubPostRepository();
        var oldPath = Path.Combine(Project.PostsDirectory, "2026-01-01-old-title.md");
        postRepo.AddExistingFile(oldPath, "---\ntitle: Old Title\n---\nkeep body");
        var useCase = CreateUseCase(postRepo);
        var hash = (await useCase.GetContentHashAsync(oldPath))!;
        postRepo.LockDeletes = true; // 模拟文件被占用

        var result = await useCase.SaveAsync(
            Project, ValidFrontMatter(), null, Prompts(null),
            originalFilePath: oldPath, originalContentHash: hash);

        Assert.True(result.IsSuccess);
        Assert.NotNull(result.Warnings);
        Assert.Contains(result.Warnings, w => w.Contains("删除失败"));
        Assert.True(postRepo.Exists(oldPath));
    }

    [Fact]
    public async Task SaveAsync_UpdateRenameConflict_AutoSuffixResolves()
    {
        var postRepo = new StubPostRepository();
        var oldPath = Path.Combine(Project.PostsDirectory, "2026-01-01-old-title.md");
        postRepo.AddExistingFile(oldPath, "---\ntitle: Old Title\n---\nbody");
        postRepo.AddExistingFile(Path.Combine(Project.PostsDirectory, "2026-07-28-hello-world.md"), "existing target");
        var useCase = CreateUseCase(postRepo);
        var hash = (await useCase.GetContentHashAsync(oldPath))!;

        var result = await useCase.SaveAsync(
            Project, ValidFrontMatter(), null, Prompts(ConflictResolutionKind.AutoSuffix),
            originalFilePath: oldPath, originalContentHash: hash);

        Assert.True(result.IsSuccess);
        Assert.EndsWith("2026-07-28-hello-world-1.md", result.FilePath);
        Assert.False(postRepo.Exists(oldPath));
    }
}
