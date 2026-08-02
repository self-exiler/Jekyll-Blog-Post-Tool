using CommunityToolkit.Mvvm.Input;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool_App.Services;

namespace JekyllPostTool_App.ViewModels;

/// <summary>
/// 博文正文操作 partial：导入正文、插入图片、AI 提取关键字。
/// </summary>
public sealed partial class PostPageViewModel
{
    private static readonly string[] ImageExtensions = { ".jpg", ".jpeg", ".png", ".gif", ".webp", ".svg" };

    [RelayCommand]
    private async Task ImportBodyAsync()
    {
        var filePath = await _filePickerService.PickFileAsync();
        if (string.IsNullOrWhiteSpace(filePath))
        {
            return;
        }

        var imported = await _bodyImporter.ImportAsync(filePath);
        // FR-4.3：默认追加，勾选 ReplaceBodyOnImport 时替换
        Body = ReplaceBodyOnImport ? imported : BodyInsertion.InsertAtCursor(Body, imported, null);
    }

    /// <summary>
    /// 插入图片：复制到 assets/img/{slug}/ 并把 markdown 引用插入到正文指定光标位置（内存，保存时统一写入）。
    /// </summary>
    /// <param name="insertionIndex">光标位置；null 或超出长度时追加到末尾。</param>
    [RelayCommand(CanExecute = nameof(CanInsertImages))]
    private async Task InsertImagesAsync(int? insertionIndex = null)
    {
        var project = _projectContext.CurrentProject;
        if (project is null)
        {
            return;
        }

        var slug = GetCurrentSlug();
        if (string.IsNullOrWhiteSpace(slug))
        {
            return;
        }

        var picked = await _filePickerService.PickFilesAsync(ImageExtensions);
        if (picked is null || picked.Count == 0)
        {
            return;
        }

        string markdown;
        try
        {
            markdown = await _imageInserter.InsertAsync(project, slug, picked, AltText ?? string.Empty);
        }
        catch (Exception ex)
        {
            await _dialogService.ShowInfoAsync("插入失败", ex.Message);
            return;
        }

        if (string.IsNullOrEmpty(markdown))
        {
            await _dialogService.ShowInfoAsync("未插入", "未找到有效的图片文件。");
            return;
        }

        Body = BodyInsertion.InsertAtCursor(Body, markdown, insertionIndex);
        await _dialogService.ShowInfoAsync("插入成功", "markdown 引用已插入到光标处。");
    }

    [RelayCommand(CanExecute = nameof(CanExtractKeywords))]
    private async Task ExtractKeywordsAsync()
    {
        if (string.IsNullOrWhiteSpace(Body))
        {
            await _dialogService.ShowInfoAsync("正文为空", "请先填写或导入正文后再提取关键字。");
            return;
        }

        IsExtractingKeywords = true;
        try
        {
            IReadOnlyList<string> keywords;
            try
            {
                keywords = await _aiService.ExtractKeywordsAsync(Body);
            }
            catch (Exception ex)
            {
                await _dialogService.ShowInfoAsync("提取失败", ex.Message);
                return;
            }

            if (keywords.Count == 0)
            {
                await _dialogService.ShowInfoAsync("未提取到关键字", "AI 未返回有效关键字，请检查正文或稍后重试。");
                return;
            }

            // FR-7.4：替换已有标签
            Tags = string.Join(" ", keywords);
        }
        finally
        {
            IsExtractingKeywords = false;
        }
    }

    private bool CanExtractKeywords() => !IsExtractingKeywords;
}
