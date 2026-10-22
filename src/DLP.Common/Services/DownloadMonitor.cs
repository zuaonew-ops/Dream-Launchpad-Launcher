using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using DLP.App.Mvvm;

namespace DLP.App.Services;

/// <summary>一个下载/安装任务，给右下角任务弹窗和下载页详情共用。</summary>
public sealed class DownloadTaskViewModel : ObservableObject
{
    private string _title = "";
    private string _stage = "准备中";
    private double _percent;
    private bool _isIndeterminate = true;
    private bool _isDone;
    private bool _isFailed;
    private string _speed = "";
    private string _detail = "";
    private DateTimeOffset _lastBytesAt = DateTimeOffset.UtcNow;
    private long _lastBytes;

    public DownloadTaskViewModel(string title)
    {
        _title = title;
        StartedAt = DateTimeOffset.Now;
    }

    public event PropertyChangedEventHandler? PropertyChanged2;

    public string Title { get => _title; private set => SetProperty(ref _title, value); }
    public string Stage { get => _stage; private set => SetProperty(ref _stage, value); }
    public string Detail { get => _detail; private set => SetProperty(ref _detail, value); }
    public double Percent { get => _percent; private set => SetProperty(ref _percent, value); }
    public bool IsIndeterminate { get => _isIndeterminate; private set => SetProperty(ref _isIndeterminate, value); }
    public bool IsDone { get => _isDone; private set { if (SetProperty(ref _isDone, value)) { OnPropertyChanged(nameof(IsActive)); } } }
    public bool IsFailed { get => _isFailed; private set { if (SetProperty(ref _isFailed, value)) { OnPropertyChanged(nameof(IsActive)); } } }
    public string Speed { get => _speed; private set => SetProperty(ref _speed, value); }
    public DateTimeOffset StartedAt { get; }
    public ObservableCollection<string> Lines { get; } = [];

    public bool IsActive => !IsDone && !IsFailed;
    public double PercentText => Math.Round(_percent, 1);
    public string PercentLabel => IsIndeterminate ? "" : $"{_percent:0.#}%";

    /// <summary>更新阶段文字和进度；completed/total 为文件计数，bytes 用于算速度。</summary>
    public void Update(string stage, int? completed = null, int? total = null, long? bytes = null, long? totalBytes = null)
    {
        if (!string.IsNullOrWhiteSpace(stage))
        {
            Stage = stage;
            AddLine(stage);
        }

        if (completed is { } done && total is { } all && all > 0)
        {
            IsIndeterminate = false;
            Percent = Math.Clamp(done * 100.0 / all, 0, 100);
            Detail = $"{done}/{all}";
        }
        else if (bytes is { } b && totalBytes is { } tb && tb > 0)
        {
            IsIndeterminate = false;
            Percent = Math.Clamp(b * 100.0 / tb, 0, 100);
        }

        if (bytes is { } current)
        {
            var now = DateTimeOffset.UtcNow;
            var seconds = (now - _lastBytesAt).TotalSeconds;
            if (seconds >= 0.9 && _lastBytes > 0 && current > _lastBytes)
            {
                var perSecond = (current - _lastBytes) / seconds;
                Speed = FormatSpeed(perSecond);
                if (totalBytes is { } tb2 && tb2 > current && perSecond > 1024)
                {
                    var remain = TimeSpan.FromSeconds((tb2 - current) / perSecond);
                    Detail = remain.TotalHours >= 1
                        ? $"剩余 {remain.TotalHours:0.#} 小时"
                        : $"剩余 {remain.Minutes:00}:{remain.Seconds:00}";
                }
            }
            _lastBytes = current;
            _lastBytesAt = now;
        }

        OnPropertyChanged(nameof(PercentLabel));
        OnPropertyChanged(nameof(PercentText));
    }

    public void Complete(string? message = null)
    {
        if (!string.IsNullOrWhiteSpace(message)) AddLine(message);
        IsDone = true;
        IsIndeterminate = false;
        Percent = 100;
        Speed = "";
        Stage = "已完成";
        OnPropertyChanged(nameof(PercentLabel));
    }

    public void Fail(string message)
    {
        AddLine(message);
        IsFailed = true;
        IsIndeterminate = false;
        Speed = "";
        Stage = "失败";
    }

    private void AddLine(string line)
    {
        UiDispatcher.Post(() =>
        {
            Lines.Add($"{DateTime.Now:HH:mm:ss}  {line}");
            while (Lines.Count > 40) Lines.RemoveAt(0);
        });
    }

    private static string FormatSpeed(double bytesPerSecond) => bytesPerSecond switch
    {
        >= 1024 * 1024 => $"{bytesPerSecond / 1024 / 1024:0.0} MB/s",
        >= 1024 => $"{bytesPerSecond / 1024:0.#} KB/s",
        _ => $"{bytesPerSecond:0} B/s",
    };
}

/// <summary>全局下载任务队列：右下角弹窗、下载页详情都从这里取数据。</summary>
public sealed class DownloadMonitor : ObservableObject
{
    private DownloadTaskViewModel? _current;

    public ObservableCollection<DownloadTaskViewModel> Tasks { get; } = [];

    /// <summary>最近一次更新的任务，下载页详情面板显示它。</summary>
    public DownloadTaskViewModel? Current
    {
        get => _current;
        private set => SetProperty(ref _current, value);
    }

    public bool HasActive => Tasks.Any(t => t.IsActive);

    public DownloadTaskViewModel Begin(string title)
    {
        var task = new DownloadTaskViewModel(title);
        UiDispatcher.Post(() =>
        {
            Tasks.Add(task);
            Current = task;
            while (Tasks.Count > 8) Tasks.RemoveAt(0);
            RaiseActive();
        });
        return task;
    }

    public void Finish(DownloadTaskViewModel task, string? message = null)
    {
        task.Complete(message);
        UiDispatcher.Post(() =>
        {
            RaiseActive();
            if (ReferenceEquals(Current, task)) Current = Tasks.LastOrDefault(t => t.IsActive) ?? task;
        });
    }

    public void Fail(DownloadTaskViewModel task, string message)
    {
        task.Fail(message);
        UiDispatcher.Post(RaiseActive);
    }

    /// <summary>清理已经结束的任务（弹窗关闭时调用）。</summary>
    public void ClearFinished()
    {
        UiDispatcher.Post(() =>
        {
            foreach (var task in Tasks.Where(t => !t.IsActive).ToList()) Tasks.Remove(task);
            RaiseActive();
        });
    }

    private void RaiseActive() => OnPropertyChanged(nameof(HasActive));
}
