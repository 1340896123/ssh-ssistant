using Microsoft.AspNetCore.Mvc;
using SshAssistant.AdminApi.Models;
using SshAssistant.AdminApi.Services;

namespace SshAssistant.AdminApi.Controllers;

[ApiController]
[Route("api/admin")]
public sealed class AdminController(AdminDataStore store) : ControllerBase
{
    private async Task<bool> HasAdminSession()
    {
        var header = Request.Headers.Authorization.ToString();
        const string prefix = "Bearer ";
        if (!header.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        var token = header[prefix.Length..].Trim();
        var session = await store.ValidateSessionAsync(token, "admin");
        return session is not null;
    }

    [HttpPost("login")]
    public async Task<ActionResult<AdminLoginResponse>> Login([FromBody] AdminLoginRequest request)
    {
        try
        {
            return Ok(await store.AdminLoginAsync(request));
        }
        catch (UnauthorizedAccessException error)
        {
            return Unauthorized(new { error = error.Message });
        }
    }

    [HttpPost("refresh")]
    public async Task<ActionResult<AdminLoginResponse>> Refresh([FromBody] RefreshTokenRequest request)
    {
        try
        {
            return Ok(await store.RefreshAdminSessionAsync(request));
        }
        catch (UnauthorizedAccessException error)
        {
            return Unauthorized(new { error = error.Message });
        }
    }

    [HttpGet("dashboard")]
    public async Task<ActionResult<AdminDashboardSnapshot>> GetDashboard()
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }
        return Ok(await store.GetSnapshotAsync());
    }

    [HttpPut("sub-accounts/{subAccountId}/assets")]
    public async Task<ActionResult<EnterpriseSubAccountSummary>> UpdateSubAccountAssets(
        string subAccountId,
        [FromBody] UpdateSubAccountAssetsRequest request)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }
        try
        {
            return Ok(await store.UpdateSubAccountAssetsAsync(subAccountId, request.AssetIds));
        }
        catch (KeyNotFoundException error)
        {
            return NotFound(new { error = error.Message });
        }
    }

    [HttpPut("ai/subscription")]
    public async Task<ActionResult<AiSubscriptionOverview>> UpdateAiSubscription(
        [FromBody] UpdateAiSubscriptionRequest request)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }
        return Ok(await store.UpdateAiSubscriptionAsync(request));
    }

    [HttpPost("ai/plans")]
    public async Task<ActionResult<AiSubscriptionPlanSummary>> UpsertSubscriptionPlan(
        [FromBody] UpsertSubscriptionPlanRequest request)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }
        return Ok(await store.UpsertSubscriptionPlanAsync(request));
    }

    [HttpPost("ai/usage-pricing")]
    public async Task<ActionResult<AiUsagePricingSummary>> UpsertAiUsagePricing(
        [FromBody] UpsertAiUsagePricingRequest request)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }
        return Ok(await store.UpsertAiUsagePricingAsync(request));
    }

    [HttpPost("ai/enterprise-subscriptions")]
    public async Task<ActionResult<EnterpriseSubscriptionSummary>> UpsertEnterpriseSubscription(
        [FromBody] UpsertEnterpriseSubscriptionRequest request)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }

        try
        {
            return Ok(await store.UpsertEnterpriseSubscriptionAsync(request));
        }
        catch (InvalidOperationException error)
        {
            return Conflict(new { error = error.Message });
        }
        catch (KeyNotFoundException error)
        {
            return NotFound(new { error = error.Message });
        }
    }

    [HttpPost("ai/personal-subscriptions")]
    public async Task<ActionResult<PersonalSubscriptionSummary>> UpsertPersonalSubscription(
        [FromBody] UpsertPersonalSubscriptionRequest request)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }

        try
        {
            return Ok(await store.UpsertPersonalSubscriptionAsync(request));
        }
        catch (KeyNotFoundException error)
        {
            return NotFound(new { error = error.Message });
        }
    }

    [HttpPut("billing/invoices/{invoiceId}")]
    public async Task<ActionResult<BillingInvoiceSummary>> UpdateBillingInvoice(
        string invoiceId,
        [FromBody] UpdateBillingInvoiceRequest request)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }

        try
        {
            return Ok(await store.UpdateBillingInvoiceAsync(invoiceId, request));
        }
        catch (InvalidOperationException error)
        {
            return Conflict(new { error = error.Message });
        }
        catch (KeyNotFoundException error)
        {
            return NotFound(new { error = error.Message });
        }
    }

    [HttpPost("billing/payments")]
    public async Task<ActionResult<PaymentTransactionSummary>> CreatePaymentTransaction(
        [FromBody] CreatePaymentTransactionRequest request)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }

        try
        {
            return Ok(await store.CreatePaymentTransactionAsync(request));
        }
        catch (InvalidOperationException error)
        {
            return Conflict(new { error = error.Message });
        }
        catch (KeyNotFoundException error)
        {
            return NotFound(new { error = error.Message });
        }
    }

    [HttpPost("billing/checkout-sessions")]
    public async Task<ActionResult<PaymentTransactionSummary>> CreateCheckoutSession(
        [FromBody] CreateCheckoutSessionRequest request)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }

        try
        {
            return Ok(await store.CreateCheckoutSessionAsync(request));
        }
        catch (InvalidOperationException error)
        {
            return Conflict(new { error = error.Message });
        }
        catch (KeyNotFoundException error)
        {
            return NotFound(new { error = error.Message });
        }
    }

    [HttpPost("billing/payment-providers")]
    public async Task<ActionResult<PaymentProviderConfigSummary>> UpsertPaymentProvider(
        [FromBody] UpsertPaymentProviderConfigRequest request)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }

        return Ok(await store.UpsertPaymentProviderConfigAsync(request));
    }

    [HttpPost("billing/generate-current-cycle")]
    public async Task<ActionResult<GenerateBillingCycleResponse>> GenerateCurrentBillingCycle()
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }

        return Ok(await store.GenerateCurrentBillingCycleAsync());
    }

    [HttpPut("ai/endpoint-sync")]
    public async Task<ActionResult<AiEndpointSyncSettings>> UpdateEndpointSync(
        [FromBody] UpdateEndpointSyncRequest request)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }
        return Ok(await store.UpdateEndpointSyncAsync(request));
    }

    [HttpPost("enterprises")]
    public async Task<ActionResult<EnterpriseSummary>> UpsertEnterprise(
        [FromBody] UpsertEnterpriseRequest request)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }
        return Ok(await store.UpsertEnterpriseAsync(request));
    }

    [HttpDelete("enterprises/{enterpriseId}")]
    public async Task<IActionResult> DeleteEnterprise(string enterpriseId)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }
        try
        {
            await store.DeleteEnterpriseAsync(enterpriseId);
            return NoContent();
        }
        catch (KeyNotFoundException error)
        {
            return NotFound(new { error = error.Message });
        }
    }

    [HttpPost("sub-accounts")]
    public async Task<ActionResult<EnterpriseSubAccountSummary>> UpsertSubAccount(
        [FromBody] UpsertSubAccountRequest request)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }
        return Ok(await store.UpsertSubAccountAsync(request));
    }

    [HttpDelete("sub-accounts/{subAccountId}")]
    public async Task<IActionResult> DeleteSubAccount(string subAccountId)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }
        try
        {
            await store.DeleteSubAccountAsync(subAccountId);
            return NoContent();
        }
        catch (KeyNotFoundException error)
        {
            return NotFound(new { error = error.Message });
        }
    }

    [HttpPost("personal-accounts")]
    public async Task<ActionResult<PersonalAccountSummary>> UpsertPersonalAccount(
        [FromBody] UpsertPersonalAccountRequest request)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }
        return Ok(await store.UpsertPersonalAccountAsync(request));
    }

    [HttpDelete("personal-accounts/{accountId}")]
    public async Task<IActionResult> DeletePersonalAccount(string accountId)
    {
        if (!await HasAdminSession())
        {
            return Unauthorized(new { error = "Missing admin token." });
        }
        try
        {
            await store.DeletePersonalAccountAsync(accountId);
            return NoContent();
        }
        catch (KeyNotFoundException error)
        {
            return NotFound(new { error = error.Message });
        }
    }

}
