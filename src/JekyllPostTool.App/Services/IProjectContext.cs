using JekyllPostTool.Domain.Projects;

namespace JekyllPostTool_App.Services;

/// <summary>
/// 当前选中的博客项目上下文。
/// </summary>
public interface IProjectContext
{
    BlogProject? CurrentProject { get; set; }

    event EventHandler? CurrentProjectChanged;
}
