using System;
using System.Windows;
using System.Windows.Threading;
using ApiCheckinHelper.Core;
using ApiCheckinHelper.Ui;

namespace ApiCheckinHelper;

public static class Program
{
    [STAThread]
    public static int Main()
    {
        AppLog.Bind();

        Application application = new() { ShutdownMode = ShutdownMode.OnMainWindowClose };
        application.Dispatcher.UnhandledException += OnDispatcherUnhandled;
        AppDomain.CurrentDomain.UnhandledException += OnDomainUnhandled;
        TaskScheduler.UnobservedTaskException += OnUnobservedTask;

        ConfigStore.Bind();
        ConfigStore.Load();

        Window window = new WebViewWindow();
        application.MainWindow = window;
        window.Show();
        int code = application.Run();
        ConfigStore.Flush();
        return code;
    }

    private static void OnDispatcherUnhandled(object sender, DispatcherUnhandledExceptionEventArgs args)
    {
        AppLog.HandleUnhandled(args.Exception);
        args.Handled = true;
    }

    private static void OnDomainUnhandled(object sender, UnhandledExceptionEventArgs args)
    {
        if (args.ExceptionObject is Exception error)
        {
            AppLog.HandleUnhandled(error);
        }
    }

    private static void OnUnobservedTask(object? sender, UnobservedTaskExceptionEventArgs args)
    {
        AppLog.Error("未观察到的任务异常", args.Exception.ToString());
        args.SetObserved();
    }
}
