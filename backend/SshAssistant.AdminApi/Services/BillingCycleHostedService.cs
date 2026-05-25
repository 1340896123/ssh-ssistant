using Microsoft.EntityFrameworkCore;
using SshAssistant.AdminApi.Data;

namespace SshAssistant.AdminApi.Services;

public sealed class BillingCycleHostedService(IServiceScopeFactory scopeFactory) : BackgroundService
{
    private static readonly TimeSpan CheckInterval = TimeSpan.FromHours(24);

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        while (!stoppingToken.IsCancellationRequested)
        {
            try
            {
                using var scope = scopeFactory.CreateScope();
                var dbContext = scope.ServiceProvider.GetRequiredService<AdminDbContext>();
                var store = scope.ServiceProvider.GetRequiredService<AdminDataStore>();

                await dbContext.Database.EnsureCreatedAsync(stoppingToken);
                await store.GenerateCurrentBillingCycleAsync();
            }
            catch
            {
                // Keep the lightweight scheduler resilient; API endpoints remain available even if one cycle fails.
            }

            await Task.Delay(CheckInterval, stoppingToken);
        }
    }
}
