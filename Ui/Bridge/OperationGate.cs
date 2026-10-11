using ApiCheckinHelper.Core;

namespace ApiCheckinHelper.Ui.Bridge;

public interface IBridgeHost
{
    OperationLease CreateOperation();

    void PostEvent(object payload);
}

public sealed class OperationLease(CancellationTokenSource source, Action<OperationLease> release) : IDisposable
{
    private bool _disposed;

    public CancellationToken Token => source.Token;

    public bool IsCancellationRequested => source.IsCancellationRequested;

    public void Cancel()
    {
        if (!_disposed)
        {
            try
            {
                source.Cancel();
            }
            catch (ObjectDisposedException error)
            {
                System.Diagnostics.Debug.WriteLine(error);
            }
        }
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }
        _disposed = true;
        source.Dispose();
        release(this);
    }
}

public sealed class OperationGate : IDisposable
{
    private readonly object _gate = new();
    private OperationLease? _current;

    public OperationLease Create()
    {
        lock (_gate)
        {
            if (_current is { IsCancellationRequested: false })
            {
                throw new ConfigError("已有任务正在运行");
            }
            _current?.Dispose();
            OperationLease lease = new(new CancellationTokenSource(), Release);
            _current = lease;
            return lease;
        }
    }

    public void CancelCurrent()
    {
        lock (_gate)
        {
            _current?.Cancel();
        }
    }

    public void Dispose()
    {
        lock (_gate)
        {
            _current?.Cancel();
            _current?.Dispose();
            _current = null;
        }
    }

    private void Release(OperationLease lease)
    {
        lock (_gate)
        {
            if (ReferenceEquals(_current, lease))
            {
                _current = null;
            }
        }
    }
}
