import type { Settings, AiEndpointRecord } from "../types";

export interface ResolvedAiRuntimeConfig {
  enabled: boolean;
  reason?: string;
  providerType: Settings["ai"]["providerType"];
  apiUrl: string;
  apiKey: string;
  modelName: string;
  usingCustomEndpoint: boolean;
}

/**
 * 解析当前 AI 运行时配置。
 *
 * 优先级：
 * 1. 若传入了独立的 AI 端点列表，且其中存在字段完整的默认端点 → 使用该默认端点（custom endpoint）。
 * 2. 否则按原有逻辑：平台托管运行时（云端订阅）→ 平台端点 → 不可用原因。
 *
 * 端点列表为可选参数，未传入时行为与旧版完全一致，保证向后兼容。
 */
export function resolveAiRuntimeConfig(
  settings: Settings,
  endpoints?: AiEndpointRecord[],
): ResolvedAiRuntimeConfig {
  // ---- 1. 独立端点列表中的默认端点（最高优先级）----
  const defaultEndpoint = endpoints?.find((item) => item.isDefault);
  if (defaultEndpoint) {
    const hasEndpointFields =
      Boolean(defaultEndpoint.apiUrl?.trim()) &&
      Boolean(defaultEndpoint.modelName?.trim()) &&
      Boolean(defaultEndpoint.apiKey?.trim());

    if (hasEndpointFields) {
      return {
        enabled: true,
        providerType: defaultEndpoint.providerType,
        apiUrl: defaultEndpoint.apiUrl,
        apiKey: defaultEndpoint.apiKey,
        modelName: defaultEndpoint.modelName,
        usingCustomEndpoint: true,
      };
    }

    // 默认端点存在但字段不完整
    return {
      enabled: false,
      reason: "custom-endpoint-incomplete",
      providerType: defaultEndpoint.providerType,
      apiUrl: defaultEndpoint.apiUrl,
      apiKey: defaultEndpoint.apiKey,
      modelName: defaultEndpoint.modelName,
      usingCustomEndpoint: true,
    };
  }

  // ---- 2. 回落到原有 settings 内嵌逻辑（平台托管 / 订阅）----
  const subscription = settings.ai.subscription;
  const canUsePlatformSubscription =
    subscription.status === "active" || subscription.status === "trialing";
  const isCloudManagedRuntime =
    settings.account.mode !== "local" &&
    Boolean(settings.account.accessToken?.trim()) &&
    !settings.ai.customEndpoint.useCustomEndpoint;

  const hasPlatformEndpoint =
    Boolean(settings.ai.apiUrl?.trim()) &&
    Boolean(settings.ai.modelName?.trim()) &&
    Boolean(settings.ai.apiKey?.trim());

  if (canUsePlatformSubscription && isCloudManagedRuntime && Boolean(settings.ai.modelName?.trim())) {
    return {
      enabled: true,
      providerType: settings.ai.providerType,
      apiUrl: settings.ai.apiUrl,
      apiKey: settings.ai.apiKey,
      modelName: settings.ai.modelName,
      usingCustomEndpoint: false,
    };
  }

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
