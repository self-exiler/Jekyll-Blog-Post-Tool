using JekyllPostTool.Domain.Projects;

namespace JekyllPostTool_App.Services;

/// <summary>
/// 当前选中的博客项目与博文上下文，供跨页共享状态。
/// </summary>
public interface IProjectContext
{
    BlogProject? CurrentProject { get; set; }

    event EventHandler? CurrentProjectChanged;
}
