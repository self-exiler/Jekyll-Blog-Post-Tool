using CommunityToolkit.Mvvm.ComponentModel;
using JekyllPostTool.Domain.Projects;

namespace JekyllPostTool_App.Services;

/// <summary>
/// 可观察的当前项目上下文。通知机制唯一：PropertyChanged(nameof(CurrentProject))。
/// </summary>
public sealed partial class ProjectContext : ObservableObject
{
    [ObservableProperty]
    private BlogProject? _currentProject;
}
