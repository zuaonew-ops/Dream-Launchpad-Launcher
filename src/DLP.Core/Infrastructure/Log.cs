namespace DLP.Core.Infrastructure;

public enum LogLevel { Debug, Info, Warn, Error }

/// <summary>极简日志：控制台 + 按日期滚动的本地文件。</summary>
public sealed class Log : IDisposable
{
    private readonly string? _dir;
    private readonly object _gate = new();
    private StreamWriter? _writer;
    private DateTime _day;

    public Log(string? directory = null)
    {
        _dir = directory;
        if (_dir is not null) Directory.CreateDirectory(_dir);
    }

    public event Action<LogLevel, string>? Written;

    public void Debug(string message) => Write(LogLevel.Debug, message);
    public void Info(string message) => Write(LogLevel.Info, message);
    public void Warn(string message) => Write(LogLevel.Warn, message);
    public void Error(string message) => Write(LogLevel.Error, message);

    public void Write(LogLevel level, string message)
    {
        var line = $"[{DateTime.Now:HH:mm:ss}] [{level.ToString().ToUpperInvariant()}] {message}";
        lock (_gate)
        {
            Console.WriteLine(line);
            if (_dir is null) return;
            var today = DateTime.Today;
            if (_writer is null || _day != today)
            {
                _writer?.Dispose();
                _day = today;
                _writer = new StreamWriter(Path.Combine(_dir, $"launcher-{today:yyyy-MM-dd}.log"), append: true) { AutoFlush = true };
            }
            _writer.WriteLine(line);
        }
        Written?.Invoke(level, line);
    }

    public void Dispose()
    {
        lock (_gate) { _writer?.Dispose(); _writer = null; }
    }
}
