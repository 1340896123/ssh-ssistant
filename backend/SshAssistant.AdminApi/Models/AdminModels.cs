namespace SshAssistant.AdminApi.Models;

public enum AdminAccountMode
{
    Personal,
    EnterpriseSubAccount,
    Local,
}

public enum SubscriptionStatus
{
    Inactive,
    Trialing,
    Active,
    PastDue,
    Cancelled,
}

public enum BillingInvoiceStatus
{
    Open,
    Paid,
    Overdue,
    Voided,
}

public sealed class PersonalAccountSummary
{
    public string Id { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string Email { get; set; } = string.Empty;
    public SubscriptionStatus SubscriptionStatus { get; set; } = SubscriptionStatus.Inactive;
    public string PlanName { get; set; } = "free";
    public bool CustomEndpointEnabled { get; set; }
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class EnterpriseSummary
{
    public string Id { get; set; } = string.Empty;
    public string Name { get; set; } = string.Empty;
    public int SeatCount { get; set; }
    public int ActiveSubAccounts { get; set; }
    public string SubscriptionPlan { get; set; } = "enterprise";
    public SubscriptionStatus SubscriptionStatus { get; set; } = SubscriptionStatus.Active;
    public DateTimeOffset RenewAt { get; set; }
}

public sealed class EnterpriseSubAccountSummary
{
    public string Id { get; set; } = string.Empty;
    public string EnterpriseId { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string Email { get; set; } = string.Empty;
    public bool Enabled { get; set; } = true;
    public IReadOnlyList<string> AssetIds { get; set; } = Array.Empty<string>();
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class ManagedAssetSummary
{
    public string Id { get; set; } = string.Empty;
    public string Name { get; set; } = string.Empty;
    public string Host { get; set; } = string.Empty;
    public string Environment { get; set; } = string.Empty;
    public string RiskLevel { get; set; } = "medium";
    public string OwnerType { get; set; } = "enterprise";
}

public sealed class AiSubscriptionOverview
{
    public string ServiceMode { get; set; } = "subscription";
    public string PlanName { get; set; } = "team";
    public string PlanDisplayName { get; set; } = "Team";
    public SubscriptionStatus Status { get; set; } = SubscriptionStatus.Active;
    public int Seats { get; set; } = 1;
    public double PricePerSeat { get; set; }
    public string Currency { get; set; } = "USD";
    public string BillingScope { get; set; } = "global";
    public bool AllowCustomEndpoint { get; set; } = true;
    public bool SyncCustomEndpoint { get; set; }
    public DateTimeOffset RenewAt { get; set; }
}

public sealed class AiSubscriptionPlanSummary
{
    public string Code { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string Scope { get; set; } = "personal";
    public double PricePerSeat { get; set; }
    public string Currency { get; set; } = "USD";
    public bool AllowCustomEndpoint { get; set; } = true;
    public bool IsActive { get; set; } = true;
    public string Description { get; set; } = string.Empty;
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class EnterpriseSubscriptionSummary
{
    public string EnterpriseId { get; set; } = string.Empty;
    public string PlanCode { get; set; } = "enterprise";
    public string PlanDisplayName { get; set; } = "Enterprise";
    public SubscriptionStatus Status { get; set; } = SubscriptionStatus.Active;
    public int SeatsPurchased { get; set; } = 1;
    public int SeatsAssigned { get; set; }
    public double PricePerSeat { get; set; }
    public string Currency { get; set; } = "USD";
    public bool AllowCustomEndpoint { get; set; } = true;
    public DateTimeOffset RenewAt { get; set; }
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class PersonalSubscriptionSummary
{
    public string AccountId { get; set; } = string.Empty;
    public string PlanCode { get; set; } = "personal";
    public string PlanDisplayName { get; set; } = "Personal";
    public SubscriptionStatus Status { get; set; } = SubscriptionStatus.Active;
    public double PricePerSeat { get; set; }
    public string Currency { get; set; } = "USD";
    public bool AllowCustomEndpoint { get; set; } = true;
    public DateTimeOffset RenewAt { get; set; }
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class AiEndpointSyncSettings
{
    public string EndpointName { get; set; } = string.Empty;
    public string Provider { get; set; } = "openai";
    public string BaseUrl { get; set; } = string.Empty;
    public string ApiKey { get; set; } = string.Empty;
    public string ModelName { get; set; } = string.Empty;
    public bool SyncToClients { get; set; } = true;
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class ClientAiEndpointConfig
{
    public bool UseCustomEndpoint { get; set; } = true;
    public string EndpointName { get; set; } = string.Empty;
    public string Provider { get; set; } = "openai";
    public string BaseUrl { get; set; } = string.Empty;
    public string ApiKey { get; set; } = string.Empty;
    public string ModelName { get; set; } = string.Empty;
}

public sealed class BillingInvoiceSummary
{
    public string Id { get; set; } = string.Empty;
    public string TargetType { get; set; } = "enterprise";
    public string TargetId { get; set; } = string.Empty;
    public string PlanCode { get; set; } = string.Empty;
    public BillingInvoiceStatus Status { get; set; } = BillingInvoiceStatus.Open;
    public int SeatCount { get; set; }
    public double UnitPrice { get; set; }
    public double SubscriptionAmount { get; set; }
    public double AiUsageAmount { get; set; }
    public double TotalAmount { get; set; }
    public string Currency { get; set; } = "USD";
    public string BillingMonth { get; set; } = string.Empty;
    public DateTimeOffset DueAt { get; set; }
    public DateTimeOffset CreatedAt { get; set; }
    public DateTimeOffset UpdatedAt { get; set; }
    public double PaidAmount { get; set; }
    public double RemainingAmount { get; set; }
    public IReadOnlyList<BillingInvoiceLineItemSummary> LineItems { get; set; } = Array.Empty<BillingInvoiceLineItemSummary>();
    public IReadOnlyList<PaymentTransactionSummary> Payments { get; set; } = Array.Empty<PaymentTransactionSummary>();
}

public sealed class BillingInvoiceLineItemSummary
{
    public string Id { get; set; } = string.Empty;
    public string InvoiceId { get; set; } = string.Empty;
    public string ItemType { get; set; } = "subscription";
    public string Description { get; set; } = string.Empty;
    public int Quantity { get; set; }
    public double UnitPrice { get; set; }
    public double Amount { get; set; }
    public string Currency { get; set; } = "USD";
    public int? TotalTokens { get; set; }
    public DateTimeOffset CreatedAt { get; set; }
}

public sealed class PaymentTransactionSummary
{
    public string Id { get; set; } = string.Empty;
    public string InvoiceId { get; set; } = string.Empty;
    public string TargetType { get; set; } = "enterprise";
    public string TargetId { get; set; } = string.Empty;
    public string ProviderKey { get; set; } = "manual";
    public double Amount { get; set; }
    public string Currency { get; set; } = "USD";
    public string PaymentMethod { get; set; } = "manual";
    public string Status { get; set; } = "completed";
    public string ExternalReference { get; set; } = string.Empty;
    public string Note { get; set; } = string.Empty;
    public string CheckoutUrl { get; set; } = string.Empty;
    public DateTimeOffset? ExpiresAt { get; set; }
    public DateTimeOffset? PaidAt { get; set; }
    public DateTimeOffset CreatedAt { get; set; }
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class PaymentProviderConfigSummary
{
    public string ProviderKey { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string ProviderType { get; set; } = "manual";
    public string WebhookSecret { get; set; } = string.Empty;
    public bool Enabled { get; set; } = true;
    public string MetadataJson { get; set; } = "{}";
    public string CheckoutBaseUrl { get; set; } = string.Empty;
    public string WebhookMode { get; set; } = "manual";
    public string ApiBaseUrl { get; set; } = string.Empty;
    public string SecretApiKey { get; set; } = string.Empty;
    public string StripeApiVersion { get; set; } = string.Empty;
    public int WebhookToleranceSeconds { get; set; } = 300;
    public string SuccessUrl { get; set; } = string.Empty;
    public string CancelUrl { get; set; } = string.Empty;
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class BillingOverview
{
    public string BillingMonth { get; set; } = string.Empty;
    public double EstimatedMonthlyRevenue { get; set; }
    public double OutstandingAmount { get; set; }
    public int OpenInvoiceCount { get; set; }
    public IReadOnlyList<BillingInvoiceSummary> RecentInvoices { get; set; } = Array.Empty<BillingInvoiceSummary>();
}

public sealed class AiUsageSummary
{
    public string BillingMonth { get; set; } = string.Empty;
    public int TotalRequests { get; set; }
    public int ManagedRequests { get; set; }
    public int PromptTokens { get; set; }
    public int CompletionTokens { get; set; }
    public int TotalTokens { get; set; }
    public double EstimatedCost { get; set; }
    public string Currency { get; set; } = "USD";
    public IReadOnlyList<AiUsageAccountSummary> TopAccounts { get; set; } = Array.Empty<AiUsageAccountSummary>();
}

public sealed class AiUsageAccountSummary
{
    public string AccountId { get; set; } = string.Empty;
    public string AccountMode { get; set; } = string.Empty;
    public int Requests { get; set; }
    public int TotalTokens { get; set; }
    public double EstimatedCost { get; set; }
    public string Currency { get; set; } = "USD";
}

public sealed class AiUsagePricingSummary
{
    public string Id { get; set; } = string.Empty;
    public string Provider { get; set; } = "openai";
    public string ModelName { get; set; } = string.Empty;
    public double PromptTokenRatePerMillion { get; set; }
    public double CompletionTokenRatePerMillion { get; set; }
    public string Currency { get; set; } = "USD";
    public bool IsActive { get; set; } = true;
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class GenerateBillingCycleResponse
{
    public BillingOverview Billing { get; set; } = new();
    public int GeneratedInvoices { get; set; }
}

public sealed class AdminDashboardSnapshot
{
    public IReadOnlyList<EnterpriseSummary> Enterprises { get; set; } = Array.Empty<EnterpriseSummary>();
    public IReadOnlyList<EnterpriseSubAccountSummary> SubAccounts { get; set; } = Array.Empty<EnterpriseSubAccountSummary>();
    public IReadOnlyList<PersonalAccountSummary> PersonalAccounts { get; set; } = Array.Empty<PersonalAccountSummary>();
    public IReadOnlyList<ManagedAssetSummary> Assets { get; set; } = Array.Empty<ManagedAssetSummary>();
    public IReadOnlyList<AiSubscriptionPlanSummary> SubscriptionPlans { get; set; } = Array.Empty<AiSubscriptionPlanSummary>();
    public IReadOnlyList<EnterpriseSubscriptionSummary> EnterpriseSubscriptions { get; set; } = Array.Empty<EnterpriseSubscriptionSummary>();
    public IReadOnlyList<PersonalSubscriptionSummary> PersonalSubscriptions { get; set; } = Array.Empty<PersonalSubscriptionSummary>();
    public IReadOnlyList<AiUsagePricingSummary> AiUsagePricing { get; set; } = Array.Empty<AiUsagePricingSummary>();
    public IReadOnlyList<PaymentProviderConfigSummary> PaymentProviders { get; set; } = Array.Empty<PaymentProviderConfigSummary>();
    public BillingOverview Billing { get; set; } = new();
    public AiUsageSummary AiUsage { get; set; } = new();
    public AiSubscriptionOverview AiSubscription { get; set; } = new();
    public AiEndpointSyncSettings EndpointSync { get; set; } = new();
}

public sealed class UpdateSubAccountAssetsRequest
{
    public List<string> AssetIds { get; set; } = new();
}

public sealed class UpdateAiSubscriptionRequest
{
    public string ServiceMode { get; set; } = "subscription";
    public string PlanName { get; set; } = "team";
    public SubscriptionStatus Status { get; set; } = SubscriptionStatus.Active;
    public int Seats { get; set; } = 1;
    public bool AllowCustomEndpoint { get; set; } = true;
    public bool SyncCustomEndpoint { get; set; }
    public DateTimeOffset RenewAt { get; set; } = DateTimeOffset.UtcNow.AddMonths(1);
}

public sealed class UpsertSubscriptionPlanRequest
{
    public string Code { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string Scope { get; set; } = "personal";
    public double PricePerSeat { get; set; }
    public string Currency { get; set; } = "USD";
    public bool AllowCustomEndpoint { get; set; } = true;
    public bool IsActive { get; set; } = true;
    public string Description { get; set; } = string.Empty;
}

public sealed class UpsertEnterpriseSubscriptionRequest
{
    public string EnterpriseId { get; set; } = string.Empty;
    public string PlanCode { get; set; } = "enterprise";
    public SubscriptionStatus Status { get; set; } = SubscriptionStatus.Active;
    public int SeatsPurchased { get; set; } = 1;
    public DateTimeOffset RenewAt { get; set; } = DateTimeOffset.UtcNow.AddMonths(1);
}

public sealed class UpsertPersonalSubscriptionRequest
{
    public string AccountId { get; set; } = string.Empty;
    public string PlanCode { get; set; } = "personal";
    public SubscriptionStatus Status { get; set; } = SubscriptionStatus.Active;
    public DateTimeOffset RenewAt { get; set; } = DateTimeOffset.UtcNow.AddMonths(1);
}

public sealed class UpsertAiUsagePricingRequest
{
    public string Id { get; set; } = string.Empty;
    public string Provider { get; set; } = "openai";
    public string ModelName { get; set; } = string.Empty;
    public double PromptTokenRatePerMillion { get; set; }
    public double CompletionTokenRatePerMillion { get; set; }
    public string Currency { get; set; } = "USD";
    public bool IsActive { get; set; } = true;
}

public sealed class UpdateBillingInvoiceRequest
{
    public BillingInvoiceStatus Status { get; set; } = BillingInvoiceStatus.Open;
}

public sealed class CreatePaymentTransactionRequest
{
    public string InvoiceId { get; set; } = string.Empty;
    public string ProviderKey { get; set; } = "manual";
    public double Amount { get; set; }
    public string Currency { get; set; } = "USD";
    public string PaymentMethod { get; set; } = "manual";
    public string Status { get; set; } = "completed";
    public string ExternalReference { get; set; } = string.Empty;
    public string Note { get; set; } = string.Empty;
    public DateTimeOffset? PaidAt { get; set; }
}

public sealed class CreateCheckoutSessionRequest
{
    public string InvoiceId { get; set; } = string.Empty;
    public string ProviderKey { get; set; } = string.Empty;
    public string ReturnUrl { get; set; } = string.Empty;
    public string CancelUrl { get; set; } = string.Empty;
}

public sealed class UpsertPaymentProviderConfigRequest
{
    public string ProviderKey { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string ProviderType { get; set; } = "manual";
    public string WebhookSecret { get; set; } = string.Empty;
    public bool Enabled { get; set; } = true;
    public string MetadataJson { get; set; } = "{}";
    public string? CheckoutBaseUrl { get; set; }
    public string? WebhookMode { get; set; }
    public string? ApiBaseUrl { get; set; }
    public string? SecretApiKey { get; set; }
    public string? StripeApiVersion { get; set; }
    public int? WebhookToleranceSeconds { get; set; }
    public string? SuccessUrl { get; set; }
    public string? CancelUrl { get; set; }
}

public sealed class PaymentWebhookEventRequest
{
    public string ProviderKey { get; set; } = string.Empty;
    public string WebhookSecret { get; set; } = string.Empty;
    public string EventType { get; set; } = string.Empty;
    public string Signature { get; set; } = string.Empty;
    public string ExternalReference { get; set; } = string.Empty;
    public string Status { get; set; } = string.Empty;
    public double Amount { get; set; }
    public string Currency { get; set; } = "USD";
    public string InvoiceId { get; set; } = string.Empty;
    public string Note { get; set; } = string.Empty;
    public string PayloadJson { get; set; } = "{}";
}

public sealed class UpdateEndpointSyncRequest
{
    public string EndpointName { get; set; } = string.Empty;
    public string Provider { get; set; } = "openai";
    public string BaseUrl { get; set; } = string.Empty;
    public string ApiKey { get; set; } = string.Empty;
    public string ModelName { get; set; } = string.Empty;
    public bool SyncToClients { get; set; } = true;
}

public sealed class UpsertEnterpriseRequest
{
    public string Id { get; set; } = string.Empty;
    public string Name { get; set; } = string.Empty;
    public int SeatCount { get; set; }
    public string SubscriptionPlan { get; set; } = "enterprise";
    public SubscriptionStatus SubscriptionStatus { get; set; } = SubscriptionStatus.Active;
    public DateTimeOffset RenewAt { get; set; } = DateTimeOffset.UtcNow.AddMonths(1);
}

public sealed class UpsertSubAccountRequest
{
    public string Id { get; set; } = string.Empty;
    public string EnterpriseId { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string Email { get; set; } = string.Empty;
    public string Secret { get; set; } = string.Empty;
    public bool Enabled { get; set; } = true;
    public List<string> AssetIds { get; set; } = new();
}

public sealed class UpsertPersonalAccountRequest
{
    public string Id { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string Email { get; set; } = string.Empty;
    public string Secret { get; set; } = string.Empty;
    public SubscriptionStatus SubscriptionStatus { get; set; } = SubscriptionStatus.Inactive;
    public string PlanName { get; set; } = "free";
    public bool CustomEndpointEnabled { get; set; }
}

public sealed class AdminLoginRequest
{
    public string Username { get; set; } = string.Empty;
    public string Password { get; set; } = string.Empty;
}

public sealed class AdminLoginResponse
{
    public string Token { get; set; } = string.Empty;
    public string RefreshToken { get; set; } = string.Empty;
    public string Username { get; set; } = string.Empty;
    public string Role { get; set; } = string.Empty;
    public DateTimeOffset ExpiresAt { get; set; }
    public DateTimeOffset RefreshExpiresAt { get; set; }
}

public sealed class ClientLoginRequest
{
    public string Mode { get; set; } = "local";
    public string Identifier { get; set; } = string.Empty;
    public string Secret { get; set; } = string.Empty;
}

public sealed class ClientLoginResponse
{
    public string Mode { get; set; } = "local";
    public string AccountKey { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string Email { get; set; } = string.Empty;
    public string EnterpriseId { get; set; } = string.Empty;
    public string EnterpriseName { get; set; } = string.Empty;
    public string SubAccountId { get; set; } = string.Empty;
    public string AccessToken { get; set; } = string.Empty;
    public string RefreshToken { get; set; } = string.Empty;
    public DateTimeOffset ExpiresAt { get; set; }
    public DateTimeOffset RefreshExpiresAt { get; set; }
    public string SyncEndpointUrl { get; set; } = string.Empty;
    public AiSubscriptionOverview AiSubscription { get; set; } = new();
    public AiEndpointSyncSettings EndpointSync { get; set; } = new();
    public ClientAiEndpointConfig CustomEndpoint { get; set; } = new();
    public ClientSubscriptionSnapshot SubscriptionSnapshot { get; set; } = new();
}

public sealed class RefreshTokenRequest
{
    public string RefreshToken { get; set; } = string.Empty;
}

public sealed class ClientSettingsSyncRequest
{
    public string Mode { get; set; } = "local";
    public string AccountKey { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string Email { get; set; } = string.Empty;
    public string EnterpriseId { get; set; } = string.Empty;
    public string EnterpriseName { get; set; } = string.Empty;
    public string SubAccountId { get; set; } = string.Empty;
    public string AccessToken { get; set; } = string.Empty;
    public string SyncEndpointUrl { get; set; } = string.Empty;
    public string OrganizationScope { get; set; } = string.Empty;
    public bool SyncAssets { get; set; } = true;
    public bool SyncSettings { get; set; } = true;
    public string SettingsJson { get; set; } = "{}";
    public bool UseCustomEndpoint { get; set; } = true;
    public string EndpointName { get; set; } = string.Empty;
    public string Provider { get; set; } = "openai";
    public string BaseUrl { get; set; } = string.Empty;
    public string ApiKey { get; set; } = string.Empty;
    public string ModelName { get; set; } = string.Empty;
}

public sealed class ClientAssetsSyncRequest
{
    public string Mode { get; set; } = "local";
    public string AccountKey { get; set; } = string.Empty;
    public string AccessToken { get; set; } = string.Empty;
    public string AssetsJson { get; set; } = "[]";
}

public sealed class ClientSyncResponse
{
    public DateTimeOffset SyncedAt { get; set; }
    public string SettingsJson { get; set; } = "{}";
    public string AssetsJson { get; set; } = "[]";
    public AiSubscriptionOverview AiSubscription { get; set; } = new();
    public AiEndpointSyncSettings EndpointSync { get; set; } = new();
    public ClientAiEndpointConfig CustomEndpoint { get; set; } = new();
    public ClientSubscriptionSnapshot SubscriptionSnapshot { get; set; } = new();
}

public sealed class ClientAiRuntimeResponse
{
    public bool Enabled { get; set; }
    public string Reason { get; set; } = string.Empty;
    public string Provider { get; set; } = "openai";
    public string BaseUrl { get; set; } = string.Empty;
    public string ModelName { get; set; } = string.Empty;
    public string ApiKey { get; set; } = string.Empty;
    public bool UsingManagedEndpoint { get; set; }
}

public sealed class ClientSubscriptionSnapshot
{
    public AiSubscriptionOverview Subscription { get; set; } = new();
    public BillingInvoiceSummary? CurrentInvoice { get; set; }
    public IReadOnlyList<BillingInvoiceSummary> RecentInvoices { get; set; } = Array.Empty<BillingInvoiceSummary>();
    public IReadOnlyList<PaymentProviderConfigSummary> PaymentProviders { get; set; } = Array.Empty<PaymentProviderConfigSummary>();
    public AiUsageSummary Usage { get; set; } = new();
}
