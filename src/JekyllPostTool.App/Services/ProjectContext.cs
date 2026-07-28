using CommunityToolkit.Mvvm.ComponentModel;
using JekyllPostTool.Domain.Projects;

namespace JekyllPostTool_App.Services;

/// <summary>
/// 可观察的当前项目上下文实现。
/// </summary>
public sealed partial class ProjectContext : ObservableObject, IProjectContext
{
    [ObservableProperty]
    private BlogProject? _currentProject;

    public event EventHandler? CurrentProjectChanged;

    partial void OnCurrentProjectChanged(BlogProject? value)
    {
        CurrentProjectChanged?.Invoke(this, EventArgs.Empty);
    }
}
