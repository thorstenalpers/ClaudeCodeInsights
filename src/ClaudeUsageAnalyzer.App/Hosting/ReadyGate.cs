namespace ClaudeUsageAnalyzer.Hosting;

/// <summary>
/// Bridges the gap between "the page navigated" and "the page is actually
/// showing something". NavigationCompleted fires long before Svelte has mounted
/// and painted, so the splash waits for the UI to say <c>app.ready</c> instead.
/// </summary>
public sealed class ReadyGate
{
    private readonly TaskCompletionSource _signalled =
        new(TaskCreationOptions.RunContinuationsAsynchronously);

    /// <summary>True when the gate opened on the timeout rather than on the UI.</summary>
    public bool TimedOut { get; private set; }

    public void Signal() => _signalled.TrySetResult();

    /// <summary>
    /// Waits for <c>app.ready</c>. On timeout it returns anyway: a UI that is
    /// merely slow is still better than a window that never reveals itself.
    /// </summary>
    public async Task WaitAsync(TimeSpan timeout)
    {
        var timeoutTask = Task.Delay(timeout);
        var winner = await Task.WhenAny(_signalled.Task, timeoutTask).ConfigureAwait(true);
        TimedOut = winner == timeoutTask;
    }
}
