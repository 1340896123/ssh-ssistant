using Microsoft.AspNetCore.Mvc;
using SshAssistant.AdminApi.Models;
using SshAssistant.AdminApi.Services;
using System.Text.Json;

namespace SshAssistant.AdminApi.Controllers;

[ApiController]
[Route("api/client")]
public sealed class ClientSyncController(AdminDataStore store) : ControllerBase
{
    [HttpPost("login")]
    public async Task<ActionResult<ClientLoginResponse>> Login([FromBody] ClientLoginRequest request)
    {
        try
        {
            return Ok(await store.LoginAsync(request));
        }
        catch (KeyNotFoundException error)
        {
            return NotFound(new { error = error.Message });
        }
    }

    [HttpPost("refresh")]
    public async Task<ActionResult<ClientLoginResponse>> Refresh([FromBody] RefreshTokenRequest request)
    {
        try
        {
            return Ok(await store.RefreshClientSessionAsync(request));
        }
        catch (UnauthorizedAccessException error)
        {
            return Unauthorized(new { error = error.Message });
        }
    }

    [HttpPost("sync/settings")]
    public async Task<ActionResult<ClientSyncResponse>> SyncSettings([FromBody] ClientSettingsSyncRequest request)
    {
        try
        {
            return Ok(await store.SyncSettingsAsync(request));
        }
        catch (UnauthorizedAccessException error)
        {
            return Unauthorized(new { error = error.Message });
        }
    }

    [HttpPost("sync/assets")]
    public async Task<ActionResult<ClientSyncResponse>> SyncAssets([FromBody] ClientAssetsSyncRequest request)
    {
        try
        {
            return Ok(await store.SyncAssetsAsync(request));
        }
        catch (UnauthorizedAccessException error)
        {
            return Unauthorized(new { error = error.Message });
        }
    }

    [HttpGet("sync/pull")]
    public async Task<ActionResult<ClientSyncResponse>> PullSync(
        [FromQuery] string mode,
        [FromQuery] string accountKey,
        [FromQuery] string accessToken)
    {
        try
        {
            return Ok(await store.PullSyncAsync(mode, accountKey, accessToken));
        }
        catch (UnauthorizedAccessException error)
        {
            return Unauthorized(new { error = error.Message });
        }
    }

    [HttpGet("ai/runtime")]
    public async Task<ActionResult<ClientAiRuntimeResponse>> GetAiRuntime(
        [FromQuery] string accessToken)
    {
        try
        {
            return Ok(await store.GetClientAiRuntimeAsync(accessToken));
        }
        catch (UnauthorizedAccessException error)
        {
            return Unauthorized(new { error = error.Message });
        }
    }

    [HttpGet("subscription")]
    public async Task<ActionResult<ClientSubscriptionSnapshot>> GetSubscriptionSnapshot(
        [FromQuery] string accessToken)
    {
        try
        {
            return Ok(await store.GetClientSubscriptionSnapshotAsync(accessToken));
        }
        catch (UnauthorizedAccessException error)
        {
            return Unauthorized(new { error = error.Message });
        }
    }

    [HttpPost("billing/checkout-sessions")]
    public async Task<ActionResult<PaymentTransactionSummary>> CreateClientCheckoutSession(
        [FromQuery] string accessToken,
        [FromBody] CreateCheckoutSessionRequest request)
    {
        try
        {
            return Ok(await store.CreateClientCheckoutSessionAsync(accessToken, request));
        }
        catch (UnauthorizedAccessException error)
        {
            return Unauthorized(new { error = error.Message });
        }
        catch (KeyNotFoundException error)
        {
            return NotFound(new { error = error.Message });
        }
        catch (InvalidOperationException error)
        {
            return Conflict(new { error = error.Message });
        }
    }

    [HttpPost("ai/proxy/openai")]
    public async Task<IActionResult> ProxyOpenAi(
        [FromQuery] string accessToken,
        [FromBody] JsonElement payload)
    {
        try
        {
            var result = await store.ProxyManagedOpenAiAsync(accessToken, payload.GetRawText());
            return new ContentResult
            {
                Content = result.Content,
                ContentType = "application/json; charset=utf-8",
                StatusCode = result.StatusCode,
            };
        }
        catch (UnauthorizedAccessException error)
        {
            return Unauthorized(new { error = error.Message });
        }
        catch (InvalidOperationException error)
        {
            return Conflict(new { error = error.Message });
        }
    }

    [HttpPost("ai/proxy/anthropic")]
    public async Task<IActionResult> ProxyAnthropic(
        [FromQuery] string accessToken,
        [FromHeader(Name = "anthropic-version")] string anthropicVersion,
        [FromHeader(Name = "anthropic-beta")] string? anthropicBeta,
        [FromHeader(Name = "x-coding-tool")] string? codingToolHeader,
        [FromBody] JsonElement payload)
    {
        try
        {
            var result = await store.ProxyManagedAnthropicAsync(
                accessToken,
                payload.GetRawText(),
                anthropicVersion,
                anthropicBeta,
                codingToolHeader);
            return new ContentResult
            {
                Content = result.Content,
                ContentType = "application/json; charset=utf-8",
                StatusCode = result.StatusCode,
            };
        }
        catch (UnauthorizedAccessException error)
        {
            return Unauthorized(new { error = error.Message });
        }
        catch (InvalidOperationException error)
        {
            return Conflict(new { error = error.Message });
        }
    }

    [HttpPost("billing/webhook")]
    public async Task<ActionResult<PaymentTransactionSummary>> HandleBillingWebhook(
        [FromBody] PaymentWebhookEventRequest request)
    {
        try
        {
            return Ok(await store.HandlePaymentWebhookAsync(request));
        }
        catch (UnauthorizedAccessException error)
        {
            return Unauthorized(new { error = error.Message });
        }
        catch (KeyNotFoundException error)
        {
            return NotFound(new { error = error.Message });
        }
        catch (InvalidOperationException error)
        {
            return Conflict(new { error = error.Message });
        }
    }
}
