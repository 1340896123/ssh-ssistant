using System.Text.Json;
using BCrypt.Net;
using Microsoft.EntityFrameworkCore;
using SshAssistant.AdminApi.Entities;

namespace SshAssistant.AdminApi.Data;

public static class AdminSeed
{
    public static async Task SeedAsync(AdminDbContext dbContext)
    {
        await dbContext.Database.EnsureCreatedAsync();
        await dbContext.Database.ExecuteSqlRawAsync("""
            CREATE TABLE IF NOT EXISTS AiSubscriptionPlans (
                Code TEXT NOT NULL PRIMARY KEY,
                DisplayName TEXT NOT NULL,
                Scope TEXT NOT NULL,
                PricePerSeat REAL NOT NULL,
                Currency TEXT NOT NULL,
                AllowCustomEndpoint INTEGER NOT NULL,
                IsActive INTEGER NOT NULL,
                Description TEXT NOT NULL,
                UpdatedAt TEXT NOT NULL
            );
            """);
        await dbContext.Database.ExecuteSqlRawAsync("""
            CREATE TABLE IF NOT EXISTS EnterpriseSubscriptions (
                EnterpriseId TEXT NOT NULL PRIMARY KEY,
                PlanCode TEXT NOT NULL,
                Status TEXT NOT NULL,
                SeatsPurchased INTEGER NOT NULL,
                SeatsAssigned INTEGER NOT NULL,
                RenewAt TEXT NOT NULL,
                UpdatedAt TEXT NOT NULL
            );
            """);
        await dbContext.Database.ExecuteSqlRawAsync("""
            CREATE TABLE IF NOT EXISTS PersonalSubscriptions (
                AccountId TEXT NOT NULL PRIMARY KEY,
                PlanCode TEXT NOT NULL,
                Status TEXT NOT NULL,
                RenewAt TEXT NOT NULL,
                UpdatedAt TEXT NOT NULL
            );
            """);
        await dbContext.Database.ExecuteSqlRawAsync("""
            CREATE TABLE IF NOT EXISTS BillingInvoices (
                Id TEXT NOT NULL PRIMARY KEY,
                TargetType TEXT NOT NULL,
                TargetId TEXT NOT NULL,
                PlanCode TEXT NOT NULL,
                Status TEXT NOT NULL,
                SeatCount INTEGER NOT NULL,
                UnitPrice REAL NOT NULL,
                SubscriptionAmount REAL NOT NULL,
                AiUsageAmount REAL NOT NULL,
                TotalAmount REAL NOT NULL,
                Currency TEXT NOT NULL,
                BillingMonth TEXT NOT NULL,
                DueAt TEXT NOT NULL,
                CreatedAt TEXT NOT NULL,
                UpdatedAt TEXT NOT NULL
            );
            """);
        await dbContext.Database.ExecuteSqlRawAsync("""
            CREATE TABLE IF NOT EXISTS BillingInvoiceLineItems (
                Id TEXT NOT NULL PRIMARY KEY,
                InvoiceId TEXT NOT NULL,
                ItemType TEXT NOT NULL,
                Description TEXT NOT NULL,
                Quantity INTEGER NOT NULL,
                UnitPrice REAL NOT NULL,
                Amount REAL NOT NULL,
                Currency TEXT NOT NULL,
                TotalTokens INTEGER,
                CreatedAt TEXT NOT NULL
            );
            """);
        await dbContext.Database.ExecuteSqlRawAsync("""
            CREATE TABLE IF NOT EXISTS PaymentTransactions (
                Id TEXT NOT NULL PRIMARY KEY,
                InvoiceId TEXT NOT NULL,
                TargetType TEXT NOT NULL,
                TargetId TEXT NOT NULL,
                ProviderKey TEXT NOT NULL,
                Amount REAL NOT NULL,
                Currency TEXT NOT NULL,
                PaymentMethod TEXT NOT NULL,
                Status TEXT NOT NULL,
                ExternalReference TEXT NOT NULL,
                Note TEXT NOT NULL,
                CheckoutUrl TEXT NOT NULL,
                ExpiresAt TEXT,
                PaidAt TEXT,
                CreatedAt TEXT NOT NULL,
                UpdatedAt TEXT NOT NULL
            );
            """);
        await dbContext.Database.ExecuteSqlRawAsync("""
            CREATE TABLE IF NOT EXISTS PaymentProviderConfigs (
                ProviderKey TEXT NOT NULL PRIMARY KEY,
                DisplayName TEXT NOT NULL,
                ProviderType TEXT NOT NULL,
                WebhookSecret TEXT NOT NULL,
                Enabled INTEGER NOT NULL,
                MetadataJson TEXT NOT NULL,
                UpdatedAt TEXT NOT NULL
            );
            """);
        await dbContext.Database.ExecuteSqlRawAsync("""
            CREATE TABLE IF NOT EXISTS AiUsageRecords (
                Id TEXT NOT NULL PRIMARY KEY,
                SessionToken TEXT NOT NULL,
                AccountId TEXT NOT NULL,
                AccountMode TEXT NOT NULL,
                Provider TEXT NOT NULL,
                ModelName TEXT NOT NULL,
                UsingManagedEndpoint INTEGER NOT NULL,
                PromptTokens INTEGER NOT NULL,
                CompletionTokens INTEGER NOT NULL,
                TotalTokens INTEGER NOT NULL,
                BillingMonth TEXT NOT NULL,
                CreatedAt TEXT NOT NULL
            );
            """);
        await dbContext.Database.ExecuteSqlRawAsync("""
            CREATE TABLE IF NOT EXISTS AiUsagePricing (
                Id TEXT NOT NULL PRIMARY KEY,
                Provider TEXT NOT NULL,
                ModelName TEXT NOT NULL,
                PromptTokenRatePerMillion REAL NOT NULL,
                CompletionTokenRatePerMillion REAL NOT NULL,
                Currency TEXT NOT NULL,
                IsActive INTEGER NOT NULL,
                UpdatedAt TEXT NOT NULL
            );
            """);
        try
        {
            await dbContext.Database.ExecuteSqlRawAsync("""
                ALTER TABLE AiEndpoints ADD COLUMN ApiKey TEXT NOT NULL DEFAULT '';
                """);
        }
        catch
        {
            // Column already exists in upgraded environments.
        }
        try
        {
            await dbContext.Database.ExecuteSqlRawAsync("""
                ALTER TABLE ClientSyncStates ADD COLUMN UseCustomEndpoint INTEGER NOT NULL DEFAULT 1;
                """);
        }
        catch
        {
        }
        try
        {
            await dbContext.Database.ExecuteSqlRawAsync("""
                ALTER TABLE ClientSyncStates ADD COLUMN EndpointName TEXT NOT NULL DEFAULT '';
                """);
        }
        catch
        {
        }
        try
        {
            await dbContext.Database.ExecuteSqlRawAsync("""
                ALTER TABLE ClientSyncStates ADD COLUMN Provider TEXT NOT NULL DEFAULT 'openai';
                """);
        }
        catch
        {
        }
        try
        {
            await dbContext.Database.ExecuteSqlRawAsync("""
                ALTER TABLE ClientSyncStates ADD COLUMN BaseUrl TEXT NOT NULL DEFAULT '';
                """);
        }
        catch
        {
        }
        try
        {
            await dbContext.Database.ExecuteSqlRawAsync("""
                ALTER TABLE ClientSyncStates ADD COLUMN ApiKey TEXT NOT NULL DEFAULT '';
                """);
        }
        catch
        {
        }
        try
        {
            await dbContext.Database.ExecuteSqlRawAsync("""
                ALTER TABLE ClientSyncStates ADD COLUMN ModelName TEXT NOT NULL DEFAULT '';
                """);
        }
        catch
        {
        }
        try
        {
            await dbContext.Database.ExecuteSqlRawAsync("""
                ALTER TABLE BillingInvoices ADD COLUMN SubscriptionAmount REAL NOT NULL DEFAULT 0;
                """);
        }
        catch
        {
        }
        try
        {
            await dbContext.Database.ExecuteSqlRawAsync("""
                ALTER TABLE BillingInvoices ADD COLUMN AiUsageAmount REAL NOT NULL DEFAULT 0;
                """);
        }
        catch
        {
        }
        try
        {
            await dbContext.Database.ExecuteSqlRawAsync("""
                ALTER TABLE PaymentTransactions ADD COLUMN ProviderKey TEXT NOT NULL DEFAULT 'manual';
                """);
        }
        catch
        {
        }
        try
        {
            await dbContext.Database.ExecuteSqlRawAsync("""
                ALTER TABLE PaymentTransactions ADD COLUMN CheckoutUrl TEXT NOT NULL DEFAULT '';
                """);
        }
        catch
        {
        }
        try
        {
            await dbContext.Database.ExecuteSqlRawAsync("""
                ALTER TABLE PaymentTransactions ADD COLUMN ExpiresAt TEXT;
                """);
        }
        catch
        {
        }

        if (!await dbContext.Enterprises.AnyAsync())
        {
            dbContext.Enterprises.Add(new EnterpriseEntity
            {
                Id = "ent-acme",
                Name = "Acme Infra",
                SeatCount = 40,
                ActiveSubAccounts = 2,
                SubscriptionPlan = "enterprise",
                SubscriptionStatus = "active",
                RenewAt = DateTimeOffset.UtcNow.AddMonths(1),
            });
        }

        if (!await dbContext.Assets.AnyAsync())
        {
            dbContext.Assets.AddRange(
                new AssetEntity { Id = "asset-app-01", Name = "app-prod-01", Host = "10.0.0.21", Environment = "production", RiskLevel = "critical", OwnerType = "enterprise" },
                new AssetEntity { Id = "asset-app-02", Name = "app-staging-01", Host = "10.0.1.15", Environment = "staging", RiskLevel = "medium", OwnerType = "enterprise" },
                new AssetEntity { Id = "asset-personal-01", Name = "personal-lab", Host = "172.16.10.8", Environment = "lab", RiskLevel = "low", OwnerType = "personal" }
            );
        }

        if (!await dbContext.SubAccounts.AnyAsync())
        {
            dbContext.SubAccounts.AddRange(
                new EnterpriseSubAccountEntity
                {
                    Id = "sub-lina",
                    EnterpriseId = "ent-acme",
                    DisplayName = "Lina Ops",
                    Email = "lina@acme.example",
                    Secret = BCrypt.Net.BCrypt.HashPassword("lina-pass"),
                    Enabled = true,
                    AssetIdsJson = JsonSerializer.Serialize(new[] { "asset-app-01", "asset-app-02" }),
                    UpdatedAt = DateTimeOffset.UtcNow,
                },
                new EnterpriseSubAccountEntity
                {
                    Id = "sub-mike",
                    EnterpriseId = "ent-acme",
                    DisplayName = "Mike SRE",
                    Email = "mike@acme.example",
                    Secret = BCrypt.Net.BCrypt.HashPassword("mike-pass"),
                    Enabled = true,
                    AssetIdsJson = JsonSerializer.Serialize(new[] { "asset-app-02" }),
                    UpdatedAt = DateTimeOffset.UtcNow,
                }
            );
        }

        if (!await dbContext.PersonalAccounts.AnyAsync())
        {
            dbContext.PersonalAccounts.Add(new PersonalAccountEntity
            {
                Id = "usr-amy",
                DisplayName = "Amy",
                Email = "amy@example.com",
                Secret = BCrypt.Net.BCrypt.HashPassword("amy-pass"),
                SubscriptionStatus = "active",
                PlanName = "personal",
                CustomEndpointEnabled = true,
                UpdatedAt = DateTimeOffset.UtcNow,
            });
        }

        if (!await dbContext.AdminUsers.AnyAsync())
        {
            dbContext.AdminUsers.Add(new AdminUserEntity
            {
                Id = "admin-root",
                Username = "admin",
                Password = BCrypt.Net.BCrypt.HashPassword("admin123"),
                Role = "admin",
                UpdatedAt = DateTimeOffset.UtcNow,
            });
        }

        if (!await dbContext.AiSubscriptions.AnyAsync())
        {
            dbContext.AiSubscriptions.Add(new AiSubscriptionEntity
            {
                Id = 1,
                ServiceMode = "subscription",
                PlanName = "enterprise",
                Status = "active",
                Seats = 41,
                AllowCustomEndpoint = true,
                SyncCustomEndpoint = true,
                RenewAt = DateTimeOffset.UtcNow.AddMonths(1),
            });
        }

        if (!await dbContext.AiSubscriptionPlans.AnyAsync())
        {
            dbContext.AiSubscriptionPlans.AddRange(
                new AiSubscriptionPlanEntity
                {
                    Code = "personal",
                    DisplayName = "Personal Monthly",
                    Scope = "personal",
                    PricePerSeat = 19,
                    Currency = "USD",
                    AllowCustomEndpoint = true,
                    IsActive = true,
                    Description = "Single user monthly AI subscription.",
                    UpdatedAt = DateTimeOffset.UtcNow,
                },
                new AiSubscriptionPlanEntity
                {
                    Code = "team",
                    DisplayName = "Team Monthly",
                    Scope = "enterprise",
                    PricePerSeat = 29,
                    Currency = "USD",
                    AllowCustomEndpoint = true,
                    IsActive = true,
                    Description = "Enterprise team subscription billed per seat monthly.",
                    UpdatedAt = DateTimeOffset.UtcNow,
                },
                new AiSubscriptionPlanEntity
                {
                    Code = "enterprise",
                    DisplayName = "Enterprise Managed",
                    Scope = "enterprise",
                    PricePerSeat = 49,
                    Currency = "USD",
                    AllowCustomEndpoint = true,
                    IsActive = true,
                    Description = "Managed enterprise AI subscription with centralized seat control.",
                    UpdatedAt = DateTimeOffset.UtcNow,
                },
                new AiSubscriptionPlanEntity
                {
                    Code = "custom",
                    DisplayName = "Bring Your Own Endpoint",
                    Scope = "hybrid",
                    PricePerSeat = 9,
                    Currency = "USD",
                    AllowCustomEndpoint = true,
                    IsActive = true,
                    Description = "Use your own AI endpoint and sync settings through the platform.",
                    UpdatedAt = DateTimeOffset.UtcNow,
                });
        }

        if (!await dbContext.EnterpriseSubscriptions.AnyAsync())
        {
            dbContext.EnterpriseSubscriptions.Add(new EnterpriseSubscriptionEntity
            {
                EnterpriseId = "ent-acme",
                PlanCode = "enterprise",
                Status = "active",
                SeatsPurchased = 40,
                SeatsAssigned = 2,
                RenewAt = DateTimeOffset.UtcNow.AddMonths(1),
                UpdatedAt = DateTimeOffset.UtcNow,
            });
        }

        if (!await dbContext.PersonalSubscriptions.AnyAsync())
        {
            dbContext.PersonalSubscriptions.Add(new PersonalSubscriptionEntity
            {
                AccountId = "usr-amy",
                PlanCode = "personal",
                Status = "active",
                RenewAt = DateTimeOffset.UtcNow.AddMonths(1),
                UpdatedAt = DateTimeOffset.UtcNow,
            });
        }

        if (!await dbContext.BillingInvoices.AnyAsync())
        {
            dbContext.BillingInvoices.AddRange(
                new BillingInvoiceEntity
                {
                    Id = "inv-ent-acme-2026-05",
                    TargetType = "enterprise",
                    TargetId = "ent-acme",
                    PlanCode = "enterprise",
                    Status = "open",
                    SeatCount = 40,
                    UnitPrice = 49,
                    SubscriptionAmount = 1960,
                    AiUsageAmount = 0,
                    TotalAmount = 1960,
                    Currency = "USD",
                    BillingMonth = DateTimeOffset.UtcNow.ToString("yyyy-MM"),
                    DueAt = DateTimeOffset.UtcNow.AddDays(15),
                    CreatedAt = DateTimeOffset.UtcNow.AddDays(-10),
                    UpdatedAt = DateTimeOffset.UtcNow.AddDays(-2),
                },
                new BillingInvoiceEntity
                {
                    Id = "inv-usr-amy-2026-05",
                    TargetType = "personal",
                    TargetId = "usr-amy",
                    PlanCode = "personal",
                    Status = "paid",
                    SeatCount = 1,
                    UnitPrice = 19,
                    SubscriptionAmount = 19,
                    AiUsageAmount = 0,
                    TotalAmount = 19,
                    Currency = "USD",
                    BillingMonth = DateTimeOffset.UtcNow.ToString("yyyy-MM"),
                    DueAt = DateTimeOffset.UtcNow.AddDays(10),
                    CreatedAt = DateTimeOffset.UtcNow.AddDays(-8),
                    UpdatedAt = DateTimeOffset.UtcNow.AddDays(-1),
                });
        }

        if (!await dbContext.PaymentTransactions.AnyAsync())
        {
            dbContext.PaymentTransactions.Add(new PaymentTransactionEntity
            {
                Id = "pay-usr-amy-2026-05",
                InvoiceId = "inv-usr-amy-2026-05",
                TargetType = "personal",
                TargetId = "usr-amy",
                ProviderKey = "manual",
                Amount = 19,
                Currency = "USD",
                PaymentMethod = "manual",
                Status = "completed",
                ExternalReference = "demo-personal-paid",
                Note = "Seeded demo payment",
                CheckoutUrl = string.Empty,
                ExpiresAt = null,
                PaidAt = DateTimeOffset.UtcNow.AddDays(-1),
                CreatedAt = DateTimeOffset.UtcNow.AddDays(-1),
                UpdatedAt = DateTimeOffset.UtcNow.AddDays(-1),
            });
        }

        if (!await dbContext.PaymentProviderConfigs.AnyAsync())
        {
            dbContext.PaymentProviderConfigs.Add(new PaymentProviderConfigEntity
            {
                ProviderKey = "manual",
                DisplayName = "Manual Reconciliation",
                ProviderType = "manual",
                WebhookSecret = "manual-secret",
                Enabled = true,
                MetadataJson = "{\"checkoutBaseUrl\":\"https://payments.example.com/manual-checkout\"}",
                UpdatedAt = DateTimeOffset.UtcNow,
            });
            dbContext.PaymentProviderConfigs.Add(new PaymentProviderConfigEntity
            {
                ProviderKey = "stripe-demo",
                DisplayName = "Stripe Demo",
                ProviderType = "stripe",
                WebhookSecret = "stripe-demo-secret",
                Enabled = false,
                MetadataJson = "{\"checkoutBaseUrl\":\"https://payments.example.com/stripe-checkout\",\"stripeCheckoutBaseUrl\":\"https://checkout.stripe.example/session\",\"webhookMode\":\"stripe-like\"}",
                UpdatedAt = DateTimeOffset.UtcNow,
            });
        }

        if (!await dbContext.AiUsagePricing.AnyAsync())
        {
            dbContext.AiUsagePricing.AddRange(
                new AiUsagePricingEntity
                {
                    Id = "price-openai-gpt-4o-mini",
                    Provider = "openai",
                    ModelName = "gpt-4o-mini",
                    PromptTokenRatePerMillion = 0.15,
                    CompletionTokenRatePerMillion = 0.60,
                    Currency = "USD",
                    IsActive = true,
                    UpdatedAt = DateTimeOffset.UtcNow,
                },
                new AiUsagePricingEntity
                {
                    Id = "price-anthropic-claude-3-5-sonnet",
                    Provider = "anthropic",
                    ModelName = "claude-3-5-sonnet",
                    PromptTokenRatePerMillion = 3.00,
                    CompletionTokenRatePerMillion = 15.00,
                    Currency = "USD",
                    IsActive = true,
                    UpdatedAt = DateTimeOffset.UtcNow,
                });
        }

        if (!await dbContext.AiEndpoints.AnyAsync())
        {
            dbContext.AiEndpoints.Add(new AiEndpointEntity
            {
                Id = 1,
                EndpointName = "Acme OpenAI Gateway",
                Provider = "openai",
                BaseUrl = "https://gateway.acme.example/v1",
                ApiKey = "managed-demo-key",
                ModelName = "gpt-4o-mini",
                SyncToClients = true,
                UpdatedAt = DateTimeOffset.UtcNow,
            });
        }

        await dbContext.SaveChangesAsync();
    }
}
