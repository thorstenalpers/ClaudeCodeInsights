using ClaudeUsageAnalyzer.Core.Bridge;
using ClaudeUsageAnalyzer.Hosting;
using Microsoft.Extensions.Logging.Abstractions;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media.Animation;
using System.Diagnostics;
using Windows.UI;

namespace ClaudeUsageAnalyzer;

public sealed partial class MainWindow : Window
{
    /// <summary>
    /// Long enough that a cold WebView2 start on a slow machine is not cut off,
    /// short enough that a hung renderer does not leave the user staring at a
    /// splash screen wondering whether the app is broken.
    /// </summary>
    private static readonly TimeSpan ReadyTimeout = TimeSpan.FromSeconds(15);

    private readonly ReadyGate _readyGate = new();
    private readonly BridgeDispatcher _dispatcher = new(NullLogger<BridgeDispatcher>.Instance);
    private readonly WebViewHost _host;

    public MainWindow()
    {
        InitializeComponent();

        Title = "ClaudeUsageAnalyzer";
        AppWindow.SetIcon(Path.Combine(AppContext.BaseDirectory, "Assets", "app.ico"));

        // Match the WebView's own background to the theme before it is revealed,
        // otherwise a dark-mode start flashes white for one frame.
        WebView.DefaultBackgroundColor = IsDarkTheme() ? Color.FromArgb(255, 10, 10, 10) : Colors.White;

        _host = new WebViewHost(WebView, _dispatcher, NullLogger<WebViewHost>.Instance);
        RegisterBridgeMethods();

        ErrorPanel.RetryRequested += async (_, _) => await StartAsync();
        ErrorPanel.RuntimeInstallRequested += (_, _) => OpenWebView2RuntimeDownload();

        Activated += OnFirstActivated;
    }

    private bool _started;

    private async void OnFirstActivated(object sender, WindowActivatedEventArgs args)
    {
        if (_started)
        {
            return;
        }

        _started = true;
        Activated -= OnFirstActivated;
        await StartAsync();
    }

    private async Task StartAsync()
    {
        ErrorPanel.Visibility = Visibility.Collapsed;
        Splash.Visibility = Visibility.Visible;
        Splash.SetStatus("Starting the interface…");

        try
        {
            await _host.InitializeAsync();
        }
        catch (Exception ex)
        {
            ShowError($"The WebView2 runtime could not be started.\n\n{ex.Message}", offerRuntimeInstall: true);
            return;
        }

        _host.NavigationCompleted += (_, e) =>
        {
            if (!e.IsSuccess)
            {
                ShowError($"The interface failed to load ({e.WebErrorStatus}).");
            }
        };
        _host.ProcessFailed += (_, e) => ShowError($"The browser process stopped unexpectedly ({e.ProcessFailedKind}).");

        Splash.SetStatus("Loading…");
        _host.Navigate();

        await _readyGate.WaitAsync(ReadyTimeout);

        // On timeout the UI is revealed anyway: it is most likely just slow, and
        // hiding a working interface forever is the worse failure.
        if (_readyGate.TimedOut)
        {
            Debug.WriteLine("app.ready did not arrive within the timeout; revealing the WebView anyway.");
        }

        RevealWebView();
    }

    private void RegisterBridgeMethods()
    {
        _dispatcher.Register("app.ready", (_, _) =>
        {
            _readyGate.Signal();
            return Task.FromResult<object?>(new { ok = true });
        });

        _dispatcher.Register("app.getInfo", (_, _) => Task.FromResult<object?>(new
        {
            version = typeof(MainWindow).Assembly.GetName().Version?.ToString() ?? "0.0.0",
            dataDir = Core.AppPaths.DataDirectory,
            readyTimedOut = _readyGate.TimedOut,
        }));
    }

    /// <summary>Cross-fades from the splash to the WebView.</summary>
    private void RevealWebView()
    {
        if (ErrorPanel.Visibility == Visibility.Visible)
        {
            return;
        }

        WebView.Visibility = Visibility.Visible;

        var storyboard = new Storyboard();
        storyboard.Children.Add(Fade(WebView, from: 0, to: 1));
        storyboard.Children.Add(Fade(Splash, from: 1, to: 0));
        storyboard.Completed += (_, _) =>
        {
            Splash.StopAnimation();
            Splash.Visibility = Visibility.Collapsed;
        };
        storyboard.Begin();
    }

    private static DoubleAnimation Fade(DependencyObject target, double from, double to)
    {
        var animation = new DoubleAnimation
        {
            From = from,
            To = to,
            Duration = new Duration(TimeSpan.FromMilliseconds(220)),
            EasingFunction = new CubicEase { EasingMode = EasingMode.EaseOut },
        };
        Storyboard.SetTarget(animation, target);
        Storyboard.SetTargetProperty(animation, "Opacity");
        return animation;
    }

    private void ShowError(string detail, bool offerRuntimeInstall = false)
    {
        Splash.StopAnimation();
        Splash.Visibility = Visibility.Collapsed;
        WebView.Visibility = Visibility.Collapsed;
        ErrorPanel.Show(detail, offerRuntimeInstall);
    }

    private bool IsDarkTheme() =>
        Content is FrameworkElement root && root.ActualTheme == ElementTheme.Dark;

    private static void OpenWebView2RuntimeDownload() =>
        Process.Start(new ProcessStartInfo("https://developer.microsoft.com/microsoft-edge/webview2/") { UseShellExecute = true });
}
