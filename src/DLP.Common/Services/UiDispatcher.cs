namespace DLP.App.Services;

/// <summary>把「回到界面线程」这件事抽象出来，WPF 和 Avalonia 各自注册实现。</summary>
public static class UiDispatcher
{
    /// <summary>界面线程调度器；没设置时直接在当前线程执行（CLI/测试场景）。</summary>
    public static Action<Action> Post { get; set; } = action => action();
}
