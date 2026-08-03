using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using JekyllPostTool.Application.Ai;
using JekyllPostTool_App.Services;

namespace JekyllPostTool_App.ViewModels;

/// <summary>
/// 高级功能页视图模型（FR-7.x：AI 设置）。
/// </summary>
public sealed partial class AdvancedPageViewModel : ObservableObject
{
    private readonly AiSettingsService _aiSettingsService;
    private readonly IDialogService _dialogService;

    [ObservableProperty]
    private string _aiBaseUrl = string.Empty;

    [ObservableProperty]
    private string _aiApiKey = string.Empty;

    [ObservableProperty]
    private string _aiModel = string.Empty;

    public AdvancedPageViewModel(AiSettingsService aiSettingsService, IDialogService dialogService)
    {
        _aiSettingsService = aiSettingsService;
        _dialogService = dialogService;

        _ = LoadAiSettingsAsync().ContinueWith(
            static t => System.Diagnostics.Debug.WriteLine($"[AdvancedPageVM] 加载 AI 设置失败: {t.Exception}"),
            TaskContinuationOptions.OnlyOnFaulted);
    }

    [RelayCommand]
    private async Task LoadAiSettingsAsync()
    {
        var settings = await _aiSettingsService.GetAsync();
        AiBaseUrl = settings.BaseUrl;
        AiApiKey = settings.ApiKey;
        AiModel = settings.Model;
    }

    [RelayCommand]
    private async Task SaveAiSettingsAsync()
    {
        var settings = new AiSettings
        {
            BaseUrl = AiBaseUrl?.Trim() ?? string.Empty,
            ApiKey = AiApiKey?.Trim() ?? string.Empty,
            Model = AiModel?.Trim() ?? string.Empty
        };

        await _aiSettingsService.SetAsync(settings);
        await _dialogService.ShowInfoAsync("已保存", "AI 配置已保存。");
    }

    [RelayCommand]
    private async Task ClearAiSettingsAsync()
    {
        AiBaseUrl = string.Empty;
        AiApiKey = string.Empty;
        AiModel = string.Empty;
        await _aiSettingsService.SetAsync(new AiSettings());
    }
}
