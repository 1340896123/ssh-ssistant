import type { Settings } from "../types";

export interface ResolvedAiRuntimeConfig {
  enabled: boolean;
  reason?: string;
  providerType: Settings["ai"]["providerType"];
  apiUrl: string;
  apiKey: string;
  modelName: string;
  usingCustomEndpoint: boolean;
}

export function resolveAiRuntimeConfig(settings: Settings): ResolvedAiRuntimeConfig {
  const subscription = settings.ai.subscription;
  const canUsePlatformSubscription =
    subscription.status === "active" || subscription.status === "trialing";

  const wantsCustomEndpoint = subscription.useCustomEndpoint;
  const customEndpointAllowed = subscription.allowCustomEndpoint ?? true;
  const hasCustomEndpoint =
    Boolean(settings.ai.customEndpoint.apiUrl?.trim()) &&
    Boolean(settings.ai.customEndpoint.modelName?.trim()) &&
    Boolean(settings.ai.customEndpoint.apiKey?.trim());

  if (wantsCustomEndpoint && customEndpointAllowed && hasCustomEndpoint) {
    return {
      enabled: true,
      providerType: settings.ai.customEndpoint.providerType,
      apiUrl: settings.ai.customEndpoint.apiUrl,
      apiKey: settings.ai.customEndpoint.apiKey,
      modelName: settings.ai.customEndpoint.modelName,
      usingCustomEndpoint: true,
    };
  }

  const hasPlatformEndpoint =
    Boolean(settings.ai.apiUrl?.trim()) &&
    Boolean(settings.ai.modelName?.trim()) &&
    Boolean(settings.ai.apiKey?.trim());

  if (canUsePlatformSubscription && hasPlatformEndpoint) {
    return {
      enabled: true,
      providerType: settings.ai.providerType,
      apiUrl: settings.ai.apiUrl,
      apiKey: settings.ai.apiKey,
      modelName: settings.ai.modelName,
      usingCustomEndpoint: false,
    };
  }

  if (wantsCustomEndpoint && !hasCustomEndpoint) {
    return {
      enabled: false,
      reason: "custom-endpoint-incomplete",
      providerType: settings.ai.providerType,
      apiUrl: settings.ai.apiUrl,
      apiKey: settings.ai.apiKey,
      modelName: settings.ai.modelName,
      usingCustomEndpoint: true,
    };
  }

  if (!canUsePlatformSubscription) {
    return {
      enabled: false,
      reason: "subscription-inactive",
      providerType: settings.ai.providerType,
      apiUrl: settings.ai.apiUrl,
      apiKey: settings.ai.apiKey,
      modelName: settings.ai.modelName,
      usingCustomEndpoint: false,
    };
  }

  return {
    enabled: false,
    reason: "platform-endpoint-incomplete",
    providerType: settings.ai.providerType,
    apiUrl: settings.ai.apiUrl,
    apiKey: settings.ai.apiKey,
    modelName: settings.ai.modelName,
    usingCustomEndpoint: false,
  };
}
