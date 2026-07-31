using System.Text.Json;
using System.Text.Json.Serialization;

namespace ClaudeUsageAnalyzer.Core.Bridge;

/// <summary>A call from the UI to the host.</summary>
public sealed record BridgeRequest
{
    [JsonPropertyName("id")]
    public required string Id { get; init; }

    [JsonPropertyName("method")]
    public required string Method { get; init; }

    [JsonPropertyName("params")]
    public JsonElement Params { get; init; }
}

/// <summary>
/// The host's answer, discriminated on <c>ok</c>. Exceptions never cross the
/// boundary; a failed call is a normal message with <c>ok: false</c>.
/// </summary>
public sealed record BridgeResponse
{
    [JsonPropertyName("id")]
    public required string Id { get; init; }

    [JsonPropertyName("ok")]
    public required bool Ok { get; init; }

    [JsonPropertyName("result")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public object? Result { get; init; }

    [JsonPropertyName("error")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public BridgeError? Error { get; init; }

    public static BridgeResponse Success(string id, object? result) =>
        new() { Id = id, Ok = true, Result = result };

    public static BridgeResponse Failure(string id, string message, string? code = null) =>
        new() { Id = id, Ok = false, Error = new BridgeError { Message = message, Code = code } };
}

public sealed record BridgeError
{
    [JsonPropertyName("message")]
    public required string Message { get; init; }

    [JsonPropertyName("code")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Code { get; init; }
}

/// <summary>A message the host pushes without being asked.</summary>
public sealed record BridgeEvent
{
    [JsonPropertyName("event")]
    public required string Event { get; init; }

    [JsonPropertyName("payload")]
    public object? Payload { get; init; }
}
