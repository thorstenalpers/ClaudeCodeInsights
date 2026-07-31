using ClaudeUsageAnalyzer.Core;
using ClaudeUsageAnalyzer.Core.Bridge;
using Microsoft.Extensions.Logging;
using Microsoft.Web.WebView2.Core;
using System.Text.Json;
using WebView2Control = Microsoft.UI.Xaml.Controls.WebView2;

namespace ClaudeUsageAnalyzer.Hosting;

/// <summary>
/// Owns the WebView2: where it stores its profile, where it may navigate, and
/// which messages it is allowed to send to the host.
/// </summary>
public sealed class WebViewHost(
    WebView2Control webView,
    BridgeDispatcher dispatcher,
    ILogger<WebViewHost> logger)
{
    /// <summary>
    /// A real origin rather than a file:// path, so the page is a secure context
    /// and both the CSP and the origin check below have something to name.
    /// </summary>
    public const string VirtualHost = "usageanalyzer.local";

    public const string StartUrl = $"https://{VirtualHost}/index.html";

    private static readonly JsonSerializerOptions SerializerOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
    };

    public event EventHandler<CoreWebView2ProcessFailedEventArgs>? ProcessFailed;

    public event EventHandler<CoreWebView2NavigationCompletedEventArgs>? NavigationCompleted;

    public async Task InitializeAsync()
    {
        AppPaths.EnsureCreated();

        // Redirecting the user data folder is the whole reason the environment is
        // built by hand. Left to itself WebView2 creates "<exe>.exe.WebView2" next
        // to the executable and turns the publish folder into a mutable directory.
        // The environment must be handed over before anything touches Source —
        // setting Source first creates a default environment and this call then
        // throws about a different CoreWebView2Environment.
        var environment = await CoreWebView2Environment.CreateWithOptionsAsync(
            browserExecutableFolder: null,
            userDataFolder: AppPaths.WebViewUserDataDirectory,
            options: new CoreWebView2EnvironmentOptions());

        await webView.EnsureCoreWebView2Async(environment);

        var core = webView.CoreWebView2;
        core.SetVirtualHostNameToFolderMapping(
            VirtualHost,
            Path.Combine(AppContext.BaseDirectory, "wwwroot"),
            CoreWebView2HostResourceAccessKind.DenyCors);

        ApplyHardening(core);

        core.WebMessageReceived += OnWebMessageReceived;
        core.ProcessFailed += (_, e) => ProcessFailed?.Invoke(this, e);
        webView.NavigationCompleted += (_, e) => NavigationCompleted?.Invoke(this, e);
    }

    public void Navigate() => webView.Source = new Uri(StartUrl);

    public void PostEvent(string name, object? payload)
    {
        var message = new BridgeEvent { Event = name, Payload = payload };
        webView.CoreWebView2?.PostWebMessageAsJson(JsonSerializer.Serialize(message, SerializerOptions));
    }

    private static void ApplyHardening(CoreWebView2 core)
    {
        var settings = core.Settings;
        settings.AreDefaultContextMenusEnabled = false;
        settings.IsSwipeNavigationEnabled = false;
        settings.IsStatusBarEnabled = false;
        settings.AreBrowserAcceleratorKeysEnabled = false;
        settings.IsPasswordAutosaveEnabled = false;
        settings.IsGeneralAutofillEnabled = false;
#if DEBUG
        settings.AreDevToolsEnabled = true;
#else
        settings.AreDevToolsEnabled = false;
#endif

        // The app is a fixed set of local pages. Anything that tries to leave it
        // — a link, a redirect, a popup, a download — is either a bug or an
        // attack, and neither deserves to succeed silently.
        core.NavigationStarting += (_, e) =>
        {
            if (!IsOwnOrigin(e.Uri))
            {
                e.Cancel = true;
            }
        };
        core.NewWindowRequested += (_, e) => e.Handled = true;
        core.DownloadStarting += (_, e) => e.Cancel = true;
    }

    private static bool IsOwnOrigin(string? uri) =>
        Uri.TryCreate(uri, UriKind.Absolute, out var parsed)
        && parsed.Scheme == Uri.UriSchemeHttps
        && string.Equals(parsed.Host, VirtualHost, StringComparison.OrdinalIgnoreCase);

    private async void OnWebMessageReceived(CoreWebView2 sender, CoreWebView2WebMessageReceivedEventArgs args)
    {
        if (!IsOwnOrigin(args.Source))
        {
            logger.LogWarning("Rejected a bridge message from a foreign origin");
            return;
        }

        BridgeRequest? request;
        try
        {
            request = JsonSerializer.Deserialize<BridgeRequest>(args.WebMessageAsJson, SerializerOptions);
        }
        catch (JsonException ex)
        {
            logger.LogWarning(ex, "Rejected a malformed bridge message");
            return;
        }

        if (request is null)
        {
            return;
        }

        var response = await dispatcher.DispatchAsync(request, CancellationToken.None);
        sender.PostWebMessageAsJson(JsonSerializer.Serialize(response, SerializerOptions));
    }
}
