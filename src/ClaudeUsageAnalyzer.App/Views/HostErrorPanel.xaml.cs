using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace ClaudeUsageAnalyzer.Views;

public sealed partial class HostErrorPanel : UserControl
{
    public HostErrorPanel()
    {
        InitializeComponent();
        RetryButton.Click += (_, _) => RetryRequested?.Invoke(this, EventArgs.Empty);
        RuntimeButton.Click += (_, _) => RuntimeInstallRequested?.Invoke(this, EventArgs.Empty);
    }

    public event EventHandler? RetryRequested;

    public event EventHandler? RuntimeInstallRequested;

    public void Show(string detail, bool offerRuntimeInstall = false)
    {
        DetailText.Text = detail;
        RuntimeButton.Visibility = offerRuntimeInstall ? Visibility.Visible : Visibility.Collapsed;
        Visibility = Visibility.Visible;
    }
}
