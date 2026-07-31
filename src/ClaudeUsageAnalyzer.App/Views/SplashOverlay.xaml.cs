using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace ClaudeUsageAnalyzer.Views;

public sealed partial class SplashOverlay : UserControl
{
    public SplashOverlay()
    {
        InitializeComponent();
        Loaded += (_, _) => PulseStoryboard.Begin();
        Unloaded += (_, _) => PulseStoryboard.Stop();
    }

    public void SetStatus(string text) => StatusText.Text = text;

    /// <summary>Stops the loop so the animation is not left running behind the WebView.</summary>
    public void StopAnimation() => PulseStoryboard.Stop();
}
