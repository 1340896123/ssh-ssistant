namespace SshAssistant.AdminApi.Entities;

public sealed class EnterpriseEntity
{
    public string Id { get; set; } = string.Empty;
    public string Name { get; set; } = string.Empty;
    public int SeatCount { get; set; }
    public int ActiveSubAccounts { get; set; }
    public string SubscriptionPlan { get; set; } = "enterprise";
    public string SubscriptionStatus { get; set; } = "active";
    public DateTimeOffset RenewAt { get; set; }
}

public sealed class EnterpriseSubAccountEntity
{
    public string Id { get; set; } = string.Empty;
    public string EnterpriseId { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string Email { get; set; } = string.Empty;
    public string Secret { get; set; } = string.Empty;
    public bool Enabled { get; set; } = true;
    public string AssetIdsJson { get; set; } = "[]";
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class PersonalAccountEntity
{
    public string Id { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string Email { get; set; } = string.Empty;
    public string Secret { get; set; } = string.Empty;
    public string SubscriptionStatus { get; set; } = "inactive";
    public string PlanName { get; set; } = "free";
    public bool CustomEndpointEnabled { get; set; }
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class AdminUserEntity
{
    public string Id { get; set; } = string.Empty;
    public string Username { get; set; } = string.Empty;
    public string Password { get; set; } = string.Empty;
    public string Role { get; set; } = "admin";
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class AssetEntity
{
    public string Id { get; set; } = string.Empty;
    public string Name { get; set; } = string.Empty;
    public string Host { get; set; } = string.Empty;
    public string Environment { get; set; } = string.Empty;
    public string RiskLevel { get; set; } = "medium";
    public string OwnerType { get; set; } = "enterprise";
}

public sealed class AiSubscriptionEntity
{
    public int Id { get; set; }
    public string ServiceMode { get; set; } = "subscription";
    public string PlanName { get; set; } = "team";
    public string Status { get; set; } = "active";
    public int Seats { get; set; } = 1;
    public bool AllowCustomEndpoint { get; set; } = true;
    public bool SyncCustomEndpoint { get; set; }
    public DateTimeOffset RenewAt { get; set; }
}

public sealed class AiSubscriptionPlanEntity
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

public sealed class EnterpriseSubscriptionEntity
{
    public string EnterpriseId { get; set; } = string.Empty;
    public string PlanCode { get; set; } = "enterprise";
    public string Status { get; set; } = "active";
    public int SeatsPurchased { get; set; } = 1;
    public int SeatsAssigned { get; set; }
    public DateTimeOffset RenewAt { get; set; }
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class PersonalSubscriptionEntity
{
    public string AccountId { get; set; } = string.Empty;
    public string PlanCode { get; set; } = "personal";
    public string Status { get; set; } = "inactive";
    public DateTimeOffset RenewAt { get; set; }
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class BillingInvoiceEntity
{
    public string Id { get; set; } = string.Empty;
    public string TargetType { get; set; } = "enterprise";
    public string TargetId { get; set; } = string.Empty;
    public string PlanCode { get; set; } = string.Empty;
    public string Status { get; set; } = "open";
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
}

public sealed class BillingInvoiceLineItemEntity
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

public sealed class PaymentTransactionEntity
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

public sealed class PaymentProviderConfigEntity
{
    public string ProviderKey { get; set; } = string.Empty;
    public string DisplayName { get; set; } = string.Empty;
    public string ProviderType { get; set; } = "manual";
    public string WebhookSecret { get; set; } = string.Empty;
    public bool Enabled { get; set; } = true;
    public string MetadataJson { get; set; } = "{}";
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class AiUsageRecordEntity
{
    public string Id { get; set; } = string.Empty;
    public string SessionToken { get; set; } = string.Empty;
    public string AccountId { get; set; } = string.Empty;
    public string AccountMode { get; set; } = string.Empty;
    public string Provider { get; set; } = "openai";
    public string ModelName { get; set; } = string.Empty;
    public bool UsingManagedEndpoint { get; set; }
    public int PromptTokens { get; set; }
    public int CompletionTokens { get; set; }
    public int TotalTokens { get; set; }
    public string BillingMonth { get; set; } = string.Empty;
    public DateTimeOffset CreatedAt { get; set; }
}

public sealed class AiUsagePricingEntity
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

public sealed class AiEndpointEntity
{
    public int Id { get; set; }
    public string EndpointName { get; set; } = string.Empty;
    public string Provider { get; set; } = "openai";
    public string BaseUrl { get; set; } = string.Empty;
    public string ApiKey { get; set; } = string.Empty;
    public string ModelName { get; set; } = string.Empty;
    public bool SyncToClients { get; set; } = true;
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class ClientAccountSyncStateEntity
{
    public int Id { get; set; }
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
    public bool UseCustomEndpoint { get; set; } = true;
    public string EndpointName { get; set; } = string.Empty;
    public string Provider { get; set; } = "openai";
    public string BaseUrl { get; set; } = string.Empty;
    public string ApiKey { get; set; } = string.Empty;
    public string ModelName { get; set; } = string.Empty;
    public string SyncedSettingsJson { get; set; } = "{}";
    public string SyncedAssetsJson { get; set; } = "[]";
    public DateTimeOffset UpdatedAt { get; set; }
}

public sealed class AuthSessionEntity
{
    public string Token { get; set; } = string.Empty;
    public string RefreshToken { get; set; } = string.Empty;
    public string SessionType { get; set; } = string.Empty;
    public string SubjectId { get; set; } = string.Empty;
    public string SubjectMode { get; set; } = string.Empty;
    public string Role { get; set; } = string.Empty;
    public DateTimeOffset CreatedAt { get; set; }
    public DateTimeOffset ExpiresAt { get; set; }
    public DateTimeOffset RefreshExpiresAt { get; set; }
    public DateTimeOffset? RevokedAt { get; set; }
}
