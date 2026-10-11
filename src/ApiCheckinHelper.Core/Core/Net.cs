using System.Buffers;
using System.Diagnostics;
using System.Net;
using System.Net.Http;
using System.Net.Sockets;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using ApiCheckinHelper.Core.Json;

namespace ApiCheckinHelper.Core;

public sealed class NetError(string message, string kind = "unknown", bool retryable = false) : Exception(message)
{
    public string Kind { get; } = kind;

    public bool Retryable { get; } = retryable;

    public static NetError Timeout() => new("请求超时", "timeout", true);

    public static NetError Offline() => new("无法连接到站点", "connect", true);
}

public sealed class NetResponse
{
    public required int Status { get; init; }

    public required string Text { get; init; }

    public required string ContentType { get; init; }

    public JsonNode? Data { get; init; }

    public IReadOnlyDictionary<string, string> Cookies { get; init; } = new Dictionary<string, string>();

    public long ElapsedMs { get; init; }

    public bool LooksHtml()
    {
        return ContentType.Contains("text/html", StringComparison.OrdinalIgnoreCase)
            || Text.AsSpan().TrimStart().StartsWith("<");
    }

    public string? Cookie(string name)
    {
        return Cookies.TryGetValue(name, out string? value) && !string.IsNullOrEmpty(value) ? value : null;
    }

    public static NetResponse Synth(int status, JsonNode? data, string? text = null, string contentType = "application/json")
    {
        string body = text ?? data?.ToJsonString() ?? "";
        return new NetResponse
        {
            Status = status,
            Text = body,
            ContentType = contentType,
            Data = data,
        };
    }
}

public static class Net
{
    public const int DefaultTimeoutMs = 15000;
    public const int MaxRetry = 2;
    public const int MaxRedirects = 5;
    public const long MaxResponseBytes = 4 * 1024 * 1024;
    public const double RetryBackoffMs = 350d;

    private const int CopyBufferSize = 8192;
    private const int ConnectTimeoutSeconds = 10;
    private const int ConnectionLifetimeMinutes = 5;
    private const int MaxConnectionsPerServer = 16;

    private static readonly string UserAgent =
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

    private static readonly string[] DefaultHeaders =
    {
        "Accept: application/json, text/plain, */*",
        "Accept-Language: zh-CN,zh;q=0.9,en;q=0.8",
    };

    private static readonly HashSet<int> RetryableStatus = new() { 408, 425, 429, 500, 502, 503, 504 };

    private static readonly HashSet<string> CrossOriginUnsafeHeaders = new(StringComparer.OrdinalIgnoreCase)
    {
        "Authorization",
        "Cookie",
        "Proxy-Authorization",
        "Origin",
        "Referer",
        Checkin.UserHeader,
    };

    private static readonly object OverrideGate = new();
    private static readonly Lazy<HttpClient> Shared = new(BuildClient, isThreadSafe: true);
    private static HttpClient? _override;

    internal static void UseHandler(HttpMessageHandler? handler)
    {
        HttpClient? next = handler is null
            ? null
            : new HttpClient(handler, disposeHandler: true)
            {
                Timeout = System.Threading.Timeout.InfiniteTimeSpan,
            };
        HttpClient? previous;
        lock (OverrideGate)
        {
            previous = _override;
            _override = next;
        }
        previous?.Dispose();
    }

    private static HttpClient Resolved
    {
        get
        {
            lock (OverrideGate)
            {
                return _override ?? Shared.Value;
            }
        }
    }

    internal static bool ValidateRequestUrl(string? value) => NetworkPolicy.ValidateUrl(value, out _);

    private static HttpClient BuildClient()
    {
        SocketsHttpHandler handler = new()
        {
            UseCookies = false,
            AutomaticDecompression = DecompressionMethods.All,
            PooledConnectionLifetime = TimeSpan.FromMinutes(ConnectionLifetimeMinutes),
            MaxConnectionsPerServer = MaxConnectionsPerServer,
            ConnectTimeout = TimeSpan.FromSeconds(ConnectTimeoutSeconds),
            AllowAutoRedirect = false,
            ConnectCallback = ConnectSafeAsync,
        };
        HttpClient client = new(handler, disposeHandler: true)
        {
            Timeout = System.Threading.Timeout.InfiniteTimeSpan,
        };
        client.DefaultRequestHeaders.UserAgent.ParseAdd(UserAgent);
        foreach (string header in DefaultHeaders)
        {
            int split = header.IndexOf(':');
            client.DefaultRequestHeaders.TryAddWithoutValidation(header[..split].Trim(), header[(split + 1)..].Trim());
        }
        return client;
    }

    private static async ValueTask<Stream> ConnectSafeAsync(
        SocketsHttpConnectionContext context,
        CancellationToken cancellation)
    {
        DnsEndPoint endpoint = context.DnsEndPoint;
        if (!NetworkPolicy.IsValidHost(endpoint.Host))
        {
            throw new NetError("目标地址被安全策略阻止", "blocked");
        }
        IPAddress[] resolved = await Dns.GetHostAddressesAsync(endpoint.Host, cancellation).ConfigureAwait(false);
        IPAddress[] allowed = resolved.Where(NetworkPolicy.IsRoutable).ToArray();
        if (allowed.Length == 0)
        {
            throw new NetError("目标地址被安全策略阻止", "blocked");
        }
        Socket socket = new(SocketType.Stream, ProtocolType.Tcp) { NoDelay = true };
        try
        {
            await socket.ConnectAsync(allowed, endpoint.Port, cancellation).ConfigureAwait(false);
            return new NetworkStream(socket, ownsSocket: true);
        }
        catch
        {
            socket.Dispose();
            throw;
        }
    }

    public static TimeSpan TimeoutOf(int timeoutMs)
    {
        int value = Coerce.ToInt(timeoutMs, DefaultTimeoutMs, 500, 600000);
        return TimeSpan.FromMilliseconds(value);
    }

    public static Task<NetResponse> GetAsync(
        string url,
        int timeoutMs = DefaultTimeoutMs,
        IReadOnlyDictionary<string, string>? headers = null,
        CancellationToken deadline = default)
    {
        return SendAsync(HttpMethod.Get, url, timeoutMs, headers, null, deadline);
    }

    public static Task<NetResponse> PostAsync(
        string url,
        int timeoutMs = DefaultTimeoutMs,
        IReadOnlyDictionary<string, string>? headers = null,
        object? body = null,
        CancellationToken deadline = default)
    {
        return SendAsync(HttpMethod.Post, url, timeoutMs, headers, body, deadline);
    }

    private static async Task<NetResponse> SendAsync(
        HttpMethod method,
        string url,
        int timeoutMs,
        IReadOnlyDictionary<string, string>? headers,
        object? body,
        CancellationToken deadline)
    {
        NetError? last = null;
        for (int attempt = 0; attempt <= MaxRetry; attempt++)
        {
            deadline.ThrowIfCancellationRequested();
            try
            {
                NetResponse response = await AttemptAsync(method, url, timeoutMs, headers, body, deadline).ConfigureAwait(false);
                if (RetryableStatus.Contains(response.Status) && attempt < MaxRetry)
                {
                    last = new NetError($"服务端暂时不可用（{response.Status}）", "server", true);
                    await BackoffAsync(attempt, deadline).ConfigureAwait(false);
                    continue;
                }
                return response;
            }
            catch (NetError error)
            {
                last = error;
                if (!error.Retryable || attempt == MaxRetry)
                {
                    throw;
                }
                await BackoffAsync(attempt, deadline).ConfigureAwait(false);
            }
            catch (OperationCanceledException) when (deadline.IsCancellationRequested)
            {
                throw;
            }
            catch (OperationCanceledException)
            {
                NetError error = NetError.Timeout();
                last = error;
                if (attempt == MaxRetry)
                {
                    throw error;
                }
                await BackoffAsync(attempt, deadline).ConfigureAwait(false);
            }
        }
        throw last ?? new NetError("网络请求失败");
    }

    private static async Task BackoffAsync(int attempt, CancellationToken deadline)
    {
        int delay = (int)Math.Min(RetryBackoffMs * (attempt + 1), 1000d);
        await Task.Delay(delay, deadline).ConfigureAwait(false);
    }

    private static async Task<NetResponse> AttemptAsync(
        HttpMethod method,
        string url,
        int timeoutMs,
        IReadOnlyDictionary<string, string>? headers,
        object? body,
        CancellationToken deadline)
    {
        if (!ValidateRequestUrl(url))
        {
            throw new NetError("目标地址被安全策略阻止", "blocked");
        }
        using CancellationTokenSource timeoutSource = new(TimeoutOf(timeoutMs));
        using CancellationTokenSource linked = CancellationTokenSource.CreateLinkedTokenSource(timeoutSource.Token, deadline);
        Stopwatch watch = Stopwatch.StartNew();
        HttpResponseMessage message;
        try
        {
            message = await SendFollowingRedirectsAsync(method, url, headers, body, linked.Token).ConfigureAwait(false);
        }
        catch (HttpRequestException error)
        {
            throw NetErrors.Classify(error);
        }
        catch (OperationCanceledException) when (deadline.IsCancellationRequested)
        {
            throw;
        }
        catch (OperationCanceledException)
        {
            throw NetError.Timeout();
        }

        using (message)
        {
            string text = await ReadTextAsync(message, linked.Token).ConfigureAwait(false);
            watch.Stop();
            string contentType = message.Content.Headers.ContentType?.MediaType ?? "";
            return new NetResponse
            {
                Status = (int)message.StatusCode,
                Text = text,
                ContentType = contentType,
                Data = ParseJson(text, contentType),
                Cookies = ReadCookies(message),
                ElapsedMs = watch.ElapsedMilliseconds,
            };
        }
    }

    private static async Task<HttpResponseMessage> SendFollowingRedirectsAsync(
        HttpMethod method,
        string url,
        IReadOnlyDictionary<string, string>? headers,
        object? body,
        CancellationToken cancellation)
    {
        string currentUrl = url;
        HttpMethod currentMethod = method;
        object? currentBody = body;
        IReadOnlyDictionary<string, string>? currentHeaders = headers;

        for (int redirect = 0; redirect <= MaxRedirects; redirect++)
        {
            cancellation.ThrowIfCancellationRequested();
            if (!ValidateRequestUrl(currentUrl))
            {
                throw new NetError("目标地址被安全策略阻止", "blocked");
            }
            using HttpRequestMessage request = BuildRequest(currentMethod, currentUrl, currentHeaders, currentBody);
            HttpResponseMessage response = await Resolved
                .SendAsync(request, HttpCompletionOption.ResponseHeadersRead, cancellation)
                .ConfigureAwait(false);
            if (!IsRedirect(response))
            {
                return response;
            }
            if (redirect == MaxRedirects)
            {
                response.Dispose();
                throw new NetError("重定向次数过多", "redirect");
            }
            Uri? location = response.Headers.Location;
            int status = (int)response.StatusCode;
            Uri? next = location is null ? null : new Uri(new Uri(currentUrl), location);
            response.Dispose();
            if (next is null || !NetworkPolicy.ValidateUrl(next.AbsoluteUri, out _))
            {
                throw new NetError("重定向目标被安全策略阻止", "blocked");
            }
            if (!SameOrigin(currentUrl, next))
            {
                currentHeaders = StripCrossOriginHeaders(currentHeaders);
            }
            if (ConvertsToGet(currentMethod, status))
            {
                currentMethod = HttpMethod.Get;
                currentBody = null;
            }
            currentUrl = next.AbsoluteUri;
        }
        throw new NetError("重定向次数过多", "redirect");
    }

    private static HttpRequestMessage BuildRequest(
        HttpMethod method,
        string url,
        IReadOnlyDictionary<string, string>? headers,
        object? body)
    {
        HttpRequestMessage request = new(method, url);
        if (headers is not null)
        {
            foreach (KeyValuePair<string, string> pair in headers)
            {
                if (string.Equals(pair.Key, "Content-Type", StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }
                request.Headers.TryAddWithoutValidation(pair.Key, pair.Value);
            }
        }
        if (body is not null)
        {
            request.Content = new StringContent(
                JsonSerializer.Serialize(body, Flexible.Options),
                Encoding.UTF8,
                "application/json");
        }
        return request;
    }

    private static bool IsRedirect(HttpResponseMessage response)
    {
        int status = (int)response.StatusCode;
        return status is 301 or 302 or 303 or 307 or 308;
    }

    private static bool ConvertsToGet(HttpMethod method, int status)
    {
        bool postLike = string.Equals(method.Method, HttpMethod.Post.Method, StringComparison.OrdinalIgnoreCase)
            || string.Equals(method.Method, HttpMethod.Put.Method, StringComparison.OrdinalIgnoreCase)
            || string.Equals(method.Method, HttpMethod.Patch.Method, StringComparison.OrdinalIgnoreCase);
        return status == 303 || (postLike && status is 301 or 302);
    }

    private static bool SameOrigin(string current, Uri next)
    {
        Uri currentUri = new(current, UriKind.Absolute);
        return string.Equals(currentUri.Scheme, next.Scheme, StringComparison.OrdinalIgnoreCase)
            && string.Equals(currentUri.Host, next.Host, StringComparison.OrdinalIgnoreCase)
            && currentUri.Port == next.Port;
    }

    private static Dictionary<string, string> StripCrossOriginHeaders(
        IReadOnlyDictionary<string, string>? headers)
    {
        if (headers is null)
        {
            return new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        }
        return headers
            .Where(pair => !CrossOriginUnsafeHeaders.Contains(pair.Key))
            .ToDictionary(pair => pair.Key, pair => pair.Value, StringComparer.OrdinalIgnoreCase);
    }

    private static async Task<string> ReadTextAsync(HttpResponseMessage message, CancellationToken cancellation)
    {
        long declaredLength = message.Content.Headers.ContentLength ?? 0;
        if (declaredLength > MaxResponseBytes)
        {
            throw new NetError("响应内容过大", "response-too-large");
        }
        await using Stream source = await message.Content.ReadAsStreamAsync(cancellation).ConfigureAwait(false);
        using MemoryStream buffer = new();
        byte[] chunk = ArrayPool<byte>.Shared.Rent(CopyBufferSize);
        try
        {
            while (true)
            {
                int read = await source.ReadAsync(chunk.AsMemory(), cancellation).ConfigureAwait(false);
                if (read == 0)
                {
                    break;
                }
                if (buffer.Length + read > MaxResponseBytes)
                {
                    throw new NetError("响应内容过大", "response-too-large");
                }
                buffer.Write(chunk, 0, read);
            }
        }
        finally
        {
            ArrayPool<byte>.Shared.Return(chunk);
        }
        Encoding encoding = ResolveEncoding(message.Content.Headers.ContentType?.CharSet);
        return encoding.GetString(buffer.GetBuffer(), 0, checked((int)buffer.Length));
    }

    private static Encoding ResolveEncoding(string? charset)
    {
        if (string.IsNullOrWhiteSpace(charset))
        {
            return Encoding.UTF8;
        }
        try
        {
            return Encoding.GetEncoding(charset.Trim('"'));
        }
        catch (ArgumentException)
        {
            return Encoding.UTF8;
        }
    }

    private static JsonNode? ParseJson(string text, string contentType)
    {
        if (string.IsNullOrEmpty(text))
        {
            return null;
        }
        ReadOnlySpan<char> trimmed = text.AsSpan().TrimStart();
        bool looksJson = contentType.Contains("json", StringComparison.OrdinalIgnoreCase)
            || contentType.Length == 0
            || trimmed.StartsWith("{")
            || trimmed.StartsWith("[");
        if (!looksJson)
        {
            return null;
        }
        try
        {
            return JsonNode.Parse(text);
        }
        catch (JsonException)
        {
            return null;
        }
    }

    private static Dictionary<string, string> ReadCookies(HttpResponseMessage message)
    {
        Dictionary<string, string> cookies = new(StringComparer.OrdinalIgnoreCase);
        if (!message.Headers.TryGetValues("Set-Cookie", out IEnumerable<string>? values))
        {
            return cookies;
        }
        foreach (string raw in values)
        {
            int end = raw.IndexOf(';');
            string pair = end >= 0 ? raw[..end] : raw;
            int eq = pair.IndexOf('=');
            if (eq <= 0)
            {
                continue;
            }
            string name = pair[..eq].Trim();
            string value = pair[(eq + 1)..].Trim();
            if (name.Length > 0)
            {
                cookies[name] = value;
            }
        }
        return cookies;
    }
}
