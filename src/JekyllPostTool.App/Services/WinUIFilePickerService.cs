using Microsoft.UI.Xaml;
using Windows.Storage.Pickers;
using WinRT.Interop;

namespace JekyllPostTool_App.Services;

/// <summary>
/// 基于 WinUI 文件选择器的实现。
/// </summary>
public sealed class WinUIFilePickerService : IFilePickerService
{
    private readonly Window _window;

    public WinUIFilePickerService(Window window)
    {
        _window = window;
    }

    public async Task<string?> PickFolderAsync()
    {
        var picker = new FolderPicker
        {
            SuggestedStartLocation = PickerLocationId.DocumentsLibrary
        };
        picker.FileTypeFilter.Add("*");

        InitializeWithWindow.Initialize(picker, WindowNative.GetWindowHandle(_window));

        var folder = await picker.PickSingleFolderAsync();
        return folder?.Path;
    }

    public async Task<string?> PickFileAsync()
    {
        var picker = new FileOpenPicker
        {
            SuggestedStartLocation = PickerLocationId.DocumentsLibrary
        };
        picker.FileTypeFilter.Add(".md");

        InitializeWithWindow.Initialize(picker, WindowNative.GetWindowHandle(_window));

        var file = await picker.PickSingleFileAsync();
        return file?.Path;
    }

    public async Task<IReadOnlyList<string>?> PickFilesAsync(IEnumerable<string> fileTypes)
    {
        var picker = new FileOpenPicker
        {
            SuggestedStartLocation = PickerLocationId.PicturesLibrary
        };
        foreach (var type in fileTypes)
        {
            picker.FileTypeFilter.Add(type);
        }

        InitializeWithWindow.Initialize(picker, WindowNative.GetWindowHandle(_window));

        var files = await picker.PickMultipleFilesAsync();
        if (files is null || files.Count == 0)
        {
            return null;
        }

        return files.Select(f => f.Path).ToList();
    }
}
