# Trust ASP.NET Core and Aspire Development Certificates
# Run this script as Administrator

Write-Host "Trusting ASP.NET Core HTTPS development certificate..." -ForegroundColor Cyan
dotnet dev-certs https --trust

Write-Host "`nTrusting Aspire development certificate..." -ForegroundColor Cyan
aspire certs trust

Write-Host "`nCertificate trust setup complete!" -ForegroundColor Green
Write-Host "You may need to restart your application and browser for changes to take effect." -ForegroundColor Yellow
