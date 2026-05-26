using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Globalization;
using Microsoft.EntityFrameworkCore;
using SshAssistant.AdminApi.Data;
using SshAssistant.AdminApi.Entities;
using SshAssistant.AdminApi.Models;

namespace SshAssistant.AdminApi.Services;

public sealed class AdminDataStore(AdminDbContext dbContext, IHttpClientFactory httpClientFactory)
{
    private const string DefaultCheckoutReturnUrl = "sshstar://billing/success";
    private const string DefaultCheckoutCancelUrl = "sshstar://billing/cancel";

    private sealed class StripeLikeProviderConfig
    {
        public string CheckoutBaseUrl { get; set; } = "https://payments.example.com/stripe-checkout";
        public string WebhookMode { get; set; } = "stripe-like";
        public string ApiBaseUrl { get; set; } = "https://api.stripe.com";
        public string SecretApiKey { get; set; } = string.Empty;
        public string StripeApiVersion { get; set; } = "2024-06-20";
        public int WebhookToleranceSeconds { get; set; } = 300;
        public string SuccessUrl { get; set; } = DefaultCheckoutReturnUrl;
        public string CancelUrl { get; set; } = DefaultCheckoutCancelUrl;
    }

    private sealed class StripeLikeWebhookPayload
    {
        public string EventType { get; set; } = string.Empty;
        public string ExternalReference { get; set; } = string.Empty;
        public string InvoiceId { get; set; } = string.Empty;
        public double Amount { get; set; }
        public string Currency { get; set; } = "USD";
        public string Status { get; set; } = string.Empty;
    }

    private sealed class ClientCloudAssetRecord
    {
        public ClientCloudAsset? Asset { get; set; }
        public ClientCloudAccessEndpoint? DefaultAccessEndpoint { get; set; }
        public ClientCloudCredentialRef? DefaultCredentialRef { get; set; }
    }

    private sealed class ClientCloudAsset
    {
        public long? Id { get; set; }
        public string? CloudId { get; set; }
        public string Name { get; set; } = string.Empty;
        public string Host { get; set; } = string.Empty;
        public int Port { get; set; } = 22;
        public string Platform { get; set; } = "Linux";
        public int? FolderId { get; set; }
        public int? EnvId { get; set; }
        public string[] Labels { get; set; } = Array.Empty<string>();
        public string? Owner { get; set; }
        public string Criticality { get; set; } = "medium";
        public string? DefaultWorkspacePath { get; set; }
        public int? AccessEndpointId { get; set; }
        public string? BastionChainId { get; set; }
        public string? HealthSummary { get; set; }
        public long? LastAccessedAt { get; set; }
        public bool IsFavorite { get; set; }
        public int? GroupId { get; set; }
    }

    private sealed class ClientCloudAccessEndpoint
    {
        public long? Id { get; set; }
        public long AssetId { get; set; }
        public string Name { get; set; } = string.Empty;
        public string Host { get; set; } = string.Empty;
        public int Port { get; set; } = 22;
        public string Username { get; set; } = "root";
        public string? AuthType { get; set; }
        public long? CredentialRefId { get; set; }
        public long? SshKeyId { get; set; }
        public string? JumpHost { get; set; }
        public int? JumpPort { get; set; }
        public string? JumpUsername { get; set; }
        public string? JumpPassword { get; set; }
    }

    private sealed class ClientCloudCredentialRef
    {
        public long? Id { get; set; }
        public string Name { get; set; } = string.Empty;
        public string CredentialKind { get; set; } = "password";
        public string? Username { get; set; }
        public string? Secret { get; set; }
        public long? SshKeyId { get; set; }
        public long? AssetId { get; set; }
        public long CreatedAt { get; set; }
        public long UpdatedAt { get; set; }
    }

    private static readonly TimeSpan AdminSessionLifetime = TimeSpan.FromHours(8);
    private static readonly TimeSpan ClientSessionLifetime = TimeSpan.FromDays(7);
    private static readonly TimeSpan AdminRefreshLifetime = TimeSpan.FromDays(14);
    private static readonly TimeSpan ClientRefreshLifetime = TimeSpan.FromDays(30);

    private sealed record ScopedSubscriptionAccessState(
        bool Enabled,
        string Reason,
        AiSubscriptionOverview Subscription);

    private static string EnsureHashedSecret(string secret)
    {
        if (string.IsNullOrWhiteSpace(secret))
        {
            return string.Empty;
        }

        return secret.StartsWith("$2", StringComparison.Ordinal)
            ? secret
            : BCrypt.Net.BCrypt.HashPassword(secret);
    }

    private static bool VerifySecret(string storedSecret, string providedSecret)
    {
        if (string.IsNullOrWhiteSpace(storedSecret) || string.IsNullOrWhiteSpace(providedSecret))
        {
            return false;
        }

        if (storedSecret.StartsWith("$2", StringComparison.Ordinal))
        {
            return BCrypt.Net.BCrypt.Verify(providedSecret, storedSecret);
        }

        return storedSecret == providedSecret;
    }

    private async Task<AuthSessionEntity> CreateSessionAsync(
        string sessionType,
        string subjectId,
        string subjectMode,
        string role,
        TimeSpan lifetime,
        TimeSpan refreshLifetime)
    {
        var session = new AuthSessionEntity
        {
            Token = Convert.ToHexString(Guid.NewGuid().ToByteArray()),
            RefreshToken = Convert.ToHexString(Guid.NewGuid().ToByteArray()),
            SessionType = sessionType,
            SubjectId = subjectId,
            SubjectMode = subjectMode,
            Role = role,
            CreatedAt = DateTimeOffset.UtcNow,
            ExpiresAt = DateTimeOffset.UtcNow.Add(lifetime),
            RefreshExpiresAt = DateTimeOffset.UtcNow.Add(refreshLifetime),
        };

        dbContext.AuthSessions.Add(session);
        await dbContext.SaveChangesAsync();
        return session;
    }

    public async Task<AuthSessionEntity?> ValidateSessionAsync(string token, string sessionType)
    {
        var session = await dbContext.AuthSessions.FirstOrDefaultAsync(
            item => item.Token == token && item.SessionType == sessionType && item.RevokedAt == null);

        if (session is null)
        {
            return null;
        }

        if (session.ExpiresAt <= DateTimeOffset.UtcNow)
        {
            session.RevokedAt = DateTimeOffset.UtcNow;
            await dbContext.SaveChangesAsync();
            return null;
        }

        if (!await IsSessionSubjectStillValidAsync(session))
        {
            session.RevokedAt = DateTimeOffset.UtcNow;
            await dbContext.SaveChangesAsync();
            return null;
        }

        return session;
    }

    public async Task<AuthSessionEntity?> RefreshSessionAsync(string refreshToken, string sessionType)
    {
        var session = await dbContext.AuthSessions.FirstOrDefaultAsync(
            item => item.RefreshToken == refreshToken && item.SessionType == sessionType && item.RevokedAt == null);

        if (session is null)
        {
            return null;
        }

        if (session.RefreshExpiresAt <= DateTimeOffset.UtcNow)
        {
            session.RevokedAt = DateTimeOffset.UtcNow;
            await dbContext.SaveChangesAsync();
            return null;
        }

        if (!await IsSessionSubjectStillValidAsync(session))
        {
            session.RevokedAt = DateTimeOffset.UtcNow;
            await dbContext.SaveChangesAsync();
            return null;
        }

        session.RevokedAt = DateTimeOffset.UtcNow;

        var refreshedSession = new AuthSessionEntity
        {
            Token = Convert.ToHexString(Guid.NewGuid().ToByteArray()),
            RefreshToken = Convert.ToHexString(Guid.NewGuid().ToByteArray()),
            SessionType = session.SessionType,
            SubjectId = session.SubjectId,
            SubjectMode = session.SubjectMode,
            Role = session.Role,
            CreatedAt = DateTimeOffset.UtcNow,
            ExpiresAt = DateTimeOffset.UtcNow.Add(
                sessionType == "admin" ? AdminSessionLifetime : ClientSessionLifetime),
            RefreshExpiresAt = DateTimeOffset.UtcNow.Add(
                sessionType == "admin" ? AdminRefreshLifetime : ClientRefreshLifetime),
        };

        dbContext.AuthSessions.Add(refreshedSession);
        await dbContext.SaveChangesAsync();
        return refreshedSession;
    }

    private async Task<bool> IsSessionSubjectStillValidAsync(AuthSessionEntity session)
    {
        if (session.SessionType.Equals("admin", StringComparison.OrdinalIgnoreCase))
        {
            return await dbContext.AdminUsers.AsNoTracking().AnyAsync(item => item.Id == session.SubjectId);
        }

        if (!session.SessionType.Equals("client", StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        if (session.SubjectMode.Equals("local", StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }

        if (session.SubjectMode.Equals("personal", StringComparison.OrdinalIgnoreCase))
        {
            return await dbContext.PersonalAccounts.AsNoTracking().AnyAsync(item => item.Id == session.SubjectId);
        }

        if (session.SubjectMode.Equals("enterpriseSubAccount", StringComparison.OrdinalIgnoreCase))
        {
            return await dbContext.SubAccounts.AsNoTracking().AnyAsync(
                item => item.Id == session.SubjectId && item.Enabled);
        }

        return false;
    }

    private async Task RevokeClientSessionsAsync(string subjectMode, params string[] subjectIds)
    {
        var effectiveSubjectIds = subjectIds
            .Where(item => !string.IsNullOrWhiteSpace(item))
            .Distinct(StringComparer.Ordinal)
            .ToArray();

        if (effectiveSubjectIds.Length == 0)
        {
            return;
        }

        var sessions = await dbContext.AuthSessions
            .Where(item =>
                item.SessionType == "client" &&
                item.SubjectMode == subjectMode &&
                item.RevokedAt == null &&
                effectiveSubjectIds.Contains(item.SubjectId))
            .ToListAsync();

        if (sessions.Count == 0)
        {
            return;
        }

        var revokedAt = DateTimeOffset.UtcNow;
        foreach (var session in sessions)
        {
            session.RevokedAt = revokedAt;
        }

        await dbContext.SaveChangesAsync();
    }

    private async Task DeleteClientSyncStatesAsync(string mode, params string[] accountKeys)
    {
        var effectiveAccountKeys = accountKeys
            .Where(item => !string.IsNullOrWhiteSpace(item))
            .Distinct(StringComparer.Ordinal)
            .ToArray();

        if (effectiveAccountKeys.Length == 0)
        {
            return;
        }

        var states = await dbContext.ClientSyncStates
            .Where(item => item.Mode == mode && effectiveAccountKeys.Contains(item.AccountKey))
            .ToListAsync();

        if (states.Count == 0)
        {
            return;
        }

        dbContext.ClientSyncStates.RemoveRange(states);
        await dbContext.SaveChangesAsync();
    }

    private static ClientAiEndpointConfig BuildCustomEndpoint(ClientAccountSyncStateEntity state) => new()
    {
        UseCustomEndpoint = state.UseCustomEndpoint && (HasConfiguredCustomEndpoint(state) || HasCustomEndpointDraft(state)),
        EndpointName = state.EndpointName,
        Provider = state.Provider,
        BaseUrl = state.BaseUrl,
        ApiKey = state.ApiKey,
        ModelName = state.ModelName,
    };

    private static bool HasCustomEndpointDraft(ClientAccountSyncStateEntity state) =>
        !string.IsNullOrWhiteSpace(state.EndpointName) ||
        !string.IsNullOrWhiteSpace(state.BaseUrl) ||
        !string.IsNullOrWhiteSpace(state.ApiKey) ||
        !string.IsNullOrWhiteSpace(state.ModelName);

    private static bool HasConfiguredCustomEndpoint(ClientAccountSyncStateEntity state) =>
        !string.IsNullOrWhiteSpace(state.BaseUrl) &&
        !string.IsNullOrWhiteSpace(state.ApiKey) &&
        !string.IsNullOrWhiteSpace(state.ModelName);

    public async Task<AdminDashboardSnapshot> GetSnapshotAsync()
    {
        var enterprises = await dbContext.Enterprises.AsNoTracking().ToListAsync();
        var subAccounts = await dbContext.SubAccounts.AsNoTracking().ToListAsync();
        var personalAccounts = await dbContext.PersonalAccounts.AsNoTracking().ToListAsync();
        var enterpriseSubscriptions = await dbContext.EnterpriseSubscriptions.AsNoTracking().ToListAsync();
        var personalSubscriptions = await dbContext.PersonalSubscriptions.AsNoTracking().ToListAsync();
        var invoices = (await dbContext.BillingInvoices.AsNoTracking().ToListAsync())
            .OrderByDescending(item => item.CreatedAt)
            .ToList();
        var invoiceLineItems = await dbContext.BillingInvoiceLineItems.AsNoTracking().ToListAsync();
        var paymentTransactions = (await dbContext.PaymentTransactions.AsNoTracking().ToListAsync())
            .OrderByDescending(item => item.CreatedAt)
            .ToList();
        var paymentProviders = await dbContext.PaymentProviderConfigs.AsNoTracking().ToListAsync();
        var usageRecords = (await dbContext.AiUsageRecords.AsNoTracking().ToListAsync())
            .OrderByDescending(item => item.CreatedAt)
            .ToList();
        var usagePricing = await dbContext.AiUsagePricing.AsNoTracking().ToListAsync();
        var plans = await dbContext.AiSubscriptionPlans.AsNoTracking().ToListAsync();
        var assets = await dbContext.Assets.AsNoTracking().ToListAsync();
        var endpointSync = await dbContext.AiEndpoints.AsNoTracking().FirstAsync();

        var planMap = plans.ToDictionary(item => item.Code, item => item, StringComparer.OrdinalIgnoreCase);
        var activeSubCounts = subAccounts
            .Where(item => item.Enabled)
            .GroupBy(item => item.EnterpriseId)
            .ToDictionary(group => group.Key, group => group.Count(), StringComparer.OrdinalIgnoreCase);
        var enterpriseSubMap = enterpriseSubscriptions.ToDictionary(item => item.EnterpriseId, item => item, StringComparer.OrdinalIgnoreCase);
        var personalSubMap = personalSubscriptions.ToDictionary(item => item.AccountId, item => item, StringComparer.OrdinalIgnoreCase);

        return new AdminDashboardSnapshot
        {
            Enterprises = enterprises
                .Select(item => Map(
                    item,
                    enterpriseSubMap.GetValueOrDefault(item.Id),
                    planMap.GetValueOrDefault(enterpriseSubMap.GetValueOrDefault(item.Id)?.PlanCode ?? item.SubscriptionPlan),
                    activeSubCounts.GetValueOrDefault(item.Id)))
                .ToList(),
            SubAccounts = subAccounts.Select(Map).ToList(),
            PersonalAccounts = personalAccounts
                .Select(item => Map(
                    item,
                    personalSubMap.GetValueOrDefault(item.Id),
                    planMap.GetValueOrDefault(personalSubMap.GetValueOrDefault(item.Id)?.PlanCode ?? item.PlanName)))
                .ToList(),
            Assets = assets.Select(Map).ToList(),
            SubscriptionPlans = plans.Select(Map).OrderBy(item => item.Scope).ThenBy(item => item.Code).ToList(),
            EnterpriseSubscriptions = enterpriseSubscriptions
                .Select(item => Map(
                    item,
                    planMap.GetValueOrDefault(item.PlanCode),
                    activeSubCounts.GetValueOrDefault(item.EnterpriseId)))
                .ToList(),
            PersonalSubscriptions = personalSubscriptions
                .Select(item => Map(item, planMap.GetValueOrDefault(item.PlanCode)))
                .ToList(),
            AiUsagePricing = usagePricing.Select(Map).ToList(),
            PaymentProviders = paymentProviders.Select(Map).ToList(),
            Billing = BuildBillingOverview(invoices, invoiceLineItems, paymentTransactions),
            AiUsage = await BuildAiUsageSummaryAsync(usageRecords, usagePricing),
            AiSubscription = await BuildGlobalAiSubscriptionAsync(),
            EndpointSync = Map(endpointSync),
        };
    }

    public async Task<EnterpriseSubAccountSummary> UpdateSubAccountAssetsAsync(string subAccountId, IEnumerable<string> assetIds)
    {
        var subAccount = await dbContext.SubAccounts.FirstOrDefaultAsync(item => item.Id == subAccountId)
            ?? throw new KeyNotFoundException($"Sub account '{subAccountId}' was not found.");

        var validAssetIds = await dbContext.Assets
            .Where(asset => assetIds.Contains(asset.Id))
            .Select(asset => asset.Id)
            .Distinct()
            .ToListAsync();

        subAccount.AssetIdsJson = JsonSerializer.Serialize(validAssetIds);
        subAccount.UpdatedAt = DateTimeOffset.UtcNow;
        await dbContext.SaveChangesAsync();
        return Map(subAccount);
    }

    public async Task<AiSubscriptionOverview> UpdateAiSubscriptionAsync(UpdateAiSubscriptionRequest request)
    {
        var entity = await dbContext.AiSubscriptions.FirstAsync();
        entity.ServiceMode = request.ServiceMode;
        entity.PlanName = request.PlanName;
        entity.Status = request.Status.ToString().ToLowerInvariant();
        entity.Seats = request.Seats;
        entity.AllowCustomEndpoint = request.AllowCustomEndpoint;
        entity.SyncCustomEndpoint = request.SyncCustomEndpoint;
        entity.RenewAt = request.RenewAt;
        await dbContext.SaveChangesAsync();
        return await BuildGlobalAiSubscriptionAsync();
    }

    public async Task<AiSubscriptionPlanSummary> UpsertSubscriptionPlanAsync(UpsertSubscriptionPlanRequest request)
    {
        var entity = await dbContext.AiSubscriptionPlans.FirstOrDefaultAsync(item => item.Code == request.Code);
        if (entity is null)
        {
            entity = new AiSubscriptionPlanEntity { Code = request.Code };
            dbContext.AiSubscriptionPlans.Add(entity);
        }

        entity.DisplayName = request.DisplayName;
        entity.Scope = request.Scope;
        entity.PricePerSeat = request.PricePerSeat;
        entity.Currency = request.Currency;
        entity.AllowCustomEndpoint = request.AllowCustomEndpoint;
        entity.IsActive = request.IsActive;
        entity.Description = request.Description;
        entity.UpdatedAt = DateTimeOffset.UtcNow;
        await dbContext.SaveChangesAsync();
        return Map(entity);
    }

    public async Task<AiUsagePricingSummary> UpsertAiUsagePricingAsync(UpsertAiUsagePricingRequest request)
    {
        var entity = await dbContext.AiUsagePricing.FirstOrDefaultAsync(item => item.Id == request.Id);
        if (entity is null)
        {
            entity = new AiUsagePricingEntity { Id = request.Id };
            dbContext.AiUsagePricing.Add(entity);
        }

        entity.Provider = request.Provider;
        entity.ModelName = request.ModelName;
        entity.PromptTokenRatePerMillion = request.PromptTokenRatePerMillion;
        entity.CompletionTokenRatePerMillion = request.CompletionTokenRatePerMillion;
        entity.Currency = request.Currency;
        entity.IsActive = request.IsActive;
        entity.UpdatedAt = DateTimeOffset.UtcNow;
        await dbContext.SaveChangesAsync();
        return Map(entity);
    }

    public async Task<EnterpriseSubscriptionSummary> UpsertEnterpriseSubscriptionAsync(UpsertEnterpriseSubscriptionRequest request)
    {
        var enterprise = await dbContext.Enterprises.FirstOrDefaultAsync(item => item.Id == request.EnterpriseId)
            ?? throw new KeyNotFoundException($"Enterprise '{request.EnterpriseId}' was not found.");
        var plan = await dbContext.AiSubscriptionPlans.AsNoTracking().FirstOrDefaultAsync(item => item.Code == request.PlanCode)
            ?? throw new KeyNotFoundException($"Subscription plan '{request.PlanCode}' was not found.");

        var entity = await dbContext.EnterpriseSubscriptions.FirstOrDefaultAsync(item => item.EnterpriseId == request.EnterpriseId);
        if (entity is null)
        {
            entity = new EnterpriseSubscriptionEntity { EnterpriseId = request.EnterpriseId };
            dbContext.EnterpriseSubscriptions.Add(entity);
        }

        var seatsAssigned = await dbContext.SubAccounts.CountAsync(
            item => item.EnterpriseId == request.EnterpriseId && item.Enabled);

        if (request.SeatsPurchased < seatsAssigned)
        {
            throw new InvalidOperationException(
                $"Enterprise '{request.EnterpriseId}' has {seatsAssigned} enabled sub accounts, which exceeds the requested seat count {request.SeatsPurchased}.");
        }

        entity.PlanCode = request.PlanCode;
        entity.Status = request.Status.ToString().ToLowerInvariant();
        entity.SeatsPurchased = request.SeatsPurchased;
        entity.SeatsAssigned = seatsAssigned;
        entity.RenewAt = request.RenewAt;
        entity.UpdatedAt = DateTimeOffset.UtcNow;

        enterprise.SeatCount = request.SeatsPurchased;
        enterprise.ActiveSubAccounts = seatsAssigned;
        enterprise.SubscriptionPlan = request.PlanCode;
        enterprise.SubscriptionStatus = request.Status.ToString().ToLowerInvariant();
        enterprise.RenewAt = request.RenewAt;

        await dbContext.SaveChangesAsync();
        await UpsertMonthlyInvoiceAsync(
            targetType: "enterprise",
            targetId: request.EnterpriseId,
            planCode: request.PlanCode,
            seatCount: request.SeatsPurchased,
            unitPrice: plan.PricePerSeat,
            currency: plan.Currency,
            status: request.Status == SubscriptionStatus.PastDue ? BillingInvoiceStatus.Overdue : BillingInvoiceStatus.Open);
        return Map(entity, plan, seatsAssigned);
    }

    public async Task<PersonalSubscriptionSummary> UpsertPersonalSubscriptionAsync(UpsertPersonalSubscriptionRequest request)
    {
        var account = await dbContext.PersonalAccounts.FirstOrDefaultAsync(item => item.Id == request.AccountId)
            ?? throw new KeyNotFoundException($"Personal account '{request.AccountId}' was not found.");
        var plan = await dbContext.AiSubscriptionPlans.AsNoTracking().FirstOrDefaultAsync(item => item.Code == request.PlanCode)
            ?? throw new KeyNotFoundException($"Subscription plan '{request.PlanCode}' was not found.");

        var entity = await dbContext.PersonalSubscriptions.FirstOrDefaultAsync(item => item.AccountId == request.AccountId);
        if (entity is null)
        {
            entity = new PersonalSubscriptionEntity { AccountId = request.AccountId };
            dbContext.PersonalSubscriptions.Add(entity);
        }

        entity.PlanCode = request.PlanCode;
        entity.Status = request.Status.ToString().ToLowerInvariant();
        entity.RenewAt = request.RenewAt;
        entity.UpdatedAt = DateTimeOffset.UtcNow;

        account.SubscriptionStatus = entity.Status;
        account.PlanName = request.PlanCode;
        account.CustomEndpointEnabled = plan.AllowCustomEndpoint;
        account.UpdatedAt = DateTimeOffset.UtcNow;

        await dbContext.SaveChangesAsync();
        await UpsertMonthlyInvoiceAsync(
            targetType: "personal",
            targetId: request.AccountId,
            planCode: request.PlanCode,
            seatCount: 1,
            unitPrice: plan.PricePerSeat,
            currency: plan.Currency,
            status: request.Status == SubscriptionStatus.PastDue ? BillingInvoiceStatus.Overdue : BillingInvoiceStatus.Open);
        return Map(entity, plan);
    }

    public async Task<BillingInvoiceSummary> UpdateBillingInvoiceAsync(string invoiceId, UpdateBillingInvoiceRequest request)
    {
        var entity = await dbContext.BillingInvoices.FirstOrDefaultAsync(item => item.Id == invoiceId)
            ?? throw new KeyNotFoundException($"Billing invoice '{invoiceId}' was not found.");

        var netPaidAmount = await CalculateNetPaidAmountAsync(invoiceId);
        if (request.Status == BillingInvoiceStatus.Paid && netPaidAmount + 0.0001d < entity.TotalAmount)
        {
            throw new InvalidOperationException(
                $"Billing invoice '{invoiceId}' cannot be marked as paid until recorded payments cover the invoice total.");
        }

        if (request.Status == BillingInvoiceStatus.Voided && netPaidAmount > 0.0001d)
        {
            throw new InvalidOperationException(
                $"Billing invoice '{invoiceId}' cannot be voided while it still has recorded net payments.");
        }

        entity.Status = request.Status == BillingInvoiceStatus.Voided
            ? BillingInvoiceStatus.Voided.ToString().ToLowerInvariant()
            : DeriveInvoiceStatus(entity, netPaidAmount, preserveVoidedIfUnpaid: false)
                .ToString()
                .ToLowerInvariant();
        entity.UpdatedAt = DateTimeOffset.UtcNow;
        await dbContext.SaveChangesAsync();
        var lineItems = await dbContext.BillingInvoiceLineItems.AsNoTracking()
            .Where(item => item.InvoiceId == entity.Id)
            .ToListAsync();
        var payments = await dbContext.PaymentTransactions.AsNoTracking()
            .Where(item => item.InvoiceId == entity.Id)
            .ToListAsync();
        return Map(entity, lineItems, payments);
    }

    public async Task<PaymentTransactionSummary> CreatePaymentTransactionAsync(CreatePaymentTransactionRequest request)
    {
        var invoice = await dbContext.BillingInvoices.FirstOrDefaultAsync(item => item.Id == request.InvoiceId)
            ?? throw new KeyNotFoundException($"Billing invoice '{request.InvoiceId}' was not found.");

        if (request.Amount <= 0)
        {
            throw new InvalidOperationException("Payment amount must be greater than zero.");
        }

        if (string.Equals(invoice.Status, BillingInvoiceStatus.Voided.ToString(), StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidOperationException(
                $"Billing invoice '{request.InvoiceId}' is voided and cannot accept payments.");
        }

        if (!string.IsNullOrWhiteSpace(request.ExternalReference))
        {
            var existing = await dbContext.PaymentTransactions.FirstOrDefaultAsync(
                item => item.ProviderKey == request.ProviderKey && item.ExternalReference == request.ExternalReference);
            if (existing is not null)
            {
                if (!string.Equals(existing.InvoiceId, invoice.Id, StringComparison.Ordinal))
                {
                    throw new InvalidOperationException(
                        $"External reference '{request.ExternalReference}' is already attached to another invoice.");
                }

                existing.Amount = request.Amount;
                existing.Currency = request.Currency;
                existing.PaymentMethod = request.PaymentMethod;
                existing.Status = request.Status;
                existing.Note = request.Note;
                existing.PaidAt = ShouldRecordPaidAt(request.Status)
                    ? request.PaidAt ?? existing.PaidAt ?? DateTimeOffset.UtcNow
                    : null;
                existing.UpdatedAt = DateTimeOffset.UtcNow;

                await dbContext.SaveChangesAsync();
                await RefreshInvoicePaymentStatusAsync(invoice.Id);
                return Map(existing);
            }
        }

        var transaction = new PaymentTransactionEntity
        {
            Id = Guid.NewGuid().ToString("N"),
            InvoiceId = invoice.Id,
            TargetType = invoice.TargetType,
            TargetId = invoice.TargetId,
            ProviderKey = request.ProviderKey,
            Amount = request.Amount,
            Currency = request.Currency,
            PaymentMethod = request.PaymentMethod,
            Status = request.Status,
            ExternalReference = request.ExternalReference,
            Note = request.Note,
            CheckoutUrl = string.Empty,
            ExpiresAt = null,
            PaidAt = ShouldRecordPaidAt(request.Status)
                ? request.PaidAt ?? DateTimeOffset.UtcNow
                : null,
            CreatedAt = DateTimeOffset.UtcNow,
            UpdatedAt = DateTimeOffset.UtcNow,
        };

        dbContext.PaymentTransactions.Add(transaction);
        await dbContext.SaveChangesAsync();

        await RefreshInvoicePaymentStatusAsync(invoice.Id);
        return Map(transaction);
    }

    public async Task<PaymentTransactionSummary> CreateCheckoutSessionAsync(CreateCheckoutSessionRequest request)
    {
        var invoice = await dbContext.BillingInvoices.FirstOrDefaultAsync(item => item.Id == request.InvoiceId)
            ?? throw new KeyNotFoundException($"Billing invoice '{request.InvoiceId}' was not found.");
        var provider = await dbContext.PaymentProviderConfigs.AsNoTracking()
            .FirstOrDefaultAsync(item => item.ProviderKey == request.ProviderKey && item.Enabled)
            ?? throw new KeyNotFoundException($"Payment provider '{request.ProviderKey}' was not found.");

        if (string.Equals(invoice.Status, BillingInvoiceStatus.Voided.ToString(), StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidOperationException(
                $"Billing invoice '{request.InvoiceId}' is voided and cannot create a checkout session.");
        }

        var outstandingAmount = invoice.TotalAmount - await CalculateNetPaidAmountAsync(invoice.Id);
        if (outstandingAmount <= 0.0001d)
        {
            throw new InvalidOperationException(
                $"Billing invoice '{request.InvoiceId}' is already fully paid.");
        }

        var now = DateTimeOffset.UtcNow;
        var pendingCheckoutCandidates = await dbContext.PaymentTransactions.AsNoTracking()
            .Where(item =>
                item.InvoiceId == invoice.Id &&
                item.ProviderKey == provider.ProviderKey &&
                item.Status == "pending")
            .ToListAsync();
        var existingPendingCheckout = pendingCheckoutCandidates
            .OrderByDescending(item => item.CreatedAt.UtcDateTime)
            .FirstOrDefault(item =>
            !string.IsNullOrWhiteSpace(item.CheckoutUrl) &&
            (!item.ExpiresAt.HasValue || item.ExpiresAt > now));
        if (existingPendingCheckout is not null)
        {
            return Map(existingPendingCheckout);
        }

        var externalReference = $"checkout-{Guid.NewGuid():N}";
        string checkoutUrl;

        if (provider.ProviderType.Equals("stripe", StringComparison.OrdinalIgnoreCase))
        {
            var stripeConfig = ParseStripeLikeProviderConfig(provider.MetadataJson);
            if (string.IsNullOrWhiteSpace(stripeConfig.SecretApiKey))
            {
                throw new InvalidOperationException("Stripe-like provider is missing secretApiKey.");
            }

            var successUrl = string.IsNullOrWhiteSpace(request.ReturnUrl)
                ? stripeConfig.SuccessUrl
                : request.ReturnUrl;
            var cancelUrl = string.IsNullOrWhiteSpace(request.CancelUrl)
                ? stripeConfig.CancelUrl
                : request.CancelUrl;

            var lineItems = await dbContext.BillingInvoiceLineItems.AsNoTracking()
                .Where(item => item.InvoiceId == invoice.Id)
                .ToListAsync();

            var form = new List<KeyValuePair<string, string>>
            {
                new("mode", "payment"),
                new("success_url", successUrl),
                new("cancel_url", cancelUrl),
                new("client_reference_id", invoice.Id),
                new("metadata[invoice_id]", invoice.Id),
                new("metadata[target_id]", invoice.TargetId),
            };

            for (var index = 0; index < lineItems.Count; index++)
            {
                var lineItem = lineItems[index];
                var quantity = Math.Max(lineItem.Quantity, 1);
                var unitAmountCents = Math.Max(
                    1,
                    (int)Math.Round((lineItem.Amount / quantity) * 100, MidpointRounding.AwayFromZero));

                form.Add(new($"line_items[{index}][price_data][currency]", invoice.Currency.ToLowerInvariant()));
                form.Add(new($"line_items[{index}][price_data][unit_amount]", unitAmountCents.ToString(CultureInfo.InvariantCulture)));
                form.Add(new($"line_items[{index}][price_data][product_data][name]", lineItem.Description));
                form.Add(new($"line_items[{index}][quantity]", quantity.ToString(CultureInfo.InvariantCulture)));
            }

            var client = httpClientFactory.CreateClient();
            using var createRequest = new HttpRequestMessage(HttpMethod.Post, $"{stripeConfig.ApiBaseUrl.TrimEnd('/')}/v1/checkout/sessions");
            createRequest.Headers.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", stripeConfig.SecretApiKey);
            createRequest.Headers.TryAddWithoutValidation("Stripe-Version", stripeConfig.StripeApiVersion);
            createRequest.Content = new FormUrlEncodedContent(form);
            using var createResponse = await client.SendAsync(createRequest);
            var responseBody = await createResponse.Content.ReadAsStringAsync();
            if (!createResponse.IsSuccessStatusCode)
            {
                throw new InvalidOperationException($"Stripe-like checkout creation failed: {responseBody}");
            }

            using var checkoutDocument = JsonDocument.Parse(responseBody);
            externalReference = checkoutDocument.RootElement.TryGetProperty("id", out var sessionId)
                ? sessionId.GetString() ?? externalReference
                : externalReference;
            checkoutUrl = checkoutDocument.RootElement.TryGetProperty("url", out var sessionUrl)
                ? sessionUrl.GetString() ?? string.Empty
                : string.Empty;
            if (string.IsNullOrWhiteSpace(checkoutUrl))
            {
                throw new InvalidOperationException("Stripe-like checkout response did not include a URL.");
            }
        }
        else
        {
            var metadata = JsonDocument.Parse(provider.MetadataJson);
            var checkoutBaseUrl = metadata.RootElement.TryGetProperty("checkoutBaseUrl", out var baseUrl)
                ? baseUrl.GetString() ?? "https://payments.example.com/checkout"
                : "https://payments.example.com/checkout";
            checkoutUrl = $"{checkoutBaseUrl.TrimEnd('/')}/{externalReference}?invoiceId={Uri.EscapeDataString(invoice.Id)}";
        }

        var transaction = new PaymentTransactionEntity
        {
            Id = Guid.NewGuid().ToString("N"),
            InvoiceId = invoice.Id,
            TargetType = invoice.TargetType,
            TargetId = invoice.TargetId,
            ProviderKey = provider.ProviderKey,
            Amount = outstandingAmount > 0 ? outstandingAmount : invoice.TotalAmount,
            Currency = invoice.Currency,
            PaymentMethod = provider.ProviderType,
            Status = "pending",
            ExternalReference = externalReference,
            Note = $"Return={request.ReturnUrl}; Cancel={request.CancelUrl}",
            CheckoutUrl = checkoutUrl,
            ExpiresAt = DateTimeOffset.UtcNow.AddHours(2),
            PaidAt = null,
            CreatedAt = DateTimeOffset.UtcNow,
            UpdatedAt = DateTimeOffset.UtcNow,
        };

        dbContext.PaymentTransactions.Add(transaction);
        await dbContext.SaveChangesAsync();
        await RefreshInvoicePaymentStatusAsync(invoice.Id);
        return Map(transaction);
    }

    public async Task<PaymentProviderConfigSummary> UpsertPaymentProviderConfigAsync(UpsertPaymentProviderConfigRequest request)
    {
        var entity = await dbContext.PaymentProviderConfigs.FirstOrDefaultAsync(item => item.ProviderKey == request.ProviderKey);
        if (entity is null)
        {
            entity = new PaymentProviderConfigEntity { ProviderKey = request.ProviderKey };
            dbContext.PaymentProviderConfigs.Add(entity);
        }

        entity.DisplayName = request.DisplayName;
        entity.ProviderType = request.ProviderType;
        entity.WebhookSecret = request.WebhookSecret;
        entity.Enabled = request.Enabled;
        entity.MetadataJson = BuildPaymentProviderMetadataJson(request);
        entity.UpdatedAt = DateTimeOffset.UtcNow;
        await dbContext.SaveChangesAsync();
        return Map(entity);
    }

    public async Task<PaymentTransactionSummary> HandlePaymentWebhookAsync(PaymentWebhookEventRequest request)
    {
        var provider = await dbContext.PaymentProviderConfigs.AsNoTracking()
            .FirstOrDefaultAsync(item => item.ProviderKey == request.ProviderKey && item.Enabled)
            ?? throw new KeyNotFoundException($"Payment provider '{request.ProviderKey}' was not found.");

        var expectedSecret = provider.WebhookSecret;
        if (!string.IsNullOrWhiteSpace(expectedSecret) &&
            !ValidateWebhookSecret(provider.ProviderType, expectedSecret, request, provider.MetadataJson))
        {
            throw new UnauthorizedAccessException("Webhook secret mismatch.");
        }

        var stripePayload = provider.ProviderType.Equals("stripe", StringComparison.OrdinalIgnoreCase)
            ? ParseStripeLikeWebhookPayload(request.PayloadJson)
            : null;
        var eventType = stripePayload?.EventType ?? request.EventType;
        var normalizedStatus = NormalizeWebhookStatus(provider.ProviderType, eventType, stripePayload?.Status ?? request.Status);
        var externalReference = stripePayload?.ExternalReference ?? request.ExternalReference;
        var invoiceId = stripePayload?.InvoiceId ?? request.InvoiceId;
        var amount = stripePayload?.Amount ?? request.Amount;
        var currency = stripePayload?.Currency ?? request.Currency;

        var transaction = await dbContext.PaymentTransactions.FirstOrDefaultAsync(
            item => item.ExternalReference == externalReference && item.ProviderKey == request.ProviderKey);

        if (transaction is null)
        {
            var invoice = await dbContext.BillingInvoices.FirstOrDefaultAsync(item => item.Id == invoiceId)
                ?? throw new KeyNotFoundException($"Billing invoice '{invoiceId}' was not found.");

            transaction = new PaymentTransactionEntity
            {
                Id = Guid.NewGuid().ToString("N"),
                InvoiceId = invoice.Id,
                TargetType = invoice.TargetType,
                TargetId = invoice.TargetId,
                ProviderKey = provider.ProviderKey,
                Amount = amount,
                Currency = currency,
                PaymentMethod = provider.ProviderType,
                Status = normalizedStatus,
                ExternalReference = externalReference,
                Note = AppendWebhookNote(request.Note, eventType),
                CheckoutUrl = string.Empty,
                ExpiresAt = null,
                PaidAt = ShouldRecordPaidAt(normalizedStatus) ? DateTimeOffset.UtcNow : null,
                CreatedAt = DateTimeOffset.UtcNow,
                UpdatedAt = DateTimeOffset.UtcNow,
            };
            dbContext.PaymentTransactions.Add(transaction);
        }
        else
        {
            transaction.Status = normalizedStatus;
            transaction.Amount = amount;
            transaction.Currency = currency;
            transaction.Note = AppendWebhookNote(request.Note, eventType);
            transaction.PaidAt = ShouldRecordPaidAt(normalizedStatus)
                ? transaction.PaidAt ?? DateTimeOffset.UtcNow
                : null;
            transaction.UpdatedAt = DateTimeOffset.UtcNow;
        }

        await dbContext.SaveChangesAsync();
        await RefreshInvoicePaymentStatusAsync(transaction.InvoiceId);
        return Map(transaction);
    }

    private static string NormalizeWebhookStatus(string providerType, string eventType, string fallbackStatus)
    {
        if (providerType.Equals("stripe", StringComparison.OrdinalIgnoreCase))
        {
            return eventType switch
            {
                "checkout.session.completed" => "completed",
                "checkout.session.expired" => "failed",
                "payment_intent.payment_failed" => "failed",
                "charge.refunded" => "refunded",
                _ => fallbackStatus,
            };
        }

        return fallbackStatus;
    }

    private static bool ValidateWebhookSecret(
        string providerType,
        string expectedSecret,
        PaymentWebhookEventRequest request,
        string metadataJson)
    {
        if (providerType.Equals("stripe", StringComparison.OrdinalIgnoreCase))
        {
            if (string.IsNullOrWhiteSpace(request.Signature))
            {
                return false;
            }

            var parts = request.Signature.Split(',', StringSplitOptions.RemoveEmptyEntries);
            string? timestamp = null;
            string? v1 = null;
            foreach (var part in parts)
            {
                if (part.StartsWith("t=", StringComparison.OrdinalIgnoreCase))
                {
                    timestamp = part[2..];
                }
                else if (part.StartsWith("v1=", StringComparison.OrdinalIgnoreCase))
                {
                    v1 = part[3..];
                }
            }

            if (string.IsNullOrWhiteSpace(timestamp) || string.IsNullOrWhiteSpace(v1))
            {
                return false;
            }

            if (!long.TryParse(timestamp, out var unixTimestamp))
            {
                return false;
            }

            var config = ParseStripeLikeProviderConfig(metadataJson);
            var nowUnix = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            if (Math.Abs(nowUnix - unixTimestamp) > config.WebhookToleranceSeconds)
            {
                return false;
            }

            var signedPayload = $"{timestamp}.{request.PayloadJson ?? string.Empty}";
            using var hmac = new HMACSHA256(Encoding.UTF8.GetBytes(expectedSecret));
            var signatureBytes = hmac.ComputeHash(Encoding.UTF8.GetBytes(signedPayload));
            var computed = Convert.ToHexString(signatureBytes).ToLowerInvariant();
            return string.Equals(computed, v1, StringComparison.OrdinalIgnoreCase);
        }

        return string.Equals(request.WebhookSecret, expectedSecret, StringComparison.Ordinal);
    }

    private static StripeLikeProviderConfig ParseStripeLikeProviderConfig(string metadataJson)
    {
        using var metadata = JsonDocument.Parse(string.IsNullOrWhiteSpace(metadataJson) ? "{}" : metadataJson);
        return new StripeLikeProviderConfig
        {
            CheckoutBaseUrl = metadata.RootElement.TryGetProperty("checkoutBaseUrl", out var checkoutBaseUrl)
                ? checkoutBaseUrl.GetString() ?? "https://payments.example.com/stripe-checkout"
                : "https://payments.example.com/stripe-checkout",
            WebhookMode = metadata.RootElement.TryGetProperty("webhookMode", out var webhookMode)
                ? webhookMode.GetString() ?? "stripe-like"
                : "stripe-like",
            ApiBaseUrl = metadata.RootElement.TryGetProperty("apiBaseUrl", out var apiBaseUrl)
                ? apiBaseUrl.GetString() ?? "https://api.stripe.com"
                : "https://api.stripe.com",
            SecretApiKey = metadata.RootElement.TryGetProperty("secretApiKey", out var secretApiKey)
                ? secretApiKey.GetString() ?? string.Empty
                : string.Empty,
            StripeApiVersion = metadata.RootElement.TryGetProperty("stripeApiVersion", out var stripeApiVersion)
                ? stripeApiVersion.GetString() ?? "2024-06-20"
                : "2024-06-20",
            WebhookToleranceSeconds = metadata.RootElement.TryGetProperty("webhookToleranceSeconds", out var tolerance)
                ? tolerance.GetInt32()
                : 300,
            SuccessUrl = metadata.RootElement.TryGetProperty("successUrl", out var successUrl)
                ? successUrl.GetString() ?? DefaultCheckoutReturnUrl
                : DefaultCheckoutReturnUrl,
            CancelUrl = metadata.RootElement.TryGetProperty("cancelUrl", out var cancelUrl)
                ? cancelUrl.GetString() ?? DefaultCheckoutCancelUrl
                : DefaultCheckoutCancelUrl,
        };
    }

    private static PaymentProviderConfigSummary BuildPaymentProviderSummary(PaymentProviderConfigEntity entity)
    {
        var metadataJson = string.IsNullOrWhiteSpace(entity.MetadataJson) ? "{}" : entity.MetadataJson;
        using var metadata = JsonDocument.Parse(metadataJson);
        var root = metadata.RootElement;
        return new PaymentProviderConfigSummary
        {
            ProviderKey = entity.ProviderKey,
            DisplayName = entity.DisplayName,
            ProviderType = entity.ProviderType,
            WebhookSecret = entity.WebhookSecret,
            Enabled = entity.Enabled,
            MetadataJson = metadataJson,
            CheckoutBaseUrl = root.TryGetProperty("checkoutBaseUrl", out var checkoutBaseUrl)
                ? checkoutBaseUrl.GetString() ?? string.Empty
                : string.Empty,
            WebhookMode = root.TryGetProperty("webhookMode", out var webhookMode)
                ? webhookMode.GetString() ?? "manual"
                : "manual",
            ApiBaseUrl = root.TryGetProperty("apiBaseUrl", out var apiBaseUrl)
                ? apiBaseUrl.GetString() ?? string.Empty
                : string.Empty,
            SecretApiKey = root.TryGetProperty("secretApiKey", out var secretApiKey)
                ? secretApiKey.GetString() ?? string.Empty
                : string.Empty,
            StripeApiVersion = root.TryGetProperty("stripeApiVersion", out var stripeApiVersion)
                ? stripeApiVersion.GetString() ?? string.Empty
                : string.Empty,
            WebhookToleranceSeconds = root.TryGetProperty("webhookToleranceSeconds", out var webhookToleranceSeconds)
                ? webhookToleranceSeconds.GetInt32()
                : 300,
            SuccessUrl = root.TryGetProperty("successUrl", out var successUrl)
                ? successUrl.GetString() ?? string.Empty
                : string.Empty,
            CancelUrl = root.TryGetProperty("cancelUrl", out var cancelUrl)
                ? cancelUrl.GetString() ?? string.Empty
                : string.Empty,
            UpdatedAt = entity.UpdatedAt,
        };
    }

    private static string BuildPaymentProviderMetadataJson(UpsertPaymentProviderConfigRequest request)
    {
        if (!string.IsNullOrWhiteSpace(request.CheckoutBaseUrl) ||
            !string.IsNullOrWhiteSpace(request.WebhookMode) ||
            !string.IsNullOrWhiteSpace(request.ApiBaseUrl) ||
            !string.IsNullOrWhiteSpace(request.SecretApiKey) ||
            !string.IsNullOrWhiteSpace(request.StripeApiVersion) ||
            request.WebhookToleranceSeconds.HasValue ||
            !string.IsNullOrWhiteSpace(request.SuccessUrl) ||
            !string.IsNullOrWhiteSpace(request.CancelUrl))
        {
            var payload = new Dictionary<string, object>();
            if (!string.IsNullOrWhiteSpace(request.CheckoutBaseUrl))
            {
                payload["checkoutBaseUrl"] = request.CheckoutBaseUrl;
            }

            if (!string.IsNullOrWhiteSpace(request.WebhookMode))
            {
                payload["webhookMode"] = request.WebhookMode;
            }

            if (!string.IsNullOrWhiteSpace(request.ApiBaseUrl))
            {
                payload["apiBaseUrl"] = request.ApiBaseUrl;
            }

            if (!string.IsNullOrWhiteSpace(request.SecretApiKey))
            {
                payload["secretApiKey"] = request.SecretApiKey;
            }

            if (!string.IsNullOrWhiteSpace(request.StripeApiVersion))
            {
                payload["stripeApiVersion"] = request.StripeApiVersion;
            }

            if (request.WebhookToleranceSeconds.HasValue)
            {
                payload["webhookToleranceSeconds"] = request.WebhookToleranceSeconds.Value.ToString(CultureInfo.InvariantCulture);
            }

            if (!string.IsNullOrWhiteSpace(request.SuccessUrl))
            {
                payload["successUrl"] = request.SuccessUrl;
            }

            if (!string.IsNullOrWhiteSpace(request.CancelUrl))
            {
                payload["cancelUrl"] = request.CancelUrl;
            }

            return JsonSerializer.Serialize(payload);
        }

        return string.IsNullOrWhiteSpace(request.MetadataJson) ? "{}" : request.MetadataJson;
    }

    private static StripeLikeWebhookPayload? ParseStripeLikeWebhookPayload(string payloadJson)
    {
        if (string.IsNullOrWhiteSpace(payloadJson))
        {
            return null;
        }

        using var document = JsonDocument.Parse(payloadJson);
        var root = document.RootElement;
        var eventType = root.TryGetProperty("type", out var typeElement)
            ? typeElement.GetString() ?? string.Empty
            : string.Empty;
        if (!root.TryGetProperty("data", out var dataElement) ||
            !dataElement.TryGetProperty("object", out var objectElement))
        {
            return null;
        }

        var externalReference = objectElement.TryGetProperty("id", out var idElement)
            ? idElement.GetString() ?? string.Empty
            : string.Empty;
        var invoiceId = objectElement.TryGetProperty("client_reference_id", out var clientReference)
            ? clientReference.GetString() ?? string.Empty
            : objectElement.TryGetProperty("metadata", out var metadata) && metadata.TryGetProperty("invoice_id", out var invoiceMeta)
                ? invoiceMeta.GetString() ?? string.Empty
                : string.Empty;
        var amount = objectElement.TryGetProperty("amount_total", out var amountTotal)
            ? amountTotal.GetDouble() / 100d
            : 0;
        var currency = objectElement.TryGetProperty("currency", out var currencyElement)
            ? (currencyElement.GetString() ?? "USD").ToUpperInvariant()
            : "USD";
        var paymentStatus = objectElement.TryGetProperty("payment_status", out var paymentStatusElement)
            ? paymentStatusElement.GetString() ?? string.Empty
            : string.Empty;

        return new StripeLikeWebhookPayload
        {
            EventType = eventType,
            ExternalReference = externalReference,
            InvoiceId = invoiceId,
            Amount = amount,
            Currency = currency,
            Status = paymentStatus,
        };
    }

    private static string AppendWebhookNote(string note, string eventType)
    {
        if (string.IsNullOrWhiteSpace(eventType))
        {
            return note;
        }

        return string.IsNullOrWhiteSpace(note)
            ? $"event={eventType}"
            : $"{note} | event={eventType}";
    }

    public async Task<GenerateBillingCycleResponse> GenerateCurrentBillingCycleAsync()
    {
        var plans = await dbContext.AiSubscriptionPlans.AsNoTracking().ToDictionaryAsync(item => item.Code, StringComparer.OrdinalIgnoreCase);
        var enterpriseSubscriptions = await dbContext.EnterpriseSubscriptions.AsNoTracking().ToListAsync();
        var personalSubscriptions = await dbContext.PersonalSubscriptions.AsNoTracking().ToListAsync();
        var generated = 0;

        foreach (var subscription in enterpriseSubscriptions)
        {
            if (!plans.TryGetValue(subscription.PlanCode, out var plan))
            {
                continue;
            }

            if (await UpsertMonthlyInvoiceAsync(
                targetType: "enterprise",
                targetId: subscription.EnterpriseId,
                planCode: subscription.PlanCode,
                seatCount: subscription.SeatsPurchased,
                unitPrice: plan.PricePerSeat,
                currency: plan.Currency,
                status: subscription.Status.Equals("pastdue", StringComparison.OrdinalIgnoreCase)
                    ? BillingInvoiceStatus.Overdue
                    : BillingInvoiceStatus.Open))
            {
                generated++;
            }
        }

        foreach (var subscription in personalSubscriptions)
        {
            if (!plans.TryGetValue(subscription.PlanCode, out var plan))
            {
                continue;
            }

            if (await UpsertMonthlyInvoiceAsync(
                targetType: "personal",
                targetId: subscription.AccountId,
                planCode: subscription.PlanCode,
                seatCount: 1,
                unitPrice: plan.PricePerSeat,
                currency: plan.Currency,
                status: subscription.Status.Equals("pastdue", StringComparison.OrdinalIgnoreCase)
                    ? BillingInvoiceStatus.Overdue
                    : BillingInvoiceStatus.Open))
            {
                generated++;
            }
        }

        var invoices = (await dbContext.BillingInvoices.AsNoTracking().ToListAsync())
            .OrderByDescending(item => item.CreatedAt)
            .ToList();
        var invoiceLineItems = await dbContext.BillingInvoiceLineItems.AsNoTracking().ToListAsync();
        var paymentTransactions = await dbContext.PaymentTransactions.AsNoTracking().ToListAsync();
        return new GenerateBillingCycleResponse
        {
            Billing = BuildBillingOverview(invoices, invoiceLineItems, paymentTransactions),
            GeneratedInvoices = generated,
        };
    }

    public async Task<ClientAiRuntimeResponse> GetClientAiRuntimeAsync(string accessToken)
    {
        var session = await ValidateSessionAsync(accessToken, "client");
        if (session is null)
        {
            throw new UnauthorizedAccessException("Client session expired or invalid.");
        }

        var state = await UpsertClientStateAsync(session.SubjectMode, session.SubjectId);
        var endpoint = await dbContext.AiEndpoints.AsNoTracking().FirstAsync();
        var runtimeState = await EvaluateScopedSubscriptionAccessAsync(
            session.SubjectMode,
            session.SubjectId,
            string.Empty,
            string.Empty);
        var hasCustomEndpoint = state.UseCustomEndpoint && HasConfiguredCustomEndpoint(state);
        var useCustomEndpoint = hasCustomEndpoint;
        var missingCustomEndpointConfig = state.UseCustomEndpoint && HasCustomEndpointDraft(state) && !hasCustomEndpoint;

        return new ClientAiRuntimeResponse
        {
            Enabled = runtimeState.Enabled && (useCustomEndpoint || endpoint.SyncToClients),
            Reason = runtimeState.Enabled
                ? (useCustomEndpoint || endpoint.SyncToClients
                    ? string.Empty
                    : missingCustomEndpointConfig
                        ? "custom-endpoint-incomplete"
                        : "managed-endpoint-disabled")
                : runtimeState.Reason,
            Provider = useCustomEndpoint ? state.Provider : endpoint.Provider,
            BaseUrl = useCustomEndpoint ? state.BaseUrl : endpoint.BaseUrl,
            ModelName = useCustomEndpoint ? state.ModelName : endpoint.ModelName,
            ApiKey = useCustomEndpoint ? state.ApiKey : string.Empty,
            UsingManagedEndpoint = !useCustomEndpoint,
        };
    }

    public async Task<ClientSubscriptionSnapshot> GetClientSubscriptionSnapshotAsync(string accessToken)
    {
        var session = await ValidateSessionAsync(accessToken, "client");
        if (session is null)
        {
            throw new UnauthorizedAccessException("Client session expired or invalid.");
        }

        return await BuildClientSubscriptionSnapshotAsync(
            session.SubjectMode,
            session.SubjectId,
            string.Empty,
            string.Empty);
    }

    public async Task<PaymentTransactionSummary> CreateClientCheckoutSessionAsync(
        string accessToken,
        CreateCheckoutSessionRequest request)
    {
        var session = await ValidateSessionAsync(accessToken, "client");
        if (session is null)
        {
            throw new UnauthorizedAccessException("Client session expired or invalid.");
        }

        var invoice = await ResolveScopedInvoiceAsync(
            session.SubjectMode,
            session.SubjectId,
            string.Empty,
            string.Empty,
            request.InvoiceId);
        if (invoice is null)
        {
            throw new KeyNotFoundException($"Billing invoice '{request.InvoiceId}' was not found for the current account scope.");
        }

        return await CreateCheckoutSessionAsync(new CreateCheckoutSessionRequest
        {
            InvoiceId = invoice.Id,
            ProviderKey = request.ProviderKey,
            ReturnUrl = request.ReturnUrl,
            CancelUrl = request.CancelUrl,
        });
    }

    public async Task<(int StatusCode, string Content)> ProxyManagedOpenAiAsync(string sessionToken, string body)
    {
        var session = await ValidateSessionAsync(sessionToken, "client");
        if (session is null)
        {
            throw new UnauthorizedAccessException("Client session expired or invalid.");
        }

        var runtime = await GetClientAiRuntimeAsync(sessionToken);
        if (!runtime.Enabled)
        {
            throw new InvalidOperationException(runtime.Reason);
        }

        if (!runtime.UsingManagedEndpoint)
        {
            return await ProxyCustomOpenAiAsync(session, runtime, body);
        }

        var endpoint = await dbContext.AiEndpoints.AsNoTracking().FirstAsync();
        var client = httpClientFactory.CreateClient();
        var upstreamUrl = $"{endpoint.BaseUrl.TrimEnd('/')}/chat/completions";
        using var request = new HttpRequestMessage(HttpMethod.Post, upstreamUrl);
        request.Content = new StringContent(body, System.Text.Encoding.UTF8, "application/json");
        request.Headers.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", endpoint.ApiKey);
        request.Headers.TryAddWithoutValidation("x-coding-tool", "opencode");

        using var response = await client.SendAsync(request);
        var content = await response.Content.ReadAsStringAsync();
        await RecordAiUsageAsync(session, endpoint.Provider, endpoint.ModelName, true, content);
        return ((int)response.StatusCode, content);
    }

    private async Task<(int StatusCode, string Content)> ProxyCustomOpenAiAsync(
        AuthSessionEntity session,
        ClientAiRuntimeResponse runtime,
        string body)
    {
        var client = httpClientFactory.CreateClient();
        var upstreamUrl = $"{runtime.BaseUrl.TrimEnd('/')}/chat/completions";
        using var request = new HttpRequestMessage(HttpMethod.Post, upstreamUrl);
        request.Content = new StringContent(body, System.Text.Encoding.UTF8, "application/json");
        request.Headers.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", runtime.ApiKey);
        request.Headers.TryAddWithoutValidation("x-coding-tool", "opencode");

        using var response = await client.SendAsync(request);
        var content = await response.Content.ReadAsStringAsync();
        await RecordAiUsageAsync(session, runtime.Provider, runtime.ModelName, false, content);
        return ((int)response.StatusCode, content);
    }

    public async Task<(int StatusCode, string Content)> ProxyManagedAnthropicAsync(
        string sessionToken,
        string body,
        string anthropicVersion,
        string? anthropicBeta,
        string? codingToolHeader)
    {
        var session = await ValidateSessionAsync(sessionToken, "client");
        if (session is null)
        {
            throw new UnauthorizedAccessException("Client session expired or invalid.");
        }

        var runtime = await GetClientAiRuntimeAsync(sessionToken);
        if (!runtime.Enabled)
        {
            throw new InvalidOperationException(runtime.Reason);
        }

        if (!runtime.UsingManagedEndpoint)
        {
            return await ProxyCustomAnthropicAsync(session, runtime, body, anthropicVersion, anthropicBeta, codingToolHeader);
        }

        var endpoint = await dbContext.AiEndpoints.AsNoTracking().FirstAsync();
        var client = httpClientFactory.CreateClient();
        var baseUrl = endpoint.BaseUrl.TrimEnd('/');
        var upstreamUrl = baseUrl.EndsWith("/messages", StringComparison.OrdinalIgnoreCase)
            ? baseUrl
            : baseUrl.EndsWith("/v1", StringComparison.OrdinalIgnoreCase)
                ? $"{baseUrl}/messages"
                : $"{baseUrl}/v1/messages";

        using var request = new HttpRequestMessage(HttpMethod.Post, upstreamUrl);
        request.Content = new StringContent(body, System.Text.Encoding.UTF8, "application/json");
        request.Headers.TryAddWithoutValidation("x-api-key", endpoint.ApiKey);
        request.Headers.TryAddWithoutValidation("anthropic-version", anthropicVersion);
        if (!string.IsNullOrWhiteSpace(anthropicBeta))
        {
            request.Headers.TryAddWithoutValidation("anthropic-beta", anthropicBeta);
        }
        if (!string.IsNullOrWhiteSpace(codingToolHeader))
        {
            request.Headers.TryAddWithoutValidation("x-coding-tool", codingToolHeader);
        }

        using var response = await client.SendAsync(request);
        var content = await response.Content.ReadAsStringAsync();
        await RecordAiUsageAsync(session, endpoint.Provider, endpoint.ModelName, true, content);
        return ((int)response.StatusCode, content);
    }

    private async Task<(int StatusCode, string Content)> ProxyCustomAnthropicAsync(
        AuthSessionEntity session,
        ClientAiRuntimeResponse runtime,
        string body,
        string anthropicVersion,
        string? anthropicBeta,
        string? codingToolHeader)
    {
        var client = httpClientFactory.CreateClient();
        var baseUrl = runtime.BaseUrl.TrimEnd('/');
        var upstreamUrl = baseUrl.EndsWith("/messages", StringComparison.OrdinalIgnoreCase)
            ? baseUrl
            : baseUrl.EndsWith("/v1", StringComparison.OrdinalIgnoreCase)
                ? $"{baseUrl}/messages"
                : $"{baseUrl}/v1/messages";

        using var request = new HttpRequestMessage(HttpMethod.Post, upstreamUrl);
        request.Content = new StringContent(body, System.Text.Encoding.UTF8, "application/json");
        request.Headers.TryAddWithoutValidation("x-api-key", runtime.ApiKey);
        request.Headers.TryAddWithoutValidation("anthropic-version", anthropicVersion);
        if (!string.IsNullOrWhiteSpace(anthropicBeta))
        {
            request.Headers.TryAddWithoutValidation("anthropic-beta", anthropicBeta);
        }
        if (!string.IsNullOrWhiteSpace(codingToolHeader))
        {
            request.Headers.TryAddWithoutValidation("x-coding-tool", codingToolHeader);
        }

        using var response = await client.SendAsync(request);
        var content = await response.Content.ReadAsStringAsync();
        await RecordAiUsageAsync(session, runtime.Provider, runtime.ModelName, false, content);
        return ((int)response.StatusCode, content);
    }

    public async Task<AiEndpointSyncSettings> UpdateEndpointSyncAsync(UpdateEndpointSyncRequest request)
    {
        var entity = await dbContext.AiEndpoints.FirstAsync();
        entity.EndpointName = request.EndpointName;
        entity.Provider = request.Provider;
        entity.BaseUrl = request.BaseUrl;
        entity.ApiKey = request.ApiKey;
        entity.ModelName = request.ModelName;
        entity.SyncToClients = request.SyncToClients;
        entity.UpdatedAt = DateTimeOffset.UtcNow;
        await dbContext.SaveChangesAsync();
        return Map(entity);
    }

    public async Task<EnterpriseSummary> UpsertEnterpriseAsync(UpsertEnterpriseRequest request)
    {
        var entity = await dbContext.Enterprises.FirstOrDefaultAsync(item => item.Id == request.Id);
        if (entity is null)
        {
            entity = new EnterpriseEntity { Id = request.Id };
            dbContext.Enterprises.Add(entity);
        }

        entity.Name = request.Name;
        entity.SeatCount = request.SeatCount;
        entity.SubscriptionPlan = request.SubscriptionPlan;
        entity.SubscriptionStatus = request.SubscriptionStatus.ToString().ToLowerInvariant();
        entity.RenewAt = request.RenewAt;
        entity.ActiveSubAccounts = await dbContext.SubAccounts.CountAsync(item => item.EnterpriseId == entity.Id && item.Enabled);
        await dbContext.SaveChangesAsync();

        var plan = await dbContext.AiSubscriptionPlans.AsNoTracking().FirstOrDefaultAsync(item => item.Code == entity.SubscriptionPlan);
        var subscription = await dbContext.EnterpriseSubscriptions.AsNoTracking().FirstOrDefaultAsync(item => item.EnterpriseId == entity.Id);
        return Map(entity, subscription, plan, entity.ActiveSubAccounts);
    }

    public async Task DeleteEnterpriseAsync(string enterpriseId)
    {
        var entity = await dbContext.Enterprises.FirstOrDefaultAsync(item => item.Id == enterpriseId)
            ?? throw new KeyNotFoundException($"Enterprise '{enterpriseId}' was not found.");

        var subAccounts = await dbContext.SubAccounts
            .Where(item => item.EnterpriseId == enterpriseId)
            .ToListAsync();
        var subAccountIds = subAccounts.Select(item => item.Id).ToArray();

        var subscription = await dbContext.EnterpriseSubscriptions.FirstOrDefaultAsync(item => item.EnterpriseId == enterpriseId);
        if (subscription is not null)
        {
            dbContext.EnterpriseSubscriptions.Remove(subscription);
        }

        if (subAccounts.Count > 0)
        {
            dbContext.SubAccounts.RemoveRange(subAccounts);
        }

        dbContext.Enterprises.Remove(entity);
        await dbContext.SaveChangesAsync();

        await RevokeClientSessionsAsync("enterpriseSubAccount", subAccountIds);
        await DeleteClientSyncStatesAsync("enterpriseSubAccount", subAccountIds);
    }

    public async Task<EnterpriseSubAccountSummary> UpsertSubAccountAsync(UpsertSubAccountRequest request)
    {
        var entity = await dbContext.SubAccounts.FirstOrDefaultAsync(item => item.Id == request.Id);
        var previousEnterpriseId = entity?.EnterpriseId;
        var wasEnabled = entity?.Enabled ?? false;
        if (entity is null)
        {
            entity = new EnterpriseSubAccountEntity { Id = request.Id };
            dbContext.SubAccounts.Add(entity);
        }

        entity.EnterpriseId = request.EnterpriseId;
        entity.DisplayName = request.DisplayName;
        entity.Email = request.Email;
        entity.Secret = string.IsNullOrWhiteSpace(request.Secret)
            ? entity.Secret
            : EnsureHashedSecret(request.Secret);
        entity.Enabled = request.Enabled;
        entity.AssetIdsJson = JsonSerializer.Serialize(request.AssetIds);
        entity.UpdatedAt = DateTimeOffset.UtcNow;
        await dbContext.SaveChangesAsync();

        if (!entity.Enabled || (wasEnabled && !request.Enabled))
        {
            await RevokeClientSessionsAsync("enterpriseSubAccount", entity.Id);
        }

        if (!entity.Enabled)
        {
            await DeleteClientSyncStatesAsync("enterpriseSubAccount", entity.Id);
        }

        if (!string.IsNullOrWhiteSpace(previousEnterpriseId) &&
            !string.Equals(previousEnterpriseId, request.EnterpriseId, StringComparison.Ordinal))
        {
            await SyncEnterpriseSeatAssignmentAsync(previousEnterpriseId);
        }

        await SyncEnterpriseSeatAssignmentAsync(request.EnterpriseId);
        return Map(entity);
    }

    public async Task DeleteSubAccountAsync(string subAccountId)
    {
        var entity = await dbContext.SubAccounts.FirstOrDefaultAsync(item => item.Id == subAccountId)
            ?? throw new KeyNotFoundException($"Sub account '{subAccountId}' was not found.");

        var enterpriseId = entity.EnterpriseId;
        dbContext.SubAccounts.Remove(entity);
        await dbContext.SaveChangesAsync();

        await RevokeClientSessionsAsync("enterpriseSubAccount", subAccountId);
        await DeleteClientSyncStatesAsync("enterpriseSubAccount", subAccountId);
        await SyncEnterpriseSeatAssignmentAsync(enterpriseId);
    }

    public async Task<PersonalAccountSummary> UpsertPersonalAccountAsync(UpsertPersonalAccountRequest request)
    {
        var entity = await dbContext.PersonalAccounts.FirstOrDefaultAsync(item => item.Id == request.Id);
        if (entity is null)
        {
            entity = new PersonalAccountEntity { Id = request.Id };
            dbContext.PersonalAccounts.Add(entity);
        }

        entity.DisplayName = request.DisplayName;
        entity.Email = request.Email;
        entity.Secret = string.IsNullOrWhiteSpace(request.Secret)
            ? entity.Secret
            : EnsureHashedSecret(request.Secret);
        entity.SubscriptionStatus = request.SubscriptionStatus.ToString().ToLowerInvariant();
        entity.PlanName = request.PlanName;
        entity.CustomEndpointEnabled = request.CustomEndpointEnabled;
        entity.UpdatedAt = DateTimeOffset.UtcNow;
        await dbContext.SaveChangesAsync();

        var plan = await dbContext.AiSubscriptionPlans.AsNoTracking().FirstOrDefaultAsync(item => item.Code == entity.PlanName);
        var subscription = await dbContext.PersonalSubscriptions.AsNoTracking().FirstOrDefaultAsync(item => item.AccountId == entity.Id);
        return Map(entity, subscription, plan);
    }

    public async Task<AdminLoginResponse> AdminLoginAsync(AdminLoginRequest request)
    {
        var user = await dbContext.AdminUsers.AsNoTracking().FirstOrDefaultAsync(item => item.Username == request.Username);
        if (user is null || !VerifySecret(user.Password, request.Password))
        {
            throw new UnauthorizedAccessException("Invalid admin credentials.");
        }

        var session = await CreateSessionAsync(
            sessionType: "admin",
            subjectId: user.Id,
            subjectMode: "admin",
            role: user.Role,
            lifetime: AdminSessionLifetime,
            refreshLifetime: AdminRefreshLifetime);

        return new AdminLoginResponse
        {
            Token = session.Token,
            RefreshToken = session.RefreshToken,
            Username = user.Username,
            Role = user.Role,
            ExpiresAt = session.ExpiresAt,
            RefreshExpiresAt = session.RefreshExpiresAt,
        };
    }

    public async Task<AdminLoginResponse> RefreshAdminSessionAsync(RefreshTokenRequest request)
    {
        var session = await RefreshSessionAsync(request.RefreshToken, "admin");
        if (session is null)
        {
            throw new UnauthorizedAccessException("Admin refresh token expired or invalid.");
        }

        var user = await dbContext.AdminUsers.AsNoTracking().FirstAsync(item => item.Id == session.SubjectId);
        return new AdminLoginResponse
        {
            Token = session.Token,
            RefreshToken = session.RefreshToken,
            Username = user.Username,
            Role = user.Role,
            ExpiresAt = session.ExpiresAt,
            RefreshExpiresAt = session.RefreshExpiresAt,
        };
    }

    public async Task DeletePersonalAccountAsync(string accountId)
    {
        var entity = await dbContext.PersonalAccounts.FirstOrDefaultAsync(item => item.Id == accountId)
            ?? throw new KeyNotFoundException($"Personal account '{accountId}' was not found.");

        var subscription = await dbContext.PersonalSubscriptions.FirstOrDefaultAsync(item => item.AccountId == accountId);
        if (subscription is not null)
        {
            dbContext.PersonalSubscriptions.Remove(subscription);
        }

        dbContext.PersonalAccounts.Remove(entity);
        await dbContext.SaveChangesAsync();

        await RevokeClientSessionsAsync("personal", accountId);
        await DeleteClientSyncStatesAsync("personal", accountId);
    }

    public async Task<ClientLoginResponse> LoginAsync(ClientLoginRequest request)
    {
        if (request.Mode.Equals("personal", StringComparison.OrdinalIgnoreCase))
        {
            var personal = await dbContext.PersonalAccounts.AsNoTracking().FirstOrDefaultAsync(
                item => item.Email == request.Identifier || item.Id == request.Identifier);

            if (personal is null || !VerifySecret(personal.Secret, request.Secret))
            {
                throw new KeyNotFoundException("Personal account not found.");
            }

            return await BuildLoginResponse(
                mode: "personal",
                accountKey: personal.Id,
                displayName: personal.DisplayName,
                email: personal.Email,
                enterpriseId: string.Empty,
                enterpriseName: string.Empty,
                subAccountId: string.Empty);
        }

        if (request.Mode.Equals("enterpriseSubAccount", StringComparison.OrdinalIgnoreCase))
        {
            var subAccount = await dbContext.SubAccounts.AsNoTracking().FirstOrDefaultAsync(
                item => item.Email == request.Identifier || item.Id == request.Identifier);

            if (subAccount is null || !subAccount.Enabled || !VerifySecret(subAccount.Secret, request.Secret))
            {
                throw new KeyNotFoundException("Enterprise sub account not found.");
            }

            var enterprise = await dbContext.Enterprises.AsNoTracking().FirstOrDefaultAsync(item => item.Id == subAccount.EnterpriseId);
            return await BuildLoginResponse(
                mode: "enterpriseSubAccount",
                accountKey: subAccount.Id,
                displayName: subAccount.DisplayName,
                email: subAccount.Email,
                enterpriseId: subAccount.EnterpriseId,
                enterpriseName: enterprise?.Name ?? string.Empty,
                subAccountId: subAccount.Id);
        }

        return await BuildLoginResponse(
            mode: "local",
            accountKey: "local-workspace",
            displayName: "Local Workspace",
            email: string.Empty,
            enterpriseId: string.Empty,
            enterpriseName: string.Empty,
            subAccountId: string.Empty);
    }

    public async Task<ClientSyncResponse> SyncSettingsAsync(ClientSettingsSyncRequest request)
    {
        var session = await ValidateSessionAsync(request.AccessToken, "client");
        if (session is null)
        {
            throw new UnauthorizedAccessException("Client session expired or invalid.");
        }

        EnsureClientScopeMatchesSession(session, request.Mode, request.AccountKey);
        var state = await UpsertClientStateAsync(session.SubjectMode, session.SubjectId);
        state.DisplayName = request.DisplayName;
        state.Email = request.Email;
        state.EnterpriseId = request.EnterpriseId;
        state.EnterpriseName = request.EnterpriseName;
        state.SubAccountId = request.SubAccountId;
        state.AccessToken = request.AccessToken;
        state.SyncEndpointUrl = request.SyncEndpointUrl;
        state.OrganizationScope = request.OrganizationScope;
        state.SyncAssets = request.SyncAssets;
        state.SyncSettings = request.SyncSettings;
        state.UseCustomEndpoint = request.UseCustomEndpoint;
        state.EndpointName = request.EndpointName;
        state.Provider = request.Provider;
        state.BaseUrl = request.BaseUrl;
        state.ApiKey = request.ApiKey;
        state.ModelName = request.ModelName;
        state.SyncedSettingsJson = request.SettingsJson;
        state.UpdatedAt = DateTimeOffset.UtcNow;
        await dbContext.SaveChangesAsync();

        return await BuildSyncResponseAsync(state);
    }

    public async Task<ClientSyncResponse> SyncAssetsAsync(ClientAssetsSyncRequest request)
    {
        var session = await ValidateSessionAsync(request.AccessToken, "client");
        if (session is null)
        {
            throw new UnauthorizedAccessException("Client session expired or invalid.");
        }

        EnsureClientScopeMatchesSession(session, request.Mode, request.AccountKey);
        var state = await UpsertClientStateAsync(session.SubjectMode, session.SubjectId);
        state.SyncedAssetsJson = request.AssetsJson;
        state.UpdatedAt = DateTimeOffset.UtcNow;
        await dbContext.SaveChangesAsync();
        return await BuildSyncResponseAsync(state);
    }

    public async Task<ClientSyncResponse> PullSyncAsync(string mode, string accountKey, string accessToken)
    {
        var session = await ValidateSessionAsync(accessToken, "client");
        if (session is null)
        {
            throw new UnauthorizedAccessException("Client session expired or invalid.");
        }

        EnsureClientScopeMatchesSession(session, mode, accountKey);
        var state = await UpsertClientStateAsync(session.SubjectMode, session.SubjectId);
        return await BuildSyncResponseAsync(state);
    }

    private static void EnsureClientScopeMatchesSession(
        AuthSessionEntity session,
        string requestedMode,
        string requestedAccountKey)
    {
        if (!string.Equals(session.SubjectMode, requestedMode, StringComparison.OrdinalIgnoreCase) ||
            !string.Equals(session.SubjectId, requestedAccountKey, StringComparison.Ordinal))
        {
            throw new UnauthorizedAccessException("Client sync scope does not match the active session.");
        }
    }

    private async Task<ClientAccountSyncStateEntity> UpsertClientStateAsync(string mode, string accountKey)
    {
        var state = await dbContext.ClientSyncStates.FirstOrDefaultAsync(item => item.Mode == mode && item.AccountKey == accountKey);
        if (state is not null)
        {
            return state;
        }

        state = new ClientAccountSyncStateEntity
        {
            Mode = mode,
            AccountKey = accountKey,
            UseCustomEndpoint = true,
            UpdatedAt = DateTimeOffset.UtcNow,
        };
        dbContext.ClientSyncStates.Add(state);
        await dbContext.SaveChangesAsync();
        return state;
    }

    private async Task<ClientSyncResponse> BuildSyncResponseAsync(ClientAccountSyncStateEntity state)
    {
        var endpointSync = await dbContext.AiEndpoints.AsNoTracking().FirstAsync();
        var resolvedEnterpriseScope = state.Mode.Equals("enterpriseSubAccount", StringComparison.OrdinalIgnoreCase)
            ? await dbContext.SubAccounts.AsNoTracking()
                .FirstOrDefaultAsync(item => item.Id == state.AccountKey || item.Id == state.SubAccountId)
            : null;
        var resolvedEnterpriseId = resolvedEnterpriseScope?.EnterpriseId ?? state.EnterpriseId;
        var resolvedSubAccountId = resolvedEnterpriseScope?.Id ?? state.SubAccountId;
        var filteredAssetsJson = await BuildFilteredAssetsJsonAsync(state);
        var subscription = await BuildAiSubscriptionForScopeAsync(
            state.Mode,
            state.AccountKey,
            resolvedEnterpriseId,
            resolvedSubAccountId);
        return new ClientSyncResponse
        {
            SyncedAt = state.UpdatedAt,
            SettingsJson = state.SyncedSettingsJson,
            AssetsJson = filteredAssetsJson,
            AiSubscription = subscription,
            EndpointSync = Map(endpointSync),
            CustomEndpoint = new ClientAiEndpointConfig
            {
                UseCustomEndpoint = state.UseCustomEndpoint,
                EndpointName = state.EndpointName,
                Provider = state.Provider,
                BaseUrl = state.BaseUrl,
                ApiKey = state.ApiKey,
                ModelName = state.ModelName,
            },
            SubscriptionSnapshot = await BuildClientSubscriptionSnapshotAsync(
                state.Mode,
                state.AccountKey,
                resolvedEnterpriseId,
                resolvedSubAccountId,
                subscription),
        };
    }

    private async Task<string> BuildFilteredAssetsJsonAsync(ClientAccountSyncStateEntity state)
    {
        var syncedRecords = ParseSyncedCloudRecords(state.SyncedAssetsJson);

        if (state.Mode.Equals("enterpriseSubAccount", StringComparison.OrdinalIgnoreCase))
        {
            var subAccount = await dbContext.SubAccounts.AsNoTracking()
                .FirstOrDefaultAsync(item => item.Id == state.AccountKey || item.Id == state.SubAccountId);

            if (subAccount is null)
            {
                return "[]";
            }

            var allowedAssetIds = JsonSerializer.Deserialize<string[]>(subAccount.AssetIdsJson) ?? Array.Empty<string>();
            return await BuildEnterpriseScopedAssetsJsonAsync(allowedAssetIds, syncedRecords);
        }

        if (syncedRecords.Count > 0)
        {
            return JsonSerializer.Serialize(FilterCloudRecordsForScope(syncedRecords, state, null));
        }

        if (state.Mode.Equals("personal", StringComparison.OrdinalIgnoreCase))
        {
            var assets = await dbContext.Assets.AsNoTracking()
                .Where(asset => asset.OwnerType == "personal")
                .ToListAsync();

            return JsonSerializer.Serialize(assets.Select(MapCloudRecord));
        }

        var allAssets = await dbContext.Assets.AsNoTracking().ToListAsync();
        return JsonSerializer.Serialize(allAssets.Select(MapCloudRecord));
    }

    private static List<ClientCloudAssetRecord> ParseSyncedCloudRecords(string? rawJson)
    {
        if (string.IsNullOrWhiteSpace(rawJson))
        {
            return new List<ClientCloudAssetRecord>();
        }

        try
        {
            return JsonSerializer.Deserialize<List<ClientCloudAssetRecord>>(rawJson) ?? new List<ClientCloudAssetRecord>();
        }
        catch
        {
            return new List<ClientCloudAssetRecord>();
        }
    }

    private static IReadOnlyList<ClientCloudAssetRecord> FilterCloudRecordsForScope(
        IEnumerable<ClientCloudAssetRecord> records,
        ClientAccountSyncStateEntity state,
        IReadOnlyCollection<string>? allowedEnterpriseAssetIds)
    {
        if (state.Mode.Equals("enterpriseSubAccount", StringComparison.OrdinalIgnoreCase))
        {
            return records
                .Where(record =>
                    record.Asset is not null &&
                    record.Asset.CloudId is not null &&
                    allowedEnterpriseAssetIds is not null &&
                    allowedEnterpriseAssetIds.Contains(record.Asset.CloudId))
                .ToList();
        }

        if (state.Mode.Equals("personal", StringComparison.OrdinalIgnoreCase))
        {
            return records
                .Where(record => record.Asset is not null)
                .ToList();
        }

        return records.ToList();
    }

    private async Task<string> BuildEnterpriseScopedAssetsJsonAsync(
        IReadOnlyCollection<string> allowedAssetIds,
        IReadOnlyList<ClientCloudAssetRecord> syncedRecords)
    {
        if (allowedAssetIds.Count == 0)
        {
            return "[]";
        }

        var assets = await dbContext.Assets.AsNoTracking()
            .Where(asset => allowedAssetIds.Contains(asset.Id))
            .ToListAsync();

        var recordMap = syncedRecords
            .Where(record => record.Asset is not null)
            .Select(record => new
            {
                Key = ResolveCloudRecordKey(record),
                Record = record,
            })
            .Where(item => !string.IsNullOrWhiteSpace(item.Key))
            .GroupBy(item => item.Key!, StringComparer.Ordinal)
            .ToDictionary(group => group.Key, group => group.First().Record, StringComparer.Ordinal);

        var scopedRecords = new List<object>(assets.Count);
        foreach (var assetId in allowedAssetIds)
        {
            var asset = assets.FirstOrDefault(item => item.Id == assetId);
            if (asset is null)
            {
                continue;
            }

            if (recordMap.TryGetValue(assetId, out var existingRecord))
            {
                scopedRecords.Add(existingRecord);
                continue;
            }

            scopedRecords.Add(MapCloudRecord(asset));
        }

        return JsonSerializer.Serialize(scopedRecords);
    }

    private static string? ResolveCloudRecordKey(ClientCloudAssetRecord record)
    {
        if (!string.IsNullOrWhiteSpace(record.Asset?.CloudId))
        {
            return record.Asset.CloudId;
        }

        if (record.Asset?.Id is long id)
        {
            return id.ToString(CultureInfo.InvariantCulture);
        }

        return null;
    }

    private static object MapCloudRecord(AssetEntity asset) => new
    {
        asset = new
        {
            id = asset.Id,
            cloudId = asset.Id,
            name = asset.Name,
            host = asset.Host,
            port = 22,
            platform = "Linux",
            folderId = (int?)null,
            envId = (int?)null,
            labels = Array.Empty<string>(),
            owner = asset.OwnerType == "personal" ? $"personal:{asset.Id}" : asset.OwnerType,
            criticality = asset.RiskLevel,
            defaultWorkspacePath = (string?)null,
            accessEndpointId = (int?)null,
            bastionChainId = (string?)null,
            healthSummary = (string?)null,
            lastAccessedAt = (long?)null,
            isFavorite = false,
            groupId = (int?)null,
        },
        defaultAccessEndpoint = new
        {
            id = (int?)null,
            assetId = 0,
            name = $"{asset.Name} default endpoint",
            host = asset.Host,
            port = 22,
            username = "root",
            authType = "password",
            credentialRefId = (int?)null,
            sshKeyId = (int?)null,
            jumpHost = (string?)null,
            jumpPort = (int?)null,
            jumpUsername = (string?)null,
            jumpPassword = (string?)null,
        },
        defaultCredentialRef = (object?)null,
    };

    private async Task<ClientLoginResponse> BuildLoginResponse(
        string mode,
        string accountKey,
        string displayName,
        string email,
        string enterpriseId,
        string enterpriseName,
        string subAccountId)
    {
        var session = await CreateSessionAsync(
            sessionType: "client",
            subjectId: accountKey,
            subjectMode: mode,
            role: mode == "enterpriseSubAccount" ? "enterprise-sub-account" : mode,
            lifetime: ClientSessionLifetime,
            refreshLifetime: ClientRefreshLifetime);
        var endpointSync = await dbContext.AiEndpoints.AsNoTracking().FirstAsync();

        return new ClientLoginResponse
        {
            Mode = mode,
            AccountKey = accountKey,
            DisplayName = displayName,
            Email = email,
            EnterpriseId = enterpriseId,
            EnterpriseName = enterpriseName,
            SubAccountId = subAccountId,
            AccessToken = session.Token,
            RefreshToken = session.RefreshToken,
            ExpiresAt = session.ExpiresAt,
            RefreshExpiresAt = session.RefreshExpiresAt,
            SyncEndpointUrl = "/api/client/sync",
            AiSubscription = await BuildAiSubscriptionForScopeAsync(mode, accountKey, enterpriseId, subAccountId),
            EndpointSync = Map(endpointSync),
            CustomEndpoint = BuildCustomEndpoint(await UpsertClientStateAsync(mode, accountKey)),
            SubscriptionSnapshot = await BuildClientSubscriptionSnapshotAsync(mode, accountKey, enterpriseId, subAccountId),
        };
    }

    public async Task<ClientLoginResponse> RefreshClientSessionAsync(RefreshTokenRequest request)
    {
        var session = await RefreshSessionAsync(request.RefreshToken, "client");
        if (session is null)
        {
            throw new UnauthorizedAccessException("Client refresh token expired or invalid.");
        }

        var displayName = "Local Workspace";
        var email = string.Empty;
        var enterpriseId = string.Empty;
        var enterpriseName = string.Empty;
        var subAccountId = string.Empty;

        if (session.SubjectMode == "personal")
        {
            var personal = await dbContext.PersonalAccounts.AsNoTracking().FirstAsync(item => item.Id == session.SubjectId);
            displayName = personal.DisplayName;
            email = personal.Email;
        }
        else if (session.SubjectMode == "enterpriseSubAccount")
        {
            var subAccount = await dbContext.SubAccounts.AsNoTracking().FirstAsync(item => item.Id == session.SubjectId);
            var enterprise = await dbContext.Enterprises.AsNoTracking().FirstOrDefaultAsync(item => item.Id == subAccount.EnterpriseId);
            displayName = subAccount.DisplayName;
            email = subAccount.Email;
            enterpriseId = subAccount.EnterpriseId;
            enterpriseName = enterprise?.Name ?? string.Empty;
            subAccountId = subAccount.Id;
        }

        var endpointSync = await dbContext.AiEndpoints.AsNoTracking().FirstAsync();
        return new ClientLoginResponse
        {
            Mode = session.SubjectMode,
            AccountKey = session.SubjectId,
            DisplayName = displayName,
            Email = email,
            EnterpriseId = enterpriseId,
            EnterpriseName = enterpriseName,
            SubAccountId = subAccountId,
            AccessToken = session.Token,
            RefreshToken = session.RefreshToken,
            ExpiresAt = session.ExpiresAt,
            RefreshExpiresAt = session.RefreshExpiresAt,
            SyncEndpointUrl = "/api/client/sync",
            AiSubscription = await BuildAiSubscriptionForScopeAsync(
                session.SubjectMode,
                session.SubjectId,
                enterpriseId,
                subAccountId),
            EndpointSync = Map(endpointSync),
            CustomEndpoint = BuildCustomEndpoint(await UpsertClientStateAsync(session.SubjectMode, session.SubjectId)),
            SubscriptionSnapshot = await BuildClientSubscriptionSnapshotAsync(
                session.SubjectMode,
                session.SubjectId,
                enterpriseId,
                subAccountId),
        };
    }

    private async Task<ClientSubscriptionSnapshot> BuildClientSubscriptionSnapshotAsync(
        string mode,
        string accountKey,
        string? enterpriseId,
        string? subAccountId,
        AiSubscriptionOverview? resolvedSubscription = null)
    {
        var subscription = resolvedSubscription ?? await BuildAiSubscriptionForScopeAsync(
            mode,
            accountKey,
            enterpriseId,
            subAccountId);
        var currentMonth = DateTimeOffset.UtcNow.ToString("yyyy-MM");
        var scopedInvoices = await ListScopedInvoicesAsync(mode, accountKey, enterpriseId, subAccountId);
        var invoiceEntity = scopedInvoices
            .FirstOrDefault(item => item.BillingMonth == currentMonth);
        var usageRecords = await ListScopedUsageRecordsAsync(mode, accountKey, enterpriseId, subAccountId, currentMonth);

        var usagePricing = await dbContext.AiUsagePricing.AsNoTracking().ToListAsync();
        var paymentProviders = await dbContext.PaymentProviderConfigs.AsNoTracking()
            .Where(item => item.Enabled)
            .OrderBy(item => item.DisplayName)
            .ToListAsync();
        BillingInvoiceSummary? currentInvoice = null;
        if (invoiceEntity is not null)
        {
            var invoiceLineItems = await dbContext.BillingInvoiceLineItems.AsNoTracking()
                .Where(item => item.InvoiceId == invoiceEntity.Id)
                .ToListAsync();
            var paymentTransactions = await dbContext.PaymentTransactions.AsNoTracking()
                .Where(item => item.InvoiceId == invoiceEntity.Id)
                .ToListAsync();
            currentInvoice = Map(invoiceEntity, invoiceLineItems, paymentTransactions);
        }

        return new ClientSubscriptionSnapshot
        {
            Subscription = subscription,
            CurrentInvoice = currentInvoice,
            RecentInvoices = await BuildBillingInvoiceSummariesAsync(scopedInvoices.Take(6).ToList()),
            PaymentProviders = paymentProviders.Select(Map).ToList(),
            Usage = BuildScopedAiUsageSummary(usageRecords, usagePricing, subscription),
        };
    }

    private async Task<IReadOnlyList<BillingInvoiceEntity>> ListScopedInvoicesAsync(
        string mode,
        string accountKey,
        string? enterpriseId,
        string? subAccountId)
    {
        if (mode.Equals("enterpriseSubAccount", StringComparison.OrdinalIgnoreCase))
        {
            var subAccount = await dbContext.SubAccounts.AsNoTracking()
                .FirstOrDefaultAsync(item => item.Id == accountKey || item.Id == subAccountId);
            var resolvedEnterpriseId = subAccount?.EnterpriseId ?? enterpriseId;

            if (string.IsNullOrWhiteSpace(resolvedEnterpriseId))
            {
                return Array.Empty<BillingInvoiceEntity>();
            }

            return await dbContext.BillingInvoices.AsNoTracking()
                .Where(item => item.TargetType == "enterprise" && item.TargetId == resolvedEnterpriseId)
                .OrderByDescending(item => item.BillingMonth)
                .ToListAsync();
        }

        if (mode.Equals("personal", StringComparison.OrdinalIgnoreCase))
        {
            return await dbContext.BillingInvoices.AsNoTracking()
                .Where(item => item.TargetType == "personal" && item.TargetId == accountKey)
                .OrderByDescending(item => item.BillingMonth)
                .ToListAsync();
        }

        return Array.Empty<BillingInvoiceEntity>();
    }

    private async Task<BillingInvoiceEntity?> ResolveScopedInvoiceAsync(
        string mode,
        string accountKey,
        string? enterpriseId,
        string? subAccountId,
        string invoiceId)
    {
        var invoices = await ListScopedInvoicesAsync(mode, accountKey, enterpriseId, subAccountId);
        return invoices.FirstOrDefault(item => item.Id == invoiceId);
    }

    private async Task<IReadOnlyList<AiUsageRecordEntity>> ListScopedUsageRecordsAsync(
        string mode,
        string accountKey,
        string? enterpriseId,
        string? subAccountId,
        string billingMonth)
    {
        if (mode.Equals("enterpriseSubAccount", StringComparison.OrdinalIgnoreCase))
        {
            return await dbContext.AiUsageRecords.AsNoTracking()
                .Where(item =>
                    item.BillingMonth == billingMonth &&
                    item.AccountMode == "enterpriseSubAccount" &&
                    item.AccountId == accountKey)
                .ToListAsync();
        }

        if (mode.Equals("personal", StringComparison.OrdinalIgnoreCase))
        {
            return await dbContext.AiUsageRecords.AsNoTracking()
                .Where(item =>
                    item.BillingMonth == billingMonth &&
                    item.AccountMode == "personal" &&
                    item.AccountId == accountKey)
                .ToListAsync();
        }

        return Array.Empty<AiUsageRecordEntity>();
    }

    private async Task<IReadOnlyList<BillingInvoiceSummary>> BuildBillingInvoiceSummariesAsync(
        IReadOnlyList<BillingInvoiceEntity> invoices)
    {
        if (invoices.Count == 0)
        {
            return Array.Empty<BillingInvoiceSummary>();
        }

        var invoiceIds = invoices.Select(item => item.Id).ToList();
        var invoiceLineItems = await dbContext.BillingInvoiceLineItems.AsNoTracking()
            .Where(item => invoiceIds.Contains(item.InvoiceId))
            .ToListAsync();
        var paymentTransactions = await dbContext.PaymentTransactions.AsNoTracking()
            .Where(item => invoiceIds.Contains(item.InvoiceId))
            .ToListAsync();

        return invoices
            .Select(item => Map(item, invoiceLineItems, paymentTransactions))
            .ToList();
    }

    private async Task SyncEnterpriseSeatAssignmentAsync(string enterpriseId)
    {
        var activeSubAccounts = await dbContext.SubAccounts.CountAsync(
            item => item.EnterpriseId == enterpriseId && item.Enabled);

        var enterprise = await dbContext.Enterprises.FirstOrDefaultAsync(item => item.Id == enterpriseId);
        if (enterprise is not null)
        {
            enterprise.ActiveSubAccounts = activeSubAccounts;
        }

        var subscription = await dbContext.EnterpriseSubscriptions.FirstOrDefaultAsync(item => item.EnterpriseId == enterpriseId);
        if (subscription is not null)
        {
            subscription.SeatsAssigned = activeSubAccounts;
            subscription.UpdatedAt = DateTimeOffset.UtcNow;

            if (subscription.SeatsPurchased < activeSubAccounts)
            {
                subscription.Status = SubscriptionStatus.PastDue.ToString().ToLowerInvariant();
            }
        }

        await dbContext.SaveChangesAsync();
    }

    private async Task<bool> UpsertMonthlyInvoiceAsync(
        string targetType,
        string targetId,
        string planCode,
        int seatCount,
        double unitPrice,
        string currency,
        BillingInvoiceStatus status)
    {
        var billingMonth = DateTimeOffset.UtcNow.ToString("yyyy-MM");
        var invoiceId = $"inv-{targetId}-{billingMonth}";
        var entity = await dbContext.BillingInvoices.FirstOrDefaultAsync(item => item.Id == invoiceId);
        var created = entity is null;
        if (entity is null)
        {
            entity = new BillingInvoiceEntity
            {
                Id = invoiceId,
                TargetType = targetType,
                TargetId = targetId,
                BillingMonth = billingMonth,
                CreatedAt = DateTimeOffset.UtcNow,
            };
            dbContext.BillingInvoices.Add(entity);
        }

        entity.PlanCode = planCode;
        entity.Status = status.ToString().ToLowerInvariant();
        entity.SeatCount = seatCount;
        entity.UnitPrice = unitPrice;
        entity.SubscriptionAmount = seatCount * unitPrice;
        entity.AiUsageAmount = await CalculateAiUsageAmountAsync(targetType, targetId, billingMonth, unitPrice);
        entity.TotalAmount = entity.SubscriptionAmount + entity.AiUsageAmount;
        entity.Currency = currency;
        entity.DueAt = DateTimeOffset.UtcNow.AddDays(15);
        entity.UpdatedAt = DateTimeOffset.UtcNow;
        await dbContext.SaveChangesAsync();

        await RewriteInvoiceLineItemsAsync(entity, targetType, targetId, seatCount, unitPrice, currency, billingMonth);
        await RefreshInvoicePaymentStatusAsync(entity.Id);
        return created;
    }

    private async Task RefreshInvoicePaymentStatusAsync(string invoiceId)
    {
        var invoice = await dbContext.BillingInvoices.FirstOrDefaultAsync(item => item.Id == invoiceId);
        if (invoice is null)
        {
            return;
        }

        var paidAmount = await CalculateNetPaidAmountAsync(invoiceId);
        invoice.Status = DeriveInvoiceStatus(invoice, paidAmount, preserveVoidedIfUnpaid: true)
            .ToString()
            .ToLowerInvariant();

        invoice.UpdatedAt = DateTimeOffset.UtcNow;
        await dbContext.SaveChangesAsync();
    }

    private async Task<double> CalculateNetPaidAmountAsync(string invoiceId)
    {
        var transactions = await dbContext.PaymentTransactions.AsNoTracking()
            .Where(item => item.InvoiceId == invoiceId)
            .ToListAsync();

        var paid = transactions
            .Where(item => IsCreditPaymentStatus(item.Status))
            .Sum(item => item.Amount);
        var refunded = transactions
            .Where(item => IsRefundPaymentStatus(item.Status))
            .Sum(item => item.Amount);

        return paid - refunded;
    }

    private static bool IsCreditPaymentStatus(string status) =>
        string.Equals(status, "completed", StringComparison.OrdinalIgnoreCase) ||
        string.Equals(status, "settled", StringComparison.OrdinalIgnoreCase) ||
        string.Equals(status, "paid", StringComparison.OrdinalIgnoreCase);

    private static bool IsRefundPaymentStatus(string status) =>
        string.Equals(status, "refunded", StringComparison.OrdinalIgnoreCase) ||
        string.Equals(status, "refund", StringComparison.OrdinalIgnoreCase);

    private static bool ShouldRecordPaidAt(string status) =>
        IsCreditPaymentStatus(status) || IsRefundPaymentStatus(status);

    private static BillingInvoiceStatus DeriveInvoiceStatus(
        BillingInvoiceEntity invoice,
        double paidAmount,
        bool preserveVoidedIfUnpaid)
    {
        if (preserveVoidedIfUnpaid &&
            string.Equals(invoice.Status, BillingInvoiceStatus.Voided.ToString(), StringComparison.OrdinalIgnoreCase) &&
            paidAmount <= 0.0001d)
        {
            return BillingInvoiceStatus.Voided;
        }

        if (paidAmount + 0.0001d >= invoice.TotalAmount && invoice.TotalAmount > 0)
        {
            return BillingInvoiceStatus.Paid;
        }

        if (invoice.DueAt < DateTimeOffset.UtcNow && paidAmount + 0.0001d < invoice.TotalAmount)
        {
            return BillingInvoiceStatus.Overdue;
        }

        return BillingInvoiceStatus.Open;
    }

    private async Task RewriteInvoiceLineItemsAsync(
        BillingInvoiceEntity invoice,
        string targetType,
        string targetId,
        int seatCount,
        double unitPrice,
        string currency,
        string billingMonth)
    {
        var existingItems = await dbContext.BillingInvoiceLineItems
            .Where(item => item.InvoiceId == invoice.Id)
            .ToListAsync();
        if (existingItems.Count > 0)
        {
            dbContext.BillingInvoiceLineItems.RemoveRange(existingItems);
            await dbContext.SaveChangesAsync();
        }

        var lineItems = new List<BillingInvoiceLineItemEntity>
        {
            new()
            {
                Id = Guid.NewGuid().ToString("N"),
                InvoiceId = invoice.Id,
                ItemType = "subscription",
                Description = $"{invoice.PlanCode} seats",
                Quantity = seatCount,
                UnitPrice = unitPrice,
                Amount = invoice.SubscriptionAmount,
                Currency = currency,
                TotalTokens = null,
                CreatedAt = DateTimeOffset.UtcNow,
            }
        };

        var usageRecords = targetType == "enterprise"
            ? await dbContext.AiUsageRecords.AsNoTracking()
                .Where(item =>
                    item.BillingMonth == billingMonth &&
                    item.AccountMode == "enterpriseSubAccount" &&
                    dbContext.SubAccounts.Any(sub => sub.Id == item.AccountId && sub.EnterpriseId == targetId))
                .ToListAsync()
            : await dbContext.AiUsageRecords.AsNoTracking()
                .Where(item =>
                    item.BillingMonth == billingMonth &&
                    item.AccountId == targetId &&
                    item.AccountMode == "personal")
                .ToListAsync();

        lineItems.Add(new BillingInvoiceLineItemEntity
        {
            Id = Guid.NewGuid().ToString("N"),
            InvoiceId = invoice.Id,
            ItemType = "aiUsage",
            Description = "Managed AI usage",
            Quantity = usageRecords.Count,
            UnitPrice = usageRecords.Count == 0 ? 0 : invoice.AiUsageAmount / usageRecords.Count,
            Amount = invoice.AiUsageAmount,
            Currency = currency,
            TotalTokens = usageRecords.Sum(item => item.TotalTokens),
            CreatedAt = DateTimeOffset.UtcNow,
        });

        dbContext.BillingInvoiceLineItems.AddRange(lineItems);
        await dbContext.SaveChangesAsync();
    }

    private async Task<double> CalculateAiUsageAmountAsync(
        string targetType,
        string targetId,
        string billingMonth,
        double pricePerSeat)
    {
        var usageRecords = targetType == "enterprise"
            ? await dbContext.AiUsageRecords.AsNoTracking()
                .Where(item =>
                    item.BillingMonth == billingMonth &&
                    item.AccountMode == "enterpriseSubAccount" &&
                    dbContext.SubAccounts.Any(sub => sub.Id == item.AccountId && sub.EnterpriseId == targetId))
                .ToListAsync()
            : await dbContext.AiUsageRecords.AsNoTracking()
                .Where(item =>
                    item.BillingMonth == billingMonth &&
                    item.AccountId == targetId &&
                    item.AccountMode == "personal")
                .ToListAsync();

        return await EstimateAiUsageCostAsync(usageRecords, pricePerSeat);
    }

    private async Task RecordAiUsageAsync(
        AuthSessionEntity session,
        string provider,
        string modelName,
        bool usingManagedEndpoint,
        string responseContent)
    {
        try
        {
            using var document = JsonDocument.Parse(responseContent);
            if (!document.RootElement.TryGetProperty("usage", out var usageElement))
            {
                return;
            }

            var promptTokens = usageElement.TryGetProperty("prompt_tokens", out var prompt)
                ? prompt.GetInt32()
                : usageElement.TryGetProperty("input_tokens", out var input)
                    ? input.GetInt32()
                    : 0;
            var completionTokens = usageElement.TryGetProperty("completion_tokens", out var completion)
                ? completion.GetInt32()
                : usageElement.TryGetProperty("output_tokens", out var output)
                    ? output.GetInt32()
                    : 0;
            var totalTokens = usageElement.TryGetProperty("total_tokens", out var total)
                ? total.GetInt32()
                : promptTokens + completionTokens;

            var entity = new AiUsageRecordEntity
            {
                Id = Guid.NewGuid().ToString("N"),
                SessionToken = session.Token,
                AccountId = session.SubjectId,
                AccountMode = session.SubjectMode,
                Provider = provider,
                ModelName = modelName,
                UsingManagedEndpoint = usingManagedEndpoint,
                PromptTokens = promptTokens,
                CompletionTokens = completionTokens,
                TotalTokens = totalTokens,
                BillingMonth = DateTimeOffset.UtcNow.ToString("yyyy-MM"),
                CreatedAt = DateTimeOffset.UtcNow,
            };

            dbContext.AiUsageRecords.Add(entity);
            await dbContext.SaveChangesAsync();
        }
        catch
        {
            // Usage extraction is best-effort and should never break the proxied AI response.
        }
    }

    private async Task<AiSubscriptionOverview> BuildGlobalAiSubscriptionAsync()
    {
        var entity = await dbContext.AiSubscriptions.AsNoTracking().FirstAsync();
        var plan = await dbContext.AiSubscriptionPlans.AsNoTracking()
            .FirstOrDefaultAsync(item => item.Code == entity.PlanName);
        var enterpriseSeats = await dbContext.EnterpriseSubscriptions.AsNoTracking()
            .SumAsync(item => (int?)item.SeatsPurchased) ?? 0;
        var personalSeats = await dbContext.PersonalSubscriptions.AsNoTracking().CountAsync();
        var totalSeats = Math.Max(entity.Seats, enterpriseSeats + personalSeats);
        return Map(entity, plan, totalSeats);
    }

    private async Task<AiSubscriptionOverview> BuildAiSubscriptionForScopeAsync(
        string mode,
        string accountKey,
        string? enterpriseId,
        string? subAccountId)
    {
        var globalConfig = await dbContext.AiSubscriptions.AsNoTracking().FirstAsync();

        if (mode.Equals("personal", StringComparison.OrdinalIgnoreCase))
        {
            var personalSubscription = await dbContext.PersonalSubscriptions.AsNoTracking()
                .FirstOrDefaultAsync(item => item.AccountId == accountKey);
            if (personalSubscription is not null)
            {
                var plan = await dbContext.AiSubscriptionPlans.AsNoTracking()
                    .FirstOrDefaultAsync(item => item.Code == personalSubscription.PlanCode);
                return Map(personalSubscription, plan, globalConfig.ServiceMode, globalConfig.SyncCustomEndpoint);
            }

            var personal = await dbContext.PersonalAccounts.AsNoTracking().FirstOrDefaultAsync(item => item.Id == accountKey);
            if (personal is not null)
            {
                var plan = await dbContext.AiSubscriptionPlans.AsNoTracking()
                    .FirstOrDefaultAsync(item => item.Code == personal.PlanName);
                return Map(personal, plan, globalConfig.ServiceMode, globalConfig.SyncCustomEndpoint);
            }
        }

        if (mode.Equals("enterpriseSubAccount", StringComparison.OrdinalIgnoreCase))
        {
            var subAccount = await dbContext.SubAccounts.AsNoTracking()
                .FirstOrDefaultAsync(item => item.Id == accountKey || item.Id == subAccountId);
            var resolvedEnterpriseId = subAccount?.EnterpriseId ?? enterpriseId;

            if (!string.IsNullOrWhiteSpace(resolvedEnterpriseId))
            {
                var enterpriseSubscription = await dbContext.EnterpriseSubscriptions.AsNoTracking()
                    .FirstOrDefaultAsync(item => item.EnterpriseId == resolvedEnterpriseId);
                if (enterpriseSubscription is not null)
                {
                    var plan = await dbContext.AiSubscriptionPlans.AsNoTracking()
                        .FirstOrDefaultAsync(item => item.Code == enterpriseSubscription.PlanCode);
                    var seatsAssigned = await dbContext.SubAccounts.AsNoTracking()
                        .CountAsync(item => item.EnterpriseId == resolvedEnterpriseId && item.Enabled);
                    return Map(enterpriseSubscription, plan, globalConfig.ServiceMode, globalConfig.SyncCustomEndpoint, seatsAssigned);
                }

                var enterprise = await dbContext.Enterprises.AsNoTracking()
                    .FirstOrDefaultAsync(item => item.Id == resolvedEnterpriseId);
                if (enterprise is not null)
                {
                    var plan = await dbContext.AiSubscriptionPlans.AsNoTracking()
                        .FirstOrDefaultAsync(item => item.Code == enterprise.SubscriptionPlan);
                    return Map(enterprise, plan, globalConfig.ServiceMode, globalConfig.SyncCustomEndpoint);
                }
            }
        }

        return await BuildGlobalAiSubscriptionAsync();
    }

    private async Task<ScopedSubscriptionAccessState> EvaluateScopedSubscriptionAccessAsync(
        string mode,
        string accountKey,
        string? enterpriseId,
        string? subAccountId)
    {
        var subscription = await BuildAiSubscriptionForScopeAsync(mode, accountKey, enterpriseId, subAccountId);
        var normalizedStatus = subscription.Status.ToString();

        if (mode.Equals("enterpriseSubAccount", StringComparison.OrdinalIgnoreCase))
        {
            var subAccount = await dbContext.SubAccounts.AsNoTracking()
                .FirstOrDefaultAsync(item => item.Id == accountKey || item.Id == subAccountId);
            var resolvedEnterpriseId = subAccount?.EnterpriseId ?? enterpriseId;

            if (!string.IsNullOrWhiteSpace(resolvedEnterpriseId))
            {
                var enterpriseSubscription = await dbContext.EnterpriseSubscriptions.AsNoTracking()
                    .FirstOrDefaultAsync(item => item.EnterpriseId == resolvedEnterpriseId);
                if (enterpriseSubscription is not null)
                {
                    var seatsAssigned = await dbContext.SubAccounts.AsNoTracking()
                        .CountAsync(item => item.EnterpriseId == resolvedEnterpriseId && item.Enabled);
                    if (enterpriseSubscription.SeatsPurchased < seatsAssigned)
                    {
                        return new ScopedSubscriptionAccessState(
                            false,
                            "subscription-seat-limit-exceeded",
                            subscription);
                    }
                }
            }
        }

        var reason = normalizedStatus.Equals(nameof(SubscriptionStatus.Active), StringComparison.OrdinalIgnoreCase) ||
            normalizedStatus.Equals(nameof(SubscriptionStatus.Trialing), StringComparison.OrdinalIgnoreCase)
            ? string.Empty
            : normalizedStatus.Equals(nameof(SubscriptionStatus.PastDue), StringComparison.OrdinalIgnoreCase)
                ? "subscription-past-due"
                : normalizedStatus.Equals(nameof(SubscriptionStatus.Cancelled), StringComparison.OrdinalIgnoreCase)
                    ? "subscription-cancelled"
                    : "subscription-inactive";

        return new ScopedSubscriptionAccessState(
            string.IsNullOrEmpty(reason),
            reason,
            subscription);
    }

    private static EnterpriseSummary Map(
        EnterpriseEntity entity,
        EnterpriseSubscriptionEntity? subscription,
        AiSubscriptionPlanEntity? plan,
        int activeSubAccounts) => new()
    {
        Id = entity.Id,
        Name = entity.Name,
        SeatCount = subscription?.SeatsPurchased ?? entity.SeatCount,
        ActiveSubAccounts = activeSubAccounts,
        SubscriptionPlan = subscription?.PlanCode ?? entity.SubscriptionPlan,
        SubscriptionStatus = ParseSubscriptionStatus(subscription?.Status ?? entity.SubscriptionStatus),
        RenewAt = subscription?.RenewAt ?? entity.RenewAt,
    };

    private static EnterpriseSubAccountSummary Map(EnterpriseSubAccountEntity entity) => new()
    {
        Id = entity.Id,
        EnterpriseId = entity.EnterpriseId,
        DisplayName = entity.DisplayName,
        Email = entity.Email,
        Enabled = entity.Enabled,
        AssetIds = JsonSerializer.Deserialize<string[]>(entity.AssetIdsJson) ?? Array.Empty<string>(),
        UpdatedAt = entity.UpdatedAt,
    };

    private static PersonalAccountSummary Map(
        PersonalAccountEntity entity,
        PersonalSubscriptionEntity? subscription,
        AiSubscriptionPlanEntity? plan) => new()
    {
        Id = entity.Id,
        DisplayName = entity.DisplayName,
        Email = entity.Email,
        SubscriptionStatus = ParseSubscriptionStatus(subscription?.Status ?? entity.SubscriptionStatus),
        PlanName = subscription?.PlanCode ?? entity.PlanName,
        CustomEndpointEnabled = plan?.AllowCustomEndpoint ?? entity.CustomEndpointEnabled,
        UpdatedAt = entity.UpdatedAt,
    };

    private static ManagedAssetSummary Map(AssetEntity entity) => new()
    {
        Id = entity.Id,
        Name = entity.Name,
        Host = entity.Host,
        Environment = entity.Environment,
        RiskLevel = entity.RiskLevel,
        OwnerType = entity.OwnerType,
    };

    private static AiSubscriptionPlanSummary Map(AiSubscriptionPlanEntity entity) => new()
    {
        Code = entity.Code,
        DisplayName = entity.DisplayName,
        Scope = entity.Scope,
        PricePerSeat = entity.PricePerSeat,
        Currency = entity.Currency,
        AllowCustomEndpoint = entity.AllowCustomEndpoint,
        IsActive = entity.IsActive,
        Description = entity.Description,
        UpdatedAt = entity.UpdatedAt,
    };

    private static EnterpriseSubscriptionSummary Map(
        EnterpriseSubscriptionEntity entity,
        AiSubscriptionPlanEntity? plan,
        int seatsAssigned) => new()
    {
        EnterpriseId = entity.EnterpriseId,
        PlanCode = entity.PlanCode,
        PlanDisplayName = plan?.DisplayName ?? entity.PlanCode,
        Status = ParseSubscriptionStatus(entity.Status),
        SeatsPurchased = entity.SeatsPurchased,
        SeatsAssigned = seatsAssigned,
        PricePerSeat = plan?.PricePerSeat ?? 0,
        Currency = plan?.Currency ?? "USD",
        AllowCustomEndpoint = plan?.AllowCustomEndpoint ?? true,
        RenewAt = entity.RenewAt,
        UpdatedAt = entity.UpdatedAt,
    };

    private static PersonalSubscriptionSummary Map(
        PersonalSubscriptionEntity entity,
        AiSubscriptionPlanEntity? plan) => new()
    {
        AccountId = entity.AccountId,
        PlanCode = entity.PlanCode,
        PlanDisplayName = plan?.DisplayName ?? entity.PlanCode,
        Status = ParseSubscriptionStatus(entity.Status),
        PricePerSeat = plan?.PricePerSeat ?? 0,
        Currency = plan?.Currency ?? "USD",
        AllowCustomEndpoint = plan?.AllowCustomEndpoint ?? true,
        RenewAt = entity.RenewAt,
        UpdatedAt = entity.UpdatedAt,
    };

    private static AiSubscriptionOverview Map(
        AiSubscriptionEntity entity,
        AiSubscriptionPlanEntity? plan,
        int seats) => new()
    {
        ServiceMode = entity.ServiceMode,
        PlanName = entity.PlanName,
        PlanDisplayName = plan?.DisplayName ?? entity.PlanName,
        Status = ParseSubscriptionStatus(entity.Status),
        Seats = seats,
        PricePerSeat = plan?.PricePerSeat ?? 0,
        Currency = plan?.Currency ?? "USD",
        BillingScope = "global",
        AllowCustomEndpoint = plan?.AllowCustomEndpoint ?? entity.AllowCustomEndpoint,
        SyncCustomEndpoint = entity.SyncCustomEndpoint,
        RenewAt = entity.RenewAt,
    };

    private static AiSubscriptionOverview Map(
        EnterpriseSubscriptionEntity entity,
        AiSubscriptionPlanEntity? plan,
        string serviceMode,
        bool syncCustomEndpoint,
        int seatsAssigned) => new()
    {
        ServiceMode = serviceMode,
        PlanName = entity.PlanCode,
        PlanDisplayName = plan?.DisplayName ?? entity.PlanCode,
        Status = ParseSubscriptionStatus(entity.Status),
        Seats = entity.SeatsPurchased,
        PricePerSeat = plan?.PricePerSeat ?? 0,
        Currency = plan?.Currency ?? "USD",
        BillingScope = "enterprise",
        AllowCustomEndpoint = plan?.AllowCustomEndpoint ?? true,
        SyncCustomEndpoint = syncCustomEndpoint,
        RenewAt = entity.RenewAt,
    };

    private static AiSubscriptionOverview Map(
        EnterpriseEntity entity,
        AiSubscriptionPlanEntity? plan,
        string serviceMode,
        bool syncCustomEndpoint) => new()
    {
        ServiceMode = serviceMode,
        PlanName = entity.SubscriptionPlan,
        PlanDisplayName = plan?.DisplayName ?? entity.SubscriptionPlan,
        Status = ParseSubscriptionStatus(entity.SubscriptionStatus),
        Seats = entity.SeatCount,
        PricePerSeat = plan?.PricePerSeat ?? 0,
        Currency = plan?.Currency ?? "USD",
        BillingScope = "enterprise",
        AllowCustomEndpoint = plan?.AllowCustomEndpoint ?? true,
        SyncCustomEndpoint = syncCustomEndpoint,
        RenewAt = entity.RenewAt,
    };

    private static AiSubscriptionOverview Map(
        PersonalSubscriptionEntity entity,
        AiSubscriptionPlanEntity? plan,
        string serviceMode,
        bool syncCustomEndpoint) => new()
    {
        ServiceMode = serviceMode,
        PlanName = entity.PlanCode,
        PlanDisplayName = plan?.DisplayName ?? entity.PlanCode,
        Status = ParseSubscriptionStatus(entity.Status),
        Seats = 1,
        PricePerSeat = plan?.PricePerSeat ?? 0,
        Currency = plan?.Currency ?? "USD",
        BillingScope = "personal",
        AllowCustomEndpoint = plan?.AllowCustomEndpoint ?? true,
        SyncCustomEndpoint = syncCustomEndpoint,
        RenewAt = entity.RenewAt,
    };

    private static AiSubscriptionOverview Map(
        PersonalAccountEntity entity,
        AiSubscriptionPlanEntity? plan,
        string serviceMode,
        bool syncCustomEndpoint) => new()
    {
        ServiceMode = serviceMode,
        PlanName = entity.PlanName,
        PlanDisplayName = plan?.DisplayName ?? entity.PlanName,
        Status = ParseSubscriptionStatus(entity.SubscriptionStatus),
        Seats = 1,
        PricePerSeat = plan?.PricePerSeat ?? 0,
        Currency = plan?.Currency ?? "USD",
        BillingScope = "personal",
        AllowCustomEndpoint = plan?.AllowCustomEndpoint ?? entity.CustomEndpointEnabled,
        SyncCustomEndpoint = syncCustomEndpoint,
        RenewAt = DateTimeOffset.UtcNow.AddMonths(1),
    };

    private static AiEndpointSyncSettings Map(AiEndpointEntity entity) => new()
    {
        EndpointName = entity.EndpointName,
        Provider = entity.Provider,
        BaseUrl = entity.BaseUrl,
        ApiKey = entity.ApiKey,
        ModelName = entity.ModelName,
        SyncToClients = entity.SyncToClients,
        UpdatedAt = entity.UpdatedAt,
    };

    private static BillingOverview BuildBillingOverview(
        IReadOnlyList<BillingInvoiceEntity> invoices,
        IReadOnlyList<BillingInvoiceLineItemEntity> invoiceLineItems,
        IReadOnlyList<PaymentTransactionEntity> paymentTransactions)
    {
        var currentMonth = DateTimeOffset.UtcNow.ToString("yyyy-MM");
        var monthInvoices = invoices.Where(item => item.BillingMonth == currentMonth).ToList();
        var openStatuses = new[]
        {
            BillingInvoiceStatus.Open.ToString().ToLowerInvariant(),
            BillingInvoiceStatus.Overdue.ToString().ToLowerInvariant(),
        };

        return new BillingOverview
        {
            BillingMonth = currentMonth,
            EstimatedMonthlyRevenue = monthInvoices.Sum(item => item.TotalAmount),
            OutstandingAmount = monthInvoices
                .Where(item => openStatuses.Contains(item.Status, StringComparer.OrdinalIgnoreCase))
                .Sum(item => item.TotalAmount),
            OpenInvoiceCount = monthInvoices.Count(item => openStatuses.Contains(item.Status, StringComparer.OrdinalIgnoreCase)),
            RecentInvoices = invoices.Take(8).Select(item => Map(item, invoiceLineItems, paymentTransactions)).ToList(),
        };
    }

    private async Task<AiUsageSummary> BuildAiUsageSummaryAsync(
        IReadOnlyList<AiUsageRecordEntity> usageRecords,
        IReadOnlyList<AiUsagePricingEntity> usagePricing)
    {
        var currentMonth = DateTimeOffset.UtcNow.ToString("yyyy-MM");
        var monthRecords = usageRecords.Where(item => item.BillingMonth == currentMonth).ToList();
        var globalSubscription = await BuildGlobalAiSubscriptionAsync();
        var estimatedCost = EstimateAiUsageCost(monthRecords, usagePricing, globalSubscription.PricePerSeat);

        return new AiUsageSummary
        {
            BillingMonth = currentMonth,
            TotalRequests = monthRecords.Count,
            ManagedRequests = monthRecords.Count(item => item.UsingManagedEndpoint),
            PromptTokens = monthRecords.Sum(item => item.PromptTokens),
            CompletionTokens = monthRecords.Sum(item => item.CompletionTokens),
            TotalTokens = monthRecords.Sum(item => item.TotalTokens),
            EstimatedCost = estimatedCost,
            Currency = globalSubscription.Currency,
            TopAccounts = monthRecords
                .GroupBy(item => new { item.AccountId, item.AccountMode })
                .OrderByDescending(group => group.Sum(item => item.TotalTokens))
                .Take(8)
                .Select(group => new AiUsageAccountSummary
                {
                    AccountId = group.Key.AccountId,
                    AccountMode = group.Key.AccountMode,
                    Requests = group.Count(),
                    TotalTokens = group.Sum(item => item.TotalTokens),
                    EstimatedCost = EstimateAiUsageCost(
                        group.ToList(),
                        usagePricing,
                        globalSubscription.PricePerSeat),
                    Currency = globalSubscription.Currency,
                })
                .ToList(),
        };
    }

    private static AiUsageSummary BuildScopedAiUsageSummary(
        IReadOnlyList<AiUsageRecordEntity> usageRecords,
        IReadOnlyList<AiUsagePricingEntity> usagePricing,
        AiSubscriptionOverview subscription)
    {
        var estimatedCost = EstimateAiUsageCost(usageRecords, usagePricing, subscription.PricePerSeat);
        return new AiUsageSummary
        {
            BillingMonth = DateTimeOffset.UtcNow.ToString("yyyy-MM"),
            TotalRequests = usageRecords.Count,
            ManagedRequests = usageRecords.Count(item => item.UsingManagedEndpoint),
            PromptTokens = usageRecords.Sum(item => item.PromptTokens),
            CompletionTokens = usageRecords.Sum(item => item.CompletionTokens),
            TotalTokens = usageRecords.Sum(item => item.TotalTokens),
            EstimatedCost = estimatedCost,
            Currency = subscription.Currency,
            TopAccounts = usageRecords
                .GroupBy(item => new { item.AccountId, item.AccountMode })
                .OrderByDescending(group => group.Sum(item => item.TotalTokens))
                .Take(8)
                .Select(group => new AiUsageAccountSummary
                {
                    AccountId = group.Key.AccountId,
                    AccountMode = group.Key.AccountMode,
                    Requests = group.Count(),
                    TotalTokens = group.Sum(item => item.TotalTokens),
                    EstimatedCost = EstimateAiUsageCost(group.ToList(), usagePricing, subscription.PricePerSeat),
                    Currency = subscription.Currency,
                })
                .ToList(),
        };
    }

    private async Task<double> EstimateAiUsageCostAsync(
        IReadOnlyList<AiUsageRecordEntity> usageRecords,
        double fallbackPricePerSeat)
    {
        var pricing = await dbContext.AiUsagePricing.AsNoTracking().ToListAsync();
        return EstimateAiUsageCost(usageRecords, pricing, fallbackPricePerSeat);
    }

    private static double EstimateAiUsageCost(
        IReadOnlyList<AiUsageRecordEntity> usageRecords,
        IReadOnlyList<AiUsagePricingEntity> pricingCatalog,
        double fallbackPricePerSeat)
    {
        if (usageRecords.Count == 0)
        {
            return 0;
        }

        double total = 0;
        foreach (var usageRecord in usageRecords)
        {
          var pricing = pricingCatalog.FirstOrDefault(item =>
              item.IsActive &&
              item.Provider.Equals(usageRecord.Provider, StringComparison.OrdinalIgnoreCase) &&
              item.ModelName.Equals(usageRecord.ModelName, StringComparison.OrdinalIgnoreCase));

          if (pricing is not null)
          {
              total += (usageRecord.PromptTokens / 1_000_000d) * pricing.PromptTokenRatePerMillion;
              total += (usageRecord.CompletionTokens / 1_000_000d) * pricing.CompletionTokenRatePerMillion;
              continue;
          }

          if (fallbackPricePerSeat > 0)
          {
              total += (usageRecord.TotalTokens / 1_000_000d) * fallbackPricePerSeat;
          }
        }

        return Math.Round(total, 4);
    }

    private static BillingInvoiceSummary Map(
        BillingInvoiceEntity entity,
        IReadOnlyList<BillingInvoiceLineItemEntity> invoiceLineItems,
        IReadOnlyList<PaymentTransactionEntity> paymentTransactions)
    {
        var creditStatuses = new[] { "completed", "settled", "paid" };
        var refundStatuses = new[] { "refunded", "refund" };
        var paidAmount = paymentTransactions
            .Where(item => item.InvoiceId == entity.Id && creditStatuses.Contains(item.Status, StringComparer.OrdinalIgnoreCase))
            .Sum(item => item.Amount)
            - paymentTransactions
                .Where(item => item.InvoiceId == entity.Id && refundStatuses.Contains(item.Status, StringComparer.OrdinalIgnoreCase))
                .Sum(item => item.Amount);

        return new BillingInvoiceSummary
        {
            Id = entity.Id,
            TargetType = entity.TargetType,
            TargetId = entity.TargetId,
            PlanCode = entity.PlanCode,
            Status = ParseBillingInvoiceStatus(entity.Status),
            SeatCount = entity.SeatCount,
            UnitPrice = entity.UnitPrice,
            SubscriptionAmount = entity.SubscriptionAmount,
            AiUsageAmount = entity.AiUsageAmount,
            TotalAmount = entity.TotalAmount,
            Currency = entity.Currency,
            BillingMonth = entity.BillingMonth,
            DueAt = entity.DueAt,
            CreatedAt = entity.CreatedAt,
            UpdatedAt = entity.UpdatedAt,
            PaidAmount = paidAmount,
            RemainingAmount = Math.Max(entity.TotalAmount - paidAmount, 0),
            LineItems = invoiceLineItems
                .Where(item => item.InvoiceId == entity.Id)
                .Select(Map)
                .ToList(),
            Payments = paymentTransactions
                .Where(item => item.InvoiceId == entity.Id)
                .Select(Map)
                .ToList(),
        };
    }

    private static BillingInvoiceLineItemSummary Map(BillingInvoiceLineItemEntity entity) => new()
    {
        Id = entity.Id,
        InvoiceId = entity.InvoiceId,
        ItemType = entity.ItemType,
        Description = entity.Description,
        Quantity = entity.Quantity,
        UnitPrice = entity.UnitPrice,
        Amount = entity.Amount,
        Currency = entity.Currency,
        TotalTokens = entity.TotalTokens,
        CreatedAt = entity.CreatedAt,
    };

    private static PaymentTransactionSummary Map(PaymentTransactionEntity entity) => new()
    {
        Id = entity.Id,
        InvoiceId = entity.InvoiceId,
        TargetType = entity.TargetType,
        TargetId = entity.TargetId,
        ProviderKey = entity.ProviderKey,
        Amount = entity.Amount,
        Currency = entity.Currency,
        PaymentMethod = entity.PaymentMethod,
        Status = entity.Status,
        ExternalReference = entity.ExternalReference,
        Note = entity.Note,
        CheckoutUrl = entity.CheckoutUrl,
        ExpiresAt = entity.ExpiresAt,
        PaidAt = entity.PaidAt,
        CreatedAt = entity.CreatedAt,
        UpdatedAt = entity.UpdatedAt,
    };

    private static PaymentProviderConfigSummary Map(PaymentProviderConfigEntity entity) => BuildPaymentProviderSummary(entity);

    private static AiUsagePricingSummary Map(AiUsagePricingEntity entity) => new()
    {
        Id = entity.Id,
        Provider = entity.Provider,
        ModelName = entity.ModelName,
        PromptTokenRatePerMillion = entity.PromptTokenRatePerMillion,
        CompletionTokenRatePerMillion = entity.CompletionTokenRatePerMillion,
        Currency = entity.Currency,
        IsActive = entity.IsActive,
        UpdatedAt = entity.UpdatedAt,
    };

    private static SubscriptionStatus ParseSubscriptionStatus(string raw) =>
        Enum.TryParse<SubscriptionStatus>(raw, ignoreCase: true, out var status)
            ? status
            : SubscriptionStatus.Inactive;

    private static BillingInvoiceStatus ParseBillingInvoiceStatus(string raw) =>
        Enum.TryParse<BillingInvoiceStatus>(raw, ignoreCase: true, out var status)
            ? status
            : BillingInvoiceStatus.Open;
}
