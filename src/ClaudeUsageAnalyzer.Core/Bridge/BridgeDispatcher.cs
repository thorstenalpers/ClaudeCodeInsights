using System.Text.Json;
using Microsoft.Extensions.Logging;

namespace ClaudeUsageAnalyzer.Core.Bridge;

public delegate Task<object?> BridgeHandler(JsonElement parameters, CancellationToken cancellationToken);

/// <summary>
/// Routes an incoming <see cref="BridgeRequest"/> to its handler. A method
/// without a registered handler does not exist — there is no fallback, so a
/// typo in the UI contract fails loudly instead of silently doing nothing.
/// </summary>
public sealed class BridgeDispatcher(ILogger<BridgeDispatcher> logger)
{
    private readonly Dictionary<string, BridgeHandler> _handlers = new(StringComparer.Ordinal);

    /// <summary>The registered method names, used by the contract-sync test.</summary>
    public IReadOnlyCollection<string> Methods => _handlers.Keys;

    public void Register(string method, BridgeHandler handler)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(method);
        ArgumentNullException.ThrowIfNull(handler);

        if (!_handlers.TryAdd(method, handler))
        {
            throw new InvalidOperationException($"Bridge method '{method}' is already registered.");
        }
    }

    public async Task<BridgeResponse> DispatchAsync(BridgeRequest request, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(request);

        if (!_handlers.TryGetValue(request.Method, out var handler))
        {
            logger.LogWarning("Bridge call to unknown method {Method}", request.Method);
            return BridgeResponse.Failure(request.Id, $"Unknown method '{request.Method}'.", "unknown-method");
        }

        try
        {
            var result = await handler(request.Params, cancellationToken).ConfigureAwait(false);
            return BridgeResponse.Success(request.Id, result);
        }
        catch (OperationCanceledException)
        {
            return BridgeResponse.Failure(request.Id, "The operation was cancelled.", "cancelled");
        }
        catch (Exception ex)
        {
            // The message is shown to the user, so it must never carry a secret.
            // Handlers that touch credentials are responsible for not putting one
            // into an exception in the first place.
            logger.LogError(ex, "Bridge method {Method} failed", request.Method);
            return BridgeResponse.Failure(request.Id, ex.Message, "handler-failed");
        }
    }
}
