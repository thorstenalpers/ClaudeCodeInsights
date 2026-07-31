namespace ClaudeUsageAnalyzer.Core;

/// <summary>
/// Every path the app writes to. Nothing is written next to the executable and
/// nothing is written into the directories the app reads transcripts from —
/// those are treated as read-only, foreign territory.
/// </summary>
public static class AppPaths
{
    public static string DataDirectory { get; } = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "ClaudeUsageAnalyzer");

    /// <summary>
    /// WebView2 would otherwise create "&lt;exe&gt;.exe.WebView2" next to the
    /// executable, which turns the publish folder into a mutable directory.
    /// </summary>
    public static string WebViewUserDataDirectory { get; } = Path.Combine(DataDirectory, "WebView2");

    public static string DatabasePath { get; } = Path.Combine(DataDirectory, "usage.db");

    public static string SettingsPath { get; } = Path.Combine(DataDirectory, "settings.json");

    public static void EnsureCreated()
    {
        Directory.CreateDirectory(DataDirectory);
        Directory.CreateDirectory(WebViewUserDataDirectory);
    }
}
