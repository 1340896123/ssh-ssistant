using Microsoft.EntityFrameworkCore;
using SshAssistant.AdminApi.Entities;

namespace SshAssistant.AdminApi.Data;

public sealed class AdminDbContext(DbContextOptions<AdminDbContext> options) : DbContext(options)
{
    public DbSet<EnterpriseEntity> Enterprises => Set<EnterpriseEntity>();
    public DbSet<EnterpriseSubAccountEntity> SubAccounts => Set<EnterpriseSubAccountEntity>();
    public DbSet<PersonalAccountEntity> PersonalAccounts => Set<PersonalAccountEntity>();
    public DbSet<AdminUserEntity> AdminUsers => Set<AdminUserEntity>();
    public DbSet<AssetEntity> Assets => Set<AssetEntity>();
    public DbSet<AiSubscriptionEntity> AiSubscriptions => Set<AiSubscriptionEntity>();
    public DbSet<AiSubscriptionPlanEntity> AiSubscriptionPlans => Set<AiSubscriptionPlanEntity>();
    public DbSet<EnterpriseSubscriptionEntity> EnterpriseSubscriptions => Set<EnterpriseSubscriptionEntity>();
    public DbSet<PersonalSubscriptionEntity> PersonalSubscriptions => Set<PersonalSubscriptionEntity>();
    public DbSet<BillingInvoiceEntity> BillingInvoices => Set<BillingInvoiceEntity>();
    public DbSet<BillingInvoiceLineItemEntity> BillingInvoiceLineItems => Set<BillingInvoiceLineItemEntity>();
    public DbSet<PaymentTransactionEntity> PaymentTransactions => Set<PaymentTransactionEntity>();
    public DbSet<PaymentProviderConfigEntity> PaymentProviderConfigs => Set<PaymentProviderConfigEntity>();
    public DbSet<AiUsageRecordEntity> AiUsageRecords => Set<AiUsageRecordEntity>();
    public DbSet<AiUsagePricingEntity> AiUsagePricing => Set<AiUsagePricingEntity>();
    public DbSet<AiEndpointEntity> AiEndpoints => Set<AiEndpointEntity>();
    public DbSet<ClientAccountSyncStateEntity> ClientSyncStates => Set<ClientAccountSyncStateEntity>();
    public DbSet<AuthSessionEntity> AuthSessions => Set<AuthSessionEntity>();

    protected override void OnModelCreating(ModelBuilder modelBuilder)
    {
        modelBuilder.Entity<EnterpriseEntity>().HasKey(item => item.Id);
        modelBuilder.Entity<EnterpriseSubAccountEntity>().HasKey(item => item.Id);
        modelBuilder.Entity<PersonalAccountEntity>().HasKey(item => item.Id);
        modelBuilder.Entity<AdminUserEntity>().HasKey(item => item.Id);
        modelBuilder.Entity<AssetEntity>().HasKey(item => item.Id);
        modelBuilder.Entity<AiSubscriptionEntity>().HasKey(item => item.Id);
        modelBuilder.Entity<AiSubscriptionPlanEntity>().HasKey(item => item.Code);
        modelBuilder.Entity<EnterpriseSubscriptionEntity>().HasKey(item => item.EnterpriseId);
        modelBuilder.Entity<PersonalSubscriptionEntity>().HasKey(item => item.AccountId);
        modelBuilder.Entity<BillingInvoiceEntity>().HasKey(item => item.Id);
        modelBuilder.Entity<BillingInvoiceLineItemEntity>().HasKey(item => item.Id);
        modelBuilder.Entity<PaymentTransactionEntity>().HasKey(item => item.Id);
        modelBuilder.Entity<PaymentProviderConfigEntity>().HasKey(item => item.ProviderKey);
        modelBuilder.Entity<AiUsageRecordEntity>().HasKey(item => item.Id);
        modelBuilder.Entity<AiUsagePricingEntity>().HasKey(item => item.Id);
        modelBuilder.Entity<AiEndpointEntity>().HasKey(item => item.Id);
        modelBuilder.Entity<ClientAccountSyncStateEntity>().HasKey(item => item.Id);
        modelBuilder.Entity<AuthSessionEntity>().HasKey(item => item.Token);

        modelBuilder.Entity<EnterpriseSubAccountEntity>()
            .Property(item => item.AssetIdsJson)
            .HasDefaultValue("[]");

        modelBuilder.Entity<ClientAccountSyncStateEntity>()
            .HasIndex(item => new { item.Mode, item.AccountKey })
            .IsUnique();

        modelBuilder.Entity<AuthSessionEntity>()
            .HasIndex(item => new { item.SessionType, item.SubjectId });
    }
}
