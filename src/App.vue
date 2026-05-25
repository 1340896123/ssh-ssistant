<script setup lang="ts">
import {
  computed,
  onBeforeUpdate,
  onMounted,
  onUnmounted,
  reactive,
  ref,
  watch,
} from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrent } from "@tauri-apps/plugin-deep-link";
import AssetCenter from "./components/AssetCenter.vue";
import LoginGateway from "./components/LoginGateway.vue";
import ConnectionModal from "./components/ConnectionModal.vue";
import TunnelModal from "./components/TunnelModal.vue";
import TunnelPanel from "./components/TunnelPanel.vue";
import OpsWorkbench from "./components/OpsWorkbench.vue";
import SessionTabs from "./components/SessionTabs.vue";
import SessionsWorkbenchPanel from "./components/SessionsWorkbenchPanel.vue";
import TerminalTabArea from "./components/TerminalTabArea.vue";
import FileManager from "./components/FileManager.vue";
import AIAssistant from "./components/AIAssistant.vue";
import SettingsModal from "./components/SettingsModal.vue";
import NotificationModal from "./components/NotificationModal.vue";
import { useSessionStore } from "./stores/sessions";
import { useAssetStore } from "./stores/assets";
import { useSettingsStore } from "./stores/settings";
import { useNotificationStore } from "./stores/notifications";
import { useTransferStore } from "./stores/transfers";
import { useI18n } from "./composables/useI18n";
import type { AccessEndpoint, CredentialRef, HostAsset } from "./types";
import {
  Bot,
  Cable,
  ClipboardCheck,
  Focus,
  FolderOpen,
  Monitor,
  PanelLeftClose,
  PanelLeftOpen,
  PanelRightClose,
  PanelRightOpen,
  Plus,
  RefreshCw,
  Rows3,
  Settings,
} from "lucide-vue-next";

type ActivityId = "connections" | "tunnels" | "ops" | "sessions";
type ContextTab = "ai" | "files";
type ResizeTarget = "resource" | "context";

interface AiAssistantRef {
  addContextPaths: (paths: { path: string; isDir: boolean }[]) => void;
}

interface WorkspaceLayoutState {
  activeActivity: ActivityId;
  resourcePaneWidth: number;
  contextPaneWidth: number;
  isResourcePaneCollapsed: boolean;
  isContextPaneCollapsed: boolean;
  isFocusMode: boolean;
}

interface DiskInfo {
  size: string;
  used: string;
  avail: string;
  percent: string;
  mount: string;
  filesystem: string;
}

interface ProcessInfo {
  pid: string;
  command: string;
  cpu: string;
  memory: string;
  memoryPercent: string;
}

interface CpuInfo {
  usage: string;
  topProcesses: ProcessInfo[];
}

interface MemoryInfo {
  usage: string;
  total: string;
  used: string;
  available: string;
  topProcesses: ProcessInfo[];
}

interface SessionStats {
  uptime: string;
  disk: DiskInfo | null;
  mounts: DiskInfo[];
  ip: string;
  cpu: CpuInfo | null;
  memory: MemoryInfo | null;
}

const sessionStore = useSessionStore();
const assetStore = useAssetStore();
const settingsStore = useSettingsStore();
const notificationStore = useNotificationStore();
const transferStore = useTransferStore();
const { t } = useI18n();
const appReady = ref(false);
const requiresLogin = ref(false);

const WORKSPACE_LAYOUT_STORAGE_KEY = "appWorkspaceLayout";
const RESOURCE_PANE_MIN = 260;
const RESOURCE_PANE_MAX = 420;
const CONTEXT_PANE_MIN = 320;
const CONTEXT_PANE_MAX = 520;
const DEFAULT_RESOURCE_PANE_WIDTH = 300;
const DEFAULT_CONTEXT_PANE_WIDTH = 380;
const RESOURCE_DRAWER_BREAKPOINT = 1280;
const CONTEXT_DRAWER_BREAKPOINT = 980;
const DEFAULT_WINDOW_WIDTH = 1440;

const showConnectionModal = ref(false);
const showSettingsModal = ref(false);
const editingAsset = ref<HostAsset | null>(null);
const editingAccessEndpoint = ref<AccessEndpoint | null>(null);
const editingCredentialRef = ref<CredentialRef | null>(null);
const showTunnelModal = ref(false);
const tunnelAsset = ref<HostAsset | null>(null);
const clockTimer = ref<number | null>(null);
const statusTimer = ref<number | null>(null);

const windowWidth = ref(
  typeof window === "undefined" ? DEFAULT_WINDOW_WIDTH : window.innerWidth
);
const shellViewportRef = ref<HTMLElement | null>(null);
const isResizing = ref<ResizeTarget | null>(null);

const resourceDrawerOpen = ref(false);
const contextDrawerOpen = ref(false);

const terminalTabAreaRefs = ref<any[]>([]);
const aiAssistantRefs = ref<any[]>([]);
const terminalContext = ref("");

const sessionContextTabs = reactive<Record<string, ContextTab>>({});
const sessionSelectionState = reactive<
  Record<string, { count: number; targetLabel: string }>
>({});
const sessionAiContextCounts = reactive<Record<string, number>>({});

const now = ref(Date.now());
const sessionStatus = ref<Record<string, SessionStats>>({});
const isRefreshingSessionStatus = ref(false);

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function deriveLegacyContextPaneWidth(legacyAiWidth?: unknown) {
  const aiWidthPercent = Number(legacyAiWidth);
  if (!Number.isFinite(aiWidthPercent)) {
    return DEFAULT_CONTEXT_PANE_WIDTH;
  }

  const viewportWidth =
    typeof window === "undefined" ? DEFAULT_WINDOW_WIDTH : window.innerWidth;
  return clamp(
    Math.round((viewportWidth * aiWidthPercent) / 100),
    CONTEXT_PANE_MIN,
    CONTEXT_PANE_MAX
  );
}

function parseWorkspaceLayoutState(): WorkspaceLayoutState {
  const defaultState: WorkspaceLayoutState = {
    activeActivity: "connections",
    resourcePaneWidth: DEFAULT_RESOURCE_PANE_WIDTH,
    contextPaneWidth: DEFAULT_CONTEXT_PANE_WIDTH,
    isResourcePaneCollapsed: false,
    isContextPaneCollapsed: false,
    isFocusMode: false,
  };

  if (typeof localStorage === "undefined") {
    return defaultState;
  }

  const raw = localStorage.getItem(WORKSPACE_LAYOUT_STORAGE_KEY);
  if (!raw) {
    return defaultState;
  }

  try {
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    const legacyActivity =
      parsed.sidebarTab === "tunnels" ? "tunnels" : "connections";
    const derivedContextWidth = deriveLegacyContextPaneWidth(parsed.aiWidth);
    const showAuxiliaryPanel =
      typeof parsed.showAuxiliaryPanel === "boolean"
        ? parsed.showAuxiliaryPanel
        : undefined;

    return {
      activeActivity:
        parsed.activeActivity === "tunnels" || parsed.activeActivity === "sessions"
          ? parsed.activeActivity
          : legacyActivity,
      resourcePaneWidth: clamp(
        Number(
          parsed.resourcePaneWidth ?? parsed.sidebarWidth ?? DEFAULT_RESOURCE_PANE_WIDTH
        ),
        RESOURCE_PANE_MIN,
        RESOURCE_PANE_MAX
      ),
      contextPaneWidth: clamp(
        Number(parsed.contextPaneWidth ?? derivedContextWidth),
        CONTEXT_PANE_MIN,
        CONTEXT_PANE_MAX
      ),
      isResourcePaneCollapsed: Boolean(
        parsed.isResourcePaneCollapsed ?? parsed.isSidebarCollapsed ?? false
      ),
      isContextPaneCollapsed: Boolean(
        parsed.isContextPaneCollapsed ??
          (showAuxiliaryPanel === undefined ? false : !showAuxiliaryPanel)
      ),
      isFocusMode: Boolean(
        parsed.isFocusMode ?? (parsed.workspaceMode === "terminalFocus")
      ),
    };
  } catch {
    return defaultState;
  }
}

const initialLayoutState = parseWorkspaceLayoutState();

const activeActivity = ref<ActivityId>(initialLayoutState.activeActivity);
const resourcePaneWidth = ref(initialLayoutState.resourcePaneWidth);
const contextPaneWidth = ref(initialLayoutState.contextPaneWidth);
const isResourcePaneCollapsed = ref(initialLayoutState.isResourcePaneCollapsed);
const isContextPaneCollapsed = ref(initialLayoutState.isContextPaneCollapsed);
const isFocusMode = ref(initialLayoutState.isFocusMode);

const activeSession = computed(() => sessionStore.activeSession);
const activeAsset = computed(() =>
  assetStore.assets.find((asset) => asset.id === activeSession.value?.assetId)
);
const activeAssetRisk = computed(
  () => activeSession.value?.riskLevel ?? activeAsset.value?.criticality ?? null
);
const activeAssetHealth = computed(
  () => activeSession.value?.healthSummary ?? activeAsset.value?.healthSummary ?? null
);
const activeWorkspace = computed(() => activeSession.value?.activeWorkspace);
const activeSelection = computed(() =>
  activeSession.value
    ? sessionSelectionState[activeSession.value.id] ?? { count: 0, targetLabel: "" }
    : { count: 0, targetLabel: "" }
);
const activeAiContextCount = computed(() =>
  activeSession.value ? sessionAiContextCounts[activeSession.value.id] ?? 0 : 0
);
const activeContextTab = computed<ContextTab>(() => {
  if (!activeSession.value) return "ai";
  return sessionContextTabs[activeSession.value.id] ?? "ai";
});

const activeSessionTransferItems = computed(() => {
  if (!activeSession.value) return [];
  return transferStore.items.filter(
    (item) => item.sessionId === activeSession.value?.id
  );
});
const activeTransferSummary = computed(() => {
  const items = activeSessionTransferItems.value;
  const running = items.filter((item) => item.status === "running").length;
  const pending = items.filter((item) => item.status === "pending").length;
  const failed = items.filter((item) => item.status === "error").length;
  return {
    total: items.length,
    running,
    pending,
    failed,
  };
});

const isCompactResourceMode = computed(
  () => windowWidth.value < RESOURCE_DRAWER_BREAKPOINT
);
const isCompactContextMode = computed(
  () => windowWidth.value < CONTEXT_DRAWER_BREAKPOINT
);
const shouldShowInlineResourcePane = computed(
  () =>
    !isFocusMode.value &&
    !isCompactResourceMode.value &&
    !isResourcePaneCollapsed.value
);
const shouldShowInlineContextPane = computed(
  () =>
    !isFocusMode.value &&
    !isCompactContextMode.value &&
    !isContextPaneCollapsed.value
);
const shouldShowResourceDrawer = computed(
  () =>
    !isFocusMode.value &&
    isCompactResourceMode.value &&
    resourceDrawerOpen.value
);
const shouldShowContextDrawer = computed(
  () => !isFocusMode.value && isCompactContextMode.value && contextDrawerOpen.value
);
const shouldShowAnyDrawer = computed(
  () => shouldShowResourceDrawer.value || shouldShowContextDrawer.value
);

const activeResourcePaneMeta = computed(() => {
  if (activeActivity.value === "tunnels") {
    return {
      title: t("app.tunnels"),
      subtitle: t("workbench.tunnelsSubtitle"),
    };
  }

  if (activeActivity.value === "ops") {
    return {
      title: t("workbench.opsTitle"),
      subtitle: t("workbench.opsSubtitle"),
    };
  }

  if (activeActivity.value === "sessions") {
    return {
      title: t("workbench.sessionsTitle"),
      subtitle: t("workbench.sessionsSubtitle"),
    };
  }

  return {
    title: t("app.connections"),
    subtitle: t("workbench.connectionsSubtitle"),
  };
});

const activeSessionDuration = computed(() => {
  if (!activeSession.value?.connectedAt) return "0s";
  const diffMs = now.value - activeSession.value.connectedAt;
  if (diffMs <= 0) return "0s";
  const totalSeconds = Math.floor(diffMs / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = Math.floor(totalSeconds % 60);
  const parts: string[] = [];
  if (hours > 0) parts.push(`${hours}h`);
  if (minutes > 0) parts.push(`${minutes}m`);
  parts.push(`${seconds}s`);
  return parts.join(" ");
});

function persistWorkspaceLayoutState() {
  if (typeof localStorage === "undefined") return;

  const layoutState: WorkspaceLayoutState = {
    activeActivity: activeActivity.value,
    resourcePaneWidth: Math.round(resourcePaneWidth.value),
    contextPaneWidth: Math.round(contextPaneWidth.value),
    isResourcePaneCollapsed: isResourcePaneCollapsed.value,
    isContextPaneCollapsed: isContextPaneCollapsed.value,
    isFocusMode: isFocusMode.value,
  };

  localStorage.setItem(WORKSPACE_LAYOUT_STORAGE_KEY, JSON.stringify(layoutState));
}

watch(
  [
    activeActivity,
    resourcePaneWidth,
    contextPaneWidth,
    isResourcePaneCollapsed,
    isContextPaneCollapsed,
    isFocusMode,
  ],
  () => {
    persistWorkspaceLayoutState();
  }
);

watch(isCompactResourceMode, (isCompact) => {
  if (!isCompact) {
    resourceDrawerOpen.value = false;
  }
});

watch(isCompactContextMode, (isCompact) => {
  if (!isCompact) {
    contextDrawerOpen.value = false;
  }
});

watch(
  () => sessionStore.sessions.map((session) => session.id),
  (sessionIds) => {
    const knownIds = new Set(sessionIds);
    for (const id of Object.keys(sessionContextTabs)) {
      if (!knownIds.has(id)) delete sessionContextTabs[id];
    }
    for (const id of Object.keys(sessionSelectionState)) {
      if (!knownIds.has(id)) delete sessionSelectionState[id];
    }
    for (const id of Object.keys(sessionAiContextCounts)) {
      if (!knownIds.has(id)) delete sessionAiContextCounts[id];
    }

    for (const id of sessionIds) {
      if (!sessionContextTabs[id]) {
        sessionContextTabs[id] = "ai";
      }
      if (!sessionSelectionState[id]) {
        sessionSelectionState[id] = { count: 0, targetLabel: "" };
      }
      if (typeof sessionAiContextCounts[id] !== "number") {
        sessionAiContextCounts[id] = 0;
      }
    }
  },
  { immediate: true }
);

watch(
  () => activeSession.value?.id,
  (sessionId) => {
    if (!sessionId) return;
    if (!sessionContextTabs[sessionId]) {
      sessionContextTabs[sessionId] = "ai";
    }
    const tab = sessionContextTabs[sessionId];
    sessionStore.setActiveTab(tab);
    void refreshActiveSessionStatus();
  },
  { immediate: true }
);

onBeforeUpdate(() => {
  terminalTabAreaRefs.value = [];
  aiAssistantRefs.value = [];
});

function getActiveTerminalView() {
  if (!activeSession.value) return null;
  const activeIndex = sessionStore.sessions.findIndex(
    (session) => session.id === activeSession.value?.id
  );
  if (activeIndex === -1) return null;
  const tabArea = terminalTabAreaRefs.value[activeIndex];
  return tabArea?.terminalView || null;
}

function updateTerminalContext() {
  const activeTerminal = getActiveTerminalView();
  if (activeTerminal && typeof activeTerminal.getContent === "function") {
    terminalContext.value = activeTerminal.getContent();
  } else {
    terminalContext.value = "";
  }
}

function setWindowWidth() {
  windowWidth.value =
    typeof window === "undefined" ? DEFAULT_WINDOW_WIDTH : window.innerWidth;
}

async function handleSaveConnection(payload: {
  asset: HostAsset;
  endpoint: AccessEndpoint;
  credentialRef?: CredentialRef | null;
}) {
  try {
    if (payload.asset.id) {
      await assetStore.updateAsset(
        payload.asset,
        payload.endpoint,
        payload.credentialRef ?? null,
      );
    } else {
      await assetStore.addAsset(
        payload.asset,
        payload.endpoint,
        payload.credentialRef ?? null,
      );
    }
    showConnectionModal.value = false;
    editingAsset.value = null;
    editingAccessEndpoint.value = null;
    editingCredentialRef.value = null;
  } catch (error) {
    console.error("Failed to save asset", error);
    notificationStore.error(t("app.assetSaveFailed"));
  }
}

function openNewConnectionModal() {
  editingAsset.value = null;
  editingAccessEndpoint.value = null;
  editingCredentialRef.value = null;
  showConnectionModal.value = true;
}

function openEditConnectionModal(asset: HostAsset | null) {
  editingAsset.value = asset;
  editingAccessEndpoint.value = asset?.id
    ? assetStore.defaultAccessEndpointForAsset(asset.id)
    : null;
  editingCredentialRef.value = editingAccessEndpoint.value
    ? assetStore.credentialRefById(editingAccessEndpoint.value.credentialRefId)
    : null;
  showConnectionModal.value = true;
}

function openTunnelModal(asset: HostAsset) {
  tunnelAsset.value = asset;
  showTunnelModal.value = true;
}

function ensureResourcePaneVisible() {
  if (isFocusMode.value) {
    isFocusMode.value = false;
  }
  if (isCompactResourceMode.value) {
    resourceDrawerOpen.value = true;
  } else {
    isResourcePaneCollapsed.value = false;
  }
}

function ensureContextPaneVisible() {
  if (isFocusMode.value) {
    isFocusMode.value = false;
  }
  if (isCompactContextMode.value) {
    contextDrawerOpen.value = true;
  } else {
    isContextPaneCollapsed.value = false;
  }
}

function activateActivity(activity: ActivityId) {
  activeActivity.value = activity;
  ensureResourcePaneVisible();
}

function toggleResourcePane() {
  if (isCompactResourceMode.value) {
    if (isFocusMode.value) {
      isFocusMode.value = false;
    }
    resourceDrawerOpen.value = !resourceDrawerOpen.value;
    return;
  }
  isResourcePaneCollapsed.value = !isResourcePaneCollapsed.value;
}

function toggleContextPane() {
  if (isCompactContextMode.value) {
    if (isFocusMode.value) {
      isFocusMode.value = false;
    }
    contextDrawerOpen.value = !contextDrawerOpen.value;
    return;
  }
  isContextPaneCollapsed.value = !isContextPaneCollapsed.value;
}

function setContextTab(tab: ContextTab) {
  if (!activeSession.value) return;
  ensureContextPaneVisible();
  sessionContextTabs[activeSession.value.id] = tab;
  sessionStore.setActiveTab(tab);
}

function toggleFocusMode() {
  isFocusMode.value = !isFocusMode.value;
  if (isFocusMode.value) {
    resourceDrawerOpen.value = false;
    contextDrawerOpen.value = false;
  }
}

function openFileEditor(sessionId: string, filePath: string, fileName: string) {
  const sessionIndex = sessionStore.sessions.findIndex(
    (session) => session.id === sessionId
  );
  if (sessionIndex !== -1 && terminalTabAreaRefs.value[sessionIndex]) {
    sessionStore.setActiveSession(sessionId);
    sessionStore.setActiveTab("terminal");
    terminalTabAreaRefs.value[sessionIndex].openFileEditor(filePath, fileName);
  }
}

function switchTerminalToPath(sessionId: string, path: string) {
  const sessionIndex = sessionStore.sessions.findIndex(
    (session) => session.id === sessionId
  );
  if (sessionIndex !== -1 && terminalTabAreaRefs.value[sessionIndex]) {
    sessionStore.setActiveSession(sessionId);
    sessionStore.setActiveTab("terminal");
    terminalTabAreaRefs.value[sessionIndex].switchToPath(path);
  }
}

function addAiContextPaths(
  sessionId: string,
  paths: { path: string; isDir: boolean }[]
) {
  const sessionIndex = sessionStore.sessions.findIndex(
    (session) => session.id === sessionId
  );
  const aiAssistant =
    sessionIndex !== -1
      ? (aiAssistantRefs.value[sessionIndex] as AiAssistantRef | undefined)
      : undefined;

  sessionStore.setActiveSession(sessionId);
  sessionContextTabs[sessionId] = "ai";
  sessionStore.setActiveTab("ai");
  ensureContextPaneVisible();
  aiAssistant?.addContextPaths(paths);
}

function handleFilePathChange(sessionId: string, path: string) {
  const session = sessionStore.sessions.find((item) => item.id === sessionId);
  if (session) {
    session.currentPath = path;
  }
}

function handleFileSelectionChange(
  sessionId: string,
  payload: { count: number; targetLabel: string }
) {
  sessionSelectionState[sessionId] = payload;
}

function handleContextMetaChange(
  sessionId: string,
  payload: { count: number }
) {
  sessionAiContextCounts[sessionId] = payload.count;
}

async function refreshActiveSessionStatus() {
  if (!activeSession.value || activeSession.value.status !== "connected") return;
  if (isRefreshingSessionStatus.value) return;

  const id = activeSession.value.id;
  isRefreshingSessionStatus.value = true;

  try {
    const stats = await invoke<SessionStats>("get_remote_system_status", { id });
    sessionStatus.value = {
      ...sessionStatus.value,
      [id]: stats,
    };
  } catch (error) {
    console.error(`System monitoring failed for ${id}:`, error);
  } finally {
    isRefreshingSessionStatus.value = false;
  }
}

function startResize(target: ResizeTarget) {
  isResizing.value = target;
  document.body.style.cursor = "col-resize";
  document.body.style.userSelect = "none";
}

function handleMouseMove(event: MouseEvent) {
  if (!isResizing.value || !shellViewportRef.value) return;

  const rect = shellViewportRef.value.getBoundingClientRect();
  if (isResizing.value === "resource") {
    const nextWidth = event.clientX - rect.left - 56;
    resourcePaneWidth.value = clamp(
      Math.round(nextWidth),
      RESOURCE_PANE_MIN,
      RESOURCE_PANE_MAX
    );
    return;
  }

  const nextWidth = rect.right - event.clientX;
  contextPaneWidth.value = clamp(
    Math.round(nextWidth),
    CONTEXT_PANE_MIN,
    CONTEXT_PANE_MAX
  );
}

function handleMouseUp() {
  if (!isResizing.value) return;
  isResizing.value = null;
  document.body.style.cursor = "";
  document.body.style.userSelect = "";
}

function closeDrawers() {
  resourceDrawerOpen.value = false;
  contextDrawerOpen.value = false;
}

function handleGlobalKeydown(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f") {
    event.preventDefault();
  }

  if (event.altKey && !event.shiftKey && !event.ctrlKey && !event.metaKey) {
    const key = event.key.toLowerCase();

    if (key === "1") {
      event.preventDefault();
      toggleFocusMode();
      return;
    }

    if (key === "2") {
      event.preventDefault();
      toggleContextPane();
      return;
    }

    if (key === "3") {
      event.preventDefault();
      setContextTab("ai");
      return;
    }

    if (key === "4") {
      event.preventDefault();
      setContextTab("files");
    }
  }
}

function openConnectionsWorkbench() {
  activateActivity("connections");
}

function openOpsWorkbench() {
  activateActivity("ops");
}

async function reconcilePendingCheckoutStatus() {
  const result = await settingsStore.reconcilePendingCheckoutSession();
  if (!result) {
    return;
  }

  if (result.settled) {
    notificationStore.success("Payment received and invoice status updated.");
    return;
  }

  if (result.expired) {
    notificationStore.warning("Payment session expired. Please create a new payment link.");
    return;
  }

  if (result.invoice.status === "overdue") {
    notificationStore.warning("Invoice is overdue and still awaiting payment.");
  }
}

function handleWindowFocus() {
  void reconcilePendingCheckoutStatus().catch((error) => {
    console.warn("Pending checkout reconciliation on focus skipped:", error);
  });
}

async function handlePaymentDeepLink(url: string) {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch (error) {
    console.warn("Invalid deep link URL:", url, error);
    return;
  }

  if (parsed.protocol !== "sshstar:" || parsed.hostname !== "billing") {
    return;
  }

  await reconcilePendingCheckoutStatus().catch((error) => {
    console.warn("Pending checkout reconciliation after deep link skipped:", error);
  });

  if (parsed.pathname === "/success") {
    notificationStore.success(t("settings.paymentReturnSuccess"));
  } else if (parsed.pathname === "/cancel") {
    notificationStore.info(t("settings.paymentReturnCancelled"));
  }
}

onMounted(async () => {
  (window as any).MonacoEnvironment = {
    getWorkerUrl: function (_moduleId: string, label: string) {
      if (label === "json") return "./json.worker.js";
      if (label === "css" || label === "scss" || label === "less") {
        return "./css.worker.js";
      }
      if (label === "html" || label === "handlebars" || label === "razor") {
        return "./html.worker.js";
      }
      if (label === "typescript" || label === "javascript") {
        return "./ts.worker.js";
      }
      return "./editor.worker.js";
    },
  };

  await settingsStore.loadSettings();
  const hasCloudIdentity =
    settingsStore.account.mode === "local" ||
    Boolean(
      settingsStore.account.accessToken &&
        (settingsStore.account.userId ||
          settingsStore.account.subAccountId ||
          settingsStore.account.email),
    );

  if (!hasCloudIdentity) {
    requiresLogin.value = true;
    appReady.value = true;
    return;
  }

  if (settingsStore.isCloudSessionExpired()) {
    if (!settingsStore.isCloudRefreshExpired()) {
      await settingsStore.refreshCloudSession().catch(async () => {
        await settingsStore.logoutFromCloud();
      });
    } else {
      await settingsStore.logoutFromCloud();
      requiresLogin.value = true;
      appReady.value = true;
      return;
    }
  }

  await settingsStore.pullCloudState().catch((error) => {
    console.warn("Cloud state pull skipped:", error);
  });
  await reconcilePendingCheckoutStatus().catch((error) => {
    console.warn("Pending checkout reconciliation skipped:", error);
  });
  await getCurrent()
    .then(async (urls: string[] | null) => {
      for (const url of urls ?? []) {
        await handlePaymentDeepLink(url);
      }
    })
    .catch((error: unknown) => {
      console.warn("Deep link current URL fetch skipped:", error);
    });
  await assetStore.loadAssets();
  if (settingsStore.sync.enabled) {
    await assetStore.pullAssetsFromCloud(
      settingsStore.sync.endpointUrl || "http://localhost:5047",
      settingsStore.account.mode,
      settingsStore.account.userId || settingsStore.account.subAccountId || "local-workspace",
      settingsStore.account.accessToken || "",
    ).catch((error) => {
      console.warn("Cloud asset pull skipped:", error);
    });
  }
  await sessionStore.setupEventListeners();
  await transferStore.initListeners();

  setWindowWidth();
  window.addEventListener("resize", setWindowWidth);
  window.addEventListener("mousemove", handleMouseMove);
  window.addEventListener("mouseup", handleMouseUp);
  window.addEventListener("keydown", handleGlobalKeydown);
  window.addEventListener("focus", handleWindowFocus);

  clockTimer.value = window.setInterval(() => {
    now.value = Date.now();
  }, 1000);

  statusTimer.value = window.setInterval(() => {
    void refreshActiveSessionStatus();
  }, 3000);

  appReady.value = true;
});

async function handleAuthenticated() {
  requiresLogin.value = false;
  await assetStore.loadAssets();
  if (settingsStore.sync.enabled) {
    await assetStore.pullAssetsFromCloud(
      settingsStore.sync.endpointUrl || "http://localhost:5047",
      settingsStore.account.mode,
      settingsStore.account.userId || settingsStore.account.subAccountId || "local-workspace",
      settingsStore.account.accessToken || "",
    );
  }
  if (sessionStore._unlistenFns.length === 0) {
    await sessionStore.setupEventListeners();
  }
  appReady.value = true;
}

async function handleSwitchAccount() {
  await settingsStore.logoutFromCloud();
  requiresLogin.value = true;
  appReady.value = true;
}

onUnmounted(() => {
  window.removeEventListener("resize", setWindowWidth);
  window.removeEventListener("mousemove", handleMouseMove);
  window.removeEventListener("mouseup", handleMouseUp);
  window.removeEventListener("keydown", handleGlobalKeydown);
  window.removeEventListener("focus", handleWindowFocus);
  sessionStore.cleanupEventListeners();
  if (clockTimer.value !== null) {
    clearInterval(clockTimer.value);
    clockTimer.value = null;
  }
  if (statusTimer.value !== null) {
    clearInterval(statusTimer.value);
    statusTimer.value = null;
  }
  document.body.style.cursor = "";
  document.body.style.userSelect = "";
});
</script>

<template>
  <div v-if="!appReady" class="flex min-h-screen items-center justify-center bg-bg-primary text-text-primary">
    <div class="rounded-2xl border border-border-primary bg-bg-secondary px-6 py-4 text-sm">
      {{ t("app.loadingStatus") }}
    </div>
  </div>

  <LoginGateway
    v-else-if="requiresLogin"
    @authenticated="handleAuthenticated"
  />

  <div v-else class="h-screen w-screen overflow-hidden bg-bg-primary text-text-primary">
    <div ref="shellViewportRef" class="flex h-full w-full min-w-0 overflow-hidden">
      <aside
        class="flex h-full w-14 shrink-0 flex-col border-r border-border-primary bg-bg-secondary"
      >
        <div class="flex h-14 items-center justify-center border-b border-border-primary">
          <div
            class="flex h-9 w-9 items-center justify-center rounded-xl border border-border-primary bg-bg-elevated text-sm font-semibold text-text-primary"
          >
            SS
          </div>
        </div>

        <div class="flex flex-1 flex-col items-center gap-2 px-2 py-3">
          <button
            class="flex h-10 w-10 items-center justify-center rounded-xl transition-colors"
            :class="
              activeActivity === 'connections'
                ? 'bg-accent/15 text-accent'
                : 'text-text-secondary hover:bg-bg-elevated hover:text-text-primary'
            "
            :title="t('app.connections')"
            @click="activateActivity('connections')"
          >
            <Monitor class="h-[18px] w-[18px]" />
          </button>
          <button
            class="flex h-10 w-10 items-center justify-center rounded-xl transition-colors"
            :class="
              activeActivity === 'tunnels'
                ? 'bg-accent/15 text-accent'
                : 'text-text-secondary hover:bg-bg-elevated hover:text-text-primary'
            "
            :title="t('app.tunnels')"
            @click="activateActivity('tunnels')"
          >
            <Cable class="h-[18px] w-[18px]" />
          </button>
          <button
            class="flex h-10 w-10 items-center justify-center rounded-xl transition-colors"
            :class="
              activeActivity === 'ops'
                ? 'bg-accent/15 text-accent'
                : 'text-text-secondary hover:bg-bg-elevated hover:text-text-primary'
            "
            :title="t('workbench.opsTitle')"
            @click="activateActivity('ops')"
          >
            <ClipboardCheck class="h-[18px] w-[18px]" />
          </button>
          <button
            class="flex h-10 w-10 items-center justify-center rounded-xl transition-colors"
            :class="
              activeActivity === 'sessions'
                ? 'bg-accent/15 text-accent'
                : 'text-text-secondary hover:bg-bg-elevated hover:text-text-primary'
            "
            :title="t('workbench.sessionsTitle')"
            @click="activateActivity('sessions')"
          >
            <Rows3 class="h-[18px] w-[18px]" />
          </button>
        </div>

        <div class="flex flex-col items-center gap-2 border-t border-border-primary px-2 py-3">
          <button
            class="flex h-10 w-10 items-center justify-center rounded-xl text-text-secondary transition-colors hover:bg-bg-elevated hover:text-text-primary"
            :title="t('app.settings')"
            @click="showSettingsModal = true"
          >
            <Settings class="h-[18px] w-[18px]" />
          </button>
          <button
            class="rounded-xl border border-border-primary px-3 py-2 text-xs text-text-secondary transition-colors hover:bg-bg-elevated hover:text-text-primary"
            @click="handleSwitchAccount"
          >
            Switch
          </button>
        </div>
      </aside>

      <div class="relative flex min-h-0 min-w-0 flex-1 overflow-hidden">
        <aside
          v-if="shouldShowInlineResourcePane"
          class="flex h-full min-w-0 shrink-0 flex-col overflow-hidden border-r border-border-primary bg-bg-secondary"
          :style="{ width: resourcePaneWidth + 'px' }"
        >
          <div
            v-if="activeActivity !== 'sessions'"
            class="flex h-14 items-center justify-between border-b border-border-primary px-4"
          >
            <div>
              <div class="text-sm font-semibold text-text-primary">
                {{ activeResourcePaneMeta.title }}
              </div>
              <div class="mt-0.5 text-xs text-text-secondary">
                {{ activeResourcePaneMeta.subtitle }}
              </div>
            </div>
            <button
              class="rounded-md p-1.5 text-text-secondary hover:bg-bg-elevated hover:text-text-primary"
              :title="t('workbench.collapseResourcePane')"
              @click="toggleResourcePane"
            >
              <PanelLeftClose class="h-4 w-4" />
            </button>
          </div>

          <div class="min-h-0 flex-1">
            <AssetCenter
              v-if="activeActivity === 'connections'"
              @edit="openEditConnectionModal"
              @tunnels="openTunnelModal"
            />
            <div
              v-else-if="activeActivity === 'tunnels'"
              class="h-full overflow-y-auto px-4 py-4"
            >
              <TunnelPanel @manage="openTunnelModal" />
            </div>
            <OpsWorkbench
              v-else-if="activeActivity === 'ops'"
              :active-asset="activeAsset"
            />
            <SessionsWorkbenchPanel
              v-else
              @new-connection="openNewConnectionModal"
            />
          </div>
        </aside>

        <div
          v-if="shouldShowInlineResourcePane"
          class="w-1 shrink-0 cursor-col-resize bg-bg-primary transition-colors hover:bg-accent"
          @mousedown.prevent="startResize('resource')"
        ></div>

        <main class="flex min-h-0 min-w-0 flex-1 flex-col bg-bg-primary">
          <div class="flex h-10 items-center justify-between border-b border-border-primary px-3">
            <div class="flex items-center gap-2">
              <button
                class="rounded-md p-1.5 text-text-secondary transition-colors hover:bg-bg-secondary hover:text-text-primary"
                :title="
                  shouldShowInlineResourcePane || shouldShowResourceDrawer
                    ? t('workbench.collapseResourcePane')
                    : t('workbench.expandResourcePane')
                "
                @click="toggleResourcePane"
              >
                <component
                  :is="
                    shouldShowInlineResourcePane || shouldShowResourceDrawer
                      ? PanelLeftClose
                      : PanelLeftOpen
                  "
                  class="h-4 w-4"
                />
              </button>
              <button
                class="rounded-md p-1.5 text-text-secondary transition-colors hover:bg-bg-secondary hover:text-text-primary"
                :class="isFocusMode ? 'bg-bg-secondary text-accent' : ''"
                :title="t('workbench.focusMode')"
                @click="toggleFocusMode"
              >
                <Focus class="h-4 w-4" />
              </button>
              <button
                class="rounded-md p-1.5 text-text-secondary transition-colors hover:bg-bg-secondary hover:text-text-primary"
                :class="
                  shouldShowInlineContextPane || shouldShowContextDrawer
                    ? 'bg-bg-secondary text-accent'
                    : ''
                "
                :title="
                  shouldShowInlineContextPane || shouldShowContextDrawer
                    ? t('workbench.hideContextPane')
                    : t('workbench.showContextPane')
                "
                @click="toggleContextPane"
              >
                <component
                  :is="
                    shouldShowInlineContextPane || shouldShowContextDrawer
                      ? PanelRightClose
                      : PanelRightOpen
                  "
                  class="h-4 w-4"
                />
              </button>
            </div>

            <div class="min-w-0 flex-1 px-4 text-center">
              <div class="truncate text-sm font-medium text-text-primary">
                {{
                  activeSession
                    ? activeSession.assetName
                    : t("workbench.noActiveSession")
                }}
              </div>
            </div>

            <div class="flex items-center gap-2">
              <button
                class="inline-flex items-center gap-1.5 rounded-md border border-border-primary bg-bg-secondary px-3 py-1.5 text-xs text-text-primary transition-colors hover:bg-bg-elevated"
                @click="openConnectionsWorkbench"
              >
                <Monitor class="h-3.5 w-3.5" />
                <span>Open Asset Center</span>
              </button>
              <button
                class="inline-flex items-center gap-1.5 rounded-md bg-accent px-3 py-1.5 text-xs text-white transition-opacity hover:opacity-90"
                @click="openNewConnectionModal"
              >
                <Plus class="h-3.5 w-3.5" />
                <span>New Asset</span>
              </button>
            </div>
          </div>

          <div class="h-10 border-b border-border-primary bg-bg-secondary">
            <SessionTabs />
          </div>

          <div class="relative min-h-0 flex-1 overflow-hidden">
            <template v-if="sessionStore.sessions.length > 0">
              <div
                v-for="(session, index) in sessionStore.sessions"
                :key="session.id"
                v-show="activeSession && session.id === activeSession.id"
                class="absolute inset-0"
              >
                <TerminalTabArea
                  :ref="
                    (el: any) => {
                      if (el) terminalTabAreaRefs[index] = el;
                    }
                  "
                  :sessionId="session.id"
                />
              </div>
            </template>

            <div v-else class="flex h-full items-center justify-center px-6">
              <div class="w-full max-w-xl rounded-2xl border border-border-primary bg-bg-secondary px-8 py-10 text-center">
                <div class="text-2xl font-semibold text-text-primary">
                  SSH Assistant Ops
                </div>
                <div class="mt-3 text-sm text-text-secondary">
                  Select an asset to open terminal, files, tunnels, and AI ops context.
                </div>
                <div class="mt-6 flex flex-wrap items-center justify-center gap-2 text-xs text-text-secondary">
                  <span class="rounded-full border border-border-primary px-2.5 py-1">Alt+1 {{ t("workbench.focusMode") }}</span>
                  <span class="rounded-full border border-border-primary px-2.5 py-1">Alt+2 {{ t("workbench.showContextPane") }}</span>
                  <span class="rounded-full border border-border-primary px-2.5 py-1">Alt+3 AI</span>
                  <span class="rounded-full border border-border-primary px-2.5 py-1">Alt+4 Files</span>
                </div>
                <div class="mt-8 flex items-center justify-center gap-3">
                  <button class="btn btn-primary" @click="openNewConnectionModal">
                    New Asset
                  </button>
                  <button class="btn btn-secondary" @click="openConnectionsWorkbench">
                    Open Asset Center
                  </button>
                  <button class="btn btn-secondary" @click="openOpsWorkbench">
                    Open Ops Workbench
                  </button>
                </div>
              </div>
            </div>
          </div>

          <div class="h-8 overflow-x-auto overflow-y-hidden border-t border-border-primary bg-bg-secondary">
            <div class="flex h-full min-w-max items-center gap-2 px-3 text-xs text-text-secondary">
              <span
                class="shrink-0 whitespace-nowrap rounded-full border border-border-primary px-2 py-0.5 text-text-primary"
              >
                {{
                  activeSession
                    ? activeSession.status
                    : t("workbench.statusIdle")
                }}
              </span>
              <span v-if="activeSession" class="shrink-0 whitespace-nowrap">
                {{ t("app.sessionDuration") }} {{ activeSessionDuration }}
              </span>
              <span
                v-if="activeAssetRisk"
                class="shrink-0 whitespace-nowrap rounded-full border border-border-primary px-2 py-0.5"
                :class="
                  activeAssetRisk === 'critical'
                    ? 'text-error'
                    : activeAssetRisk === 'high'
                      ? 'text-warning'
                      : 'text-text-secondary'
                "
              >
                Risk {{ activeAssetRisk }}
              </span>
              <span
                v-if="activeAssetHealth"
                class="max-w-[12rem] shrink-0 truncate rounded-full border border-border-primary px-2 py-0.5"
                :title="activeAssetHealth"
              >
                {{ activeAssetHealth }}
              </span>
              <span
                v-if="activeSession?.currentPath"
                class="max-w-[18rem] shrink-0 truncate rounded-full border border-border-primary px-2 py-0.5"
                :title="activeSession.currentPath"
              >
                {{ activeSession.currentPath }}
              </span>
              <span
                v-if="activeSelection.count > 0"
                class="max-w-[18rem] shrink-0 truncate rounded-full border border-border-primary px-2 py-0.5"
                :title="
                  activeSelection.targetLabel
                    ? `${t('workbench.statusSelection', {
                        count: activeSelection.count,
                      })} · ${activeSelection.targetLabel}`
                    : t('workbench.statusSelection', {
                        count: activeSelection.count,
                      })
                "
              >
                {{
                  t("workbench.statusSelection", {
                    count: activeSelection.count,
                  })
                }}
                <span v-if="activeSelection.targetLabel"> · {{ activeSelection.targetLabel }}</span>
              </span>
              <span class="shrink-0 whitespace-nowrap rounded-full border border-border-primary px-2 py-0.5">
                {{ t("workbench.statusContext", { count: activeAiContextCount }) }}
              </span>
              <span
                v-if="activeTransferSummary.total > 0"
                class="shrink-0 whitespace-nowrap rounded-full border border-border-primary px-2 py-0.5"
              >
                {{
                  t("workbench.statusTransfers", {
                    total: activeTransferSummary.total,
                    running: activeTransferSummary.running,
                  })
                }}
              </span>
              <span
                v-if="activeSession && sessionStatus[activeSession.id]?.uptime"
                class="shrink-0 whitespace-nowrap rounded-full border border-border-primary px-2 py-0.5"
              >
                {{ sessionStatus[activeSession.id].uptime }}
              </span>
              <span
                v-if="activeSession && sessionStatus[activeSession.id]?.disk?.percent"
                class="shrink-0 whitespace-nowrap rounded-full border border-border-primary px-2 py-0.5"
              >
                Disk {{ sessionStatus[activeSession.id].disk?.percent }}
              </span>
              <span
                v-if="activeSession && sessionStatus[activeSession.id]?.ip"
                class="max-w-[10rem] shrink-0 truncate rounded-full border border-border-primary px-2 py-0.5"
                :title="sessionStatus[activeSession.id].ip"
              >
                {{ sessionStatus[activeSession.id].ip }}
              </span>
            </div>
          </div>
        </main>

        <div
          v-if="shouldShowInlineContextPane"
          class="w-1 shrink-0 cursor-col-resize bg-bg-primary transition-colors hover:bg-accent"
          @mousedown.prevent="startResize('context')"
        ></div>

        <aside
          v-if="shouldShowInlineContextPane"
          class="flex h-full min-w-0 shrink-0 flex-col overflow-hidden border-l border-border-primary bg-bg-secondary"
          :style="{ width: contextPaneWidth + 'px' }"
        >
          <div class="border-b border-border-primary px-4 py-3">
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0">
                <div class="truncate text-sm font-semibold text-text-primary">
                  {{
                    activeSession
                      ? activeSession.assetName
                      : t("workbench.contextTitle")
                  }}
                </div>
                <div class="mt-1 flex flex-wrap items-center gap-2 text-xs text-text-secondary">
                  <span v-if="activeAsset && activeSession?.accessEndpointId">
                    {{
                      (() => {
                        const endpoint = assetStore.defaultAccessEndpointForAsset(activeAsset.id);
                        return endpoint ? `${endpoint.username}@${endpoint.host}` : `${activeAsset.host}`;
                      })()
                    }}
                  </span>
                  <span
                    v-if="activeAssetRisk"
                    class="rounded-full border border-border-primary bg-bg-primary px-2 py-0.5"
                  >
                    Risk {{ activeAssetRisk }}
                  </span>
                  <span
                    v-if="activeWorkspace"
                    class="rounded-full border border-border-primary bg-bg-primary px-2 py-0.5"
                  >
                    {{ activeWorkspace.name }}
                  </span>
                  <span class="rounded-full border border-border-primary bg-bg-primary px-2 py-0.5">
                    {{ t("workbench.statusContext", { count: activeAiContextCount }) }}
                  </span>
                </div>
              </div>

              <button
                class="rounded-md p-1.5 text-text-secondary hover:bg-bg-elevated hover:text-text-primary"
                :title="t('workbench.refreshContext')"
                @click="updateTerminalContext"
              >
                <RefreshCw class="h-4 w-4" />
              </button>
            </div>
          </div>

          <div class="flex h-11 items-center gap-2 border-b border-border-primary px-3">
            <button
              class="flex-1 rounded-lg px-3 py-2 text-sm transition-colors"
              :class="
                activeContextTab === 'ai'
                  ? 'bg-bg-elevated text-text-primary'
                  : 'text-text-secondary hover:bg-bg-primary hover:text-text-primary'
              "
              :disabled="!activeSession"
              @click="setContextTab('ai')"
            >
              <span class="inline-flex items-center gap-2">
                <Bot class="h-4 w-4" />
                <span>AI</span>
              </span>
            </button>
            <button
              class="flex-1 rounded-lg px-3 py-2 text-sm transition-colors"
              :class="
                activeContextTab === 'files'
                  ? 'bg-bg-elevated text-text-primary'
                  : 'text-text-secondary hover:bg-bg-primary hover:text-text-primary'
              "
              :disabled="!activeSession"
              @click="setContextTab('files')"
            >
              <span class="inline-flex items-center gap-2">
                <FolderOpen class="h-4 w-4" />
                <span>Files</span>
              </span>
            </button>
          </div>

          <div class="relative min-h-0 flex-1 overflow-hidden">
            <template v-if="activeSession">
              <div
                v-for="(session, index) in sessionStore.sessions"
                :key="`${session.id}-context`"
                v-show="activeSession && session.id === activeSession.id"
                class="absolute inset-0"
              >
                <div v-show="sessionContextTabs[session.id] === 'ai'" class="h-full">
                  <AIAssistant
                    :ref="
                      (el: any) => {
                        if (el) aiAssistantRefs[index] = el;
                      }
                    "
                    :sessionId="session.id"
                    :terminal-context="terminalContext"
                    :show-header="false"
                    @refresh-context="updateTerminalContext"
                    @context-meta-change="handleContextMetaChange"
                  />
                </div>

                <div v-show="sessionContextTabs[session.id] === 'files'" class="h-full">
                  <FileManager
                    :sessionId="session.id"
                    :active="
                      activeSession?.id === session.id &&
                      sessionContextTabs[session.id] === 'files' &&
                      (shouldShowInlineContextPane || shouldShowContextDrawer)
                    "
                    @openFileEditor="
                      (filePath, fileName) =>
                        openFileEditor(session.id, filePath, fileName)
                    "
                    @switchToTerminalPath="switchTerminalToPath"
                    @addAiContextPaths="addAiContextPaths"
                    @path-change="handleFilePathChange"
                    @selection-change="handleFileSelectionChange"
                  />
                </div>
              </div>
            </template>

            <div v-else class="flex h-full items-center justify-center px-6 text-center">
              <div class="space-y-2">
                <div class="text-sm font-medium text-text-primary">
                  {{ t("workbench.contextEmptyTitle") }}
                </div>
                <div class="text-xs text-text-secondary">
                  {{ t("workbench.contextEmptyDescription") }}
                </div>
              </div>
            </div>
          </div>
        </aside>

        <div
          v-if="shouldShowAnyDrawer"
          class="absolute inset-0 z-30 bg-black/30"
          @click="closeDrawers"
        ></div>

        <aside
          v-if="shouldShowResourceDrawer"
          class="absolute inset-y-0 left-0 z-40 flex w-[320px] max-w-[calc(100%-56px)] min-h-0 min-w-0 flex-col overflow-hidden border-r border-border-primary bg-bg-secondary shadow-xl"
        >
          <div
            v-if="activeActivity !== 'sessions'"
            class="flex h-14 items-center justify-between border-b border-border-primary px-4"
          >
            <div>
              <div class="text-sm font-semibold text-text-primary">
                {{ activeResourcePaneMeta.title }}
              </div>
              <div class="mt-0.5 text-xs text-text-secondary">
                {{ activeResourcePaneMeta.subtitle }}
              </div>
            </div>
            <button
              class="rounded-md p-1.5 text-text-secondary hover:bg-bg-elevated hover:text-text-primary"
              :title="t('workbench.collapseResourcePane')"
              @click="resourceDrawerOpen = false"
            >
              <PanelLeftClose class="h-4 w-4" />
            </button>
          </div>
          <div class="min-h-0 flex-1">
            <AssetCenter
              v-if="activeActivity === 'connections'"
              @edit="openEditConnectionModal"
              @tunnels="openTunnelModal"
            />
            <div
              v-else-if="activeActivity === 'tunnels'"
              class="h-full overflow-y-auto px-4 py-4"
            >
              <TunnelPanel @manage="openTunnelModal" />
            </div>
            <OpsWorkbench
              v-else-if="activeActivity === 'ops'"
              :active-asset="activeAsset"
            />
            <SessionsWorkbenchPanel
              v-else
              @new-connection="openNewConnectionModal"
            />
          </div>
        </aside>

        <aside
          v-if="shouldShowContextDrawer"
          class="absolute inset-y-0 right-0 z-40 flex w-[min(420px,calc(100%-56px))] min-h-0 min-w-0 flex-col overflow-hidden border-l border-border-primary bg-bg-secondary shadow-xl"
        >
          <div class="border-b border-border-primary px-4 py-3">
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0">
                <div class="truncate text-sm font-semibold text-text-primary">
                  {{
                    activeSession
                      ? activeSession.assetName
                      : t("workbench.contextTitle")
                  }}
                </div>
                <div class="mt-1 text-xs text-text-secondary">
                  {{
                    activeAsset
                      ? (() => {
                          const endpoint = activeAsset.id
                            ? assetStore.defaultAccessEndpointForAsset(activeAsset.id)
                            : null;
                          return endpoint
                            ? `${endpoint.username}@${endpoint.host}`
                            : activeAsset.host;
                        })()
                      : t("workbench.contextEmptyDescription")
                  }}
                </div>
                <div
                  v-if="activeAssetHealth || activeAssetRisk"
                  class="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-text-secondary"
                >
                  <span v-if="activeAssetHealth">{{ activeAssetHealth }}</span>
                  <span v-if="activeAssetRisk">Risk {{ activeAssetRisk }}</span>
                </div>
              </div>
              <button
                class="rounded-md p-1.5 text-text-secondary hover:bg-bg-elevated hover:text-text-primary"
                :title="t('workbench.hideContextPane')"
                @click="contextDrawerOpen = false"
              >
                <PanelRightClose class="h-4 w-4" />
              </button>
            </div>
          </div>

          <div class="flex h-11 items-center gap-2 border-b border-border-primary px-3">
            <button
              class="flex-1 rounded-lg px-3 py-2 text-sm transition-colors"
              :class="
                activeContextTab === 'ai'
                  ? 'bg-bg-elevated text-text-primary'
                  : 'text-text-secondary hover:bg-bg-primary hover:text-text-primary'
              "
              :disabled="!activeSession"
              @click="setContextTab('ai')"
            >
              AI
            </button>
            <button
              class="flex-1 rounded-lg px-3 py-2 text-sm transition-colors"
              :class="
                activeContextTab === 'files'
                  ? 'bg-bg-elevated text-text-primary'
                  : 'text-text-secondary hover:bg-bg-primary hover:text-text-primary'
              "
              :disabled="!activeSession"
              @click="setContextTab('files')"
            >
              Files
            </button>
          </div>

          <div class="relative min-h-0 flex-1 overflow-hidden">
            <template v-if="activeSession">
              <div
                v-for="(session, index) in sessionStore.sessions"
                :key="`${session.id}-drawer-context`"
                v-show="activeSession && session.id === activeSession.id"
                class="absolute inset-0"
              >
                <div v-show="sessionContextTabs[session.id] === 'ai'" class="h-full">
                  <AIAssistant
                    :ref="
                      (el: any) => {
                        if (el) aiAssistantRefs[index] = el;
                      }
                    "
                    :sessionId="session.id"
                    :terminal-context="terminalContext"
                    :show-header="false"
                    @refresh-context="updateTerminalContext"
                    @context-meta-change="handleContextMetaChange"
                  />
                </div>

                <div v-show="sessionContextTabs[session.id] === 'files'" class="h-full">
                  <FileManager
                    :sessionId="session.id"
                    :active="
                      activeSession?.id === session.id &&
                      sessionContextTabs[session.id] === 'files' &&
                      shouldShowContextDrawer
                    "
                    @openFileEditor="
                      (filePath, fileName) =>
                        openFileEditor(session.id, filePath, fileName)
                    "
                    @switchToTerminalPath="switchTerminalToPath"
                    @addAiContextPaths="addAiContextPaths"
                    @path-change="handleFilePathChange"
                    @selection-change="handleFileSelectionChange"
                  />
                </div>
              </div>
            </template>

            <div v-else class="flex h-full items-center justify-center px-6 text-center">
              <div class="space-y-2">
                <div class="text-sm font-medium text-text-primary">
                  {{ t("workbench.contextEmptyTitle") }}
                </div>
                <div class="text-xs text-text-secondary">
                  {{ t("workbench.contextEmptyDescription") }}
                </div>
              </div>
            </div>
          </div>
        </aside>
      </div>
    </div>

    <ConnectionModal
      :show="showConnectionModal"
      :assetToEdit="editingAsset"
      :endpointToEdit="editingAccessEndpoint"
      :credentialRefToEdit="editingCredentialRef"
      @close="showConnectionModal = false"
      @save="handleSaveConnection"
    />
    <TunnelModal
      :show="showTunnelModal"
      :asset="tunnelAsset"
      @close="showTunnelModal = false"
    />
    <SettingsModal :show="showSettingsModal" @close="showSettingsModal = false" />

    <NotificationModal
      v-if="notificationStore.show"
      :show="notificationStore.show"
      :type="notificationStore.type"
      :title="notificationStore.title"
      :message="notificationStore.message"
      :duration="notificationStore.duration"
      @close="notificationStore.close()"
    />
  </div>
</template>
