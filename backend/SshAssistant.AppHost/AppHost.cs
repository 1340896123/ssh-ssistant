var builder = DistributedApplication.CreateBuilder(args);

var adminApi = builder
    .AddProject<Projects.SshAssistant_AdminApi>("admin-api", launchProfileName: "http")
    .WithHttpHealthCheck("/health")
    .WithExternalHttpEndpoints();

var adminWeb = builder
    .AddViteApp("admin-web", "../../admin")
    .WithReference(adminApi)
    .WaitFor(adminApi);

adminApi.PublishWithContainerFiles(adminWeb, "wwwroot");

builder.Build().Run();
