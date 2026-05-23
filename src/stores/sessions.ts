import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useNotificationStore } from './notifications';
import type {
  OpsSession,
  HostAsset,
  ConnectionHistorySource,
  ConnectionStatusEvent,
  ReconnectEvent,
} from '../types';
import { useAssetStore } from './assets';
import { sessionService } from '../services';

export const useSessionStore = defineStore('sessions', {
  state: () => ({
    sessions: [] as OpsSession[],
    activeSessionId: null as string | null,
    _unlistenFns: [] as UnlistenFn[],
  }),
  getters: {
    activeSession: (state) => state.sessions.find(s => s.id === state.activeSessionId),
  },
  actions: {
    async setSessionWorkspace(sessionId: string, path: string) {
      const session = this.sessions.find(s => s.id === sessionId);
      if (!session) return;

      const name = path.split('/').pop() || 'workspace';
      
      // Initialize workspace state
      session.activeWorkspace = {
        path,
        name,
        context: 'Indexing...',
        fileTree: '',
        isIndexed: false,
      };

      try {
        // 1. Generate file tree (Limit depth to 2 to avoid huge output)
        // Try 'tree' first, then fallback to 'find'
        let treeOutput = '';
        try {
            // Check if tree exists
            const hasTree = await invoke<string>('exec_command', { id: sessionId, command: 'which tree' });
            if (hasTree && !hasTree.includes('no tree')) {
                 treeOutput = await invoke<string>('exec_command', { 
                    id: sessionId, 
                    command: `cd '${path}' && tree -L 2 --noreport` 
                });
            } else {
                 throw new Error('no tree');
            }
        } catch {
            // Fallback to find
            try {
                 // Find directories and files, max depth 2, exclude hidden files
                 const findCmd = `cd '${path}' && find . -maxdepth 2 -not -path '*/.*'`;
                 treeOutput = await invoke<string>('exec_command', { id: sessionId, command: findCmd });
            } catch (e) {
                treeOutput = "(Unable to list files: " + e + ")";
            }
        }

        // 2. Read key config files
        // List of common config files to check
        const configFiles = ['package.json', 'Cargo.toml', 'requirements.txt', 'docker-compose.yml', 'README.md', 'nginx.conf'];
        let contextSummary = "Key Configuration Files:\n";

        for (const file of configFiles) {
            try {
                // Check if file exists
                const checkCmd = `cd '${path}' && test -f ${file} && echo "yes"`;
                const exists = await invoke<string>('exec_command', { id: sessionId, command: checkCmd });
                
                if (exists && exists.trim() === 'yes') {
                     const content = await invoke<string>('read_remote_file', { 
                        id: sessionId, 
                        path: `${path.replace(/\/$/, '')}/${file}`, 
                        maxBytes: 2048 // Read first 2KB
                    });
                    contextSummary += `\n--- ${file} ---\n${content.substring(0, 1000)}${content.length > 1000 ? '\n...(truncated)' : ''}\n`;
                }
            } catch {
                // Ignore errors for individual files
            }
        }

        // 3. Check Git Status
        try {
             const gitStatus = await invoke<string>('exec_command', { 
                id: sessionId, 
                command: `cd '${path}' && git status -s | head -n 10` 
            });
            if (gitStatus && !gitStatus.includes('not a git repository')) {
                contextSummary += `\n--- Git Status ---\n${gitStatus}`;
            }
        } catch {}

        // Update workspace
        if (session.activeWorkspace) {
            session.activeWorkspace.fileTree = treeOutput;
            session.activeWorkspace.context = contextSummary;
            session.activeWorkspace.isIndexed = true;
        }

      } catch (e) {
        console.error("Failed to index workspace", e);
        if (session.activeWorkspace) {
            session.activeWorkspace.context = `Failed to index: ${e}`;
        }
      }
    },

    async createSession(asset: HostAsset, source: ConnectionHistorySource = 'tree') {
      const assetStore = useAssetStore();

      try {
        if (asset.id === undefined) {
          throw new Error('Asset ID is required');
        }
        const connectionResult = await sessionService.connectAsset(
          asset.id,
          asset.accessEndpointId ?? null,
          null,
          source,
        );
        const session: OpsSession = {
          id: connectionResult.sessionId,
          assetId: connectionResult.assetId,
          assetName: connectionResult.assetName,
          createdAt: connectionResult.createdAt,
          accessEndpointId: connectionResult.accessEndpointId ?? asset.accessEndpointId ?? null,
          credentialRefId: connectionResult.credentialRefId ?? null,
          bastionChainId: connectionResult.bastionChainId ?? asset.bastionChainId ?? null,
          currentPath: '.',
          riskLevel: connectionResult.riskLevel ?? asset.criticality ?? 'medium',
          healthSummary: connectionResult.healthSummary ?? asset.healthSummary ?? null,
          lastJobRunId: null,
          status: 'connected',
          activeTab: 'terminal',
          files: [],
          connectedAt: Date.now(),
          envId: connectionResult.envId ?? asset.envId ?? null,
          os: connectionResult.osInfo,
        };

        this.sessions.push(session);
        this.activeSessionId = connectionResult.sessionId;
        if (asset.id !== undefined) {
          assetStore.addSuccessfulConnection(asset.id, source);
          void assetStore.touchAsset(asset.id);
        }
      } catch (e) {
        console.error('Failed to connect', e);
        if (asset.id !== undefined) {
          assetStore.addFailedConnection(asset.id, String(e), source);
          void assetStore.appendAuditEvent({
            eventType: 'session.connectFailed',
            assetId: asset.id,
            sessionId: null,
            jobRunId: null,
            title: 'Session connection failed',
            detail: String(e),
            severity: 'warning',
            metadataJson: JSON.stringify({ source }),
            createdAt: Date.now(),
          });
        }
        useNotificationStore().error('Failed to connect: ' + e);
      }
    },
    async closeSession(id: string) {
      const session = this.sessions.find((item) => item.id === id);
      // 1. Optimistically update UI first
      this.sessions = this.sessions.filter(s => s.id !== id);
      if (this.activeSessionId === id) {
        this.activeSessionId = this.sessions.length > 0 ? this.sessions[0].id : null;
      }

      // 2. Perform backend disconnect in background
      try {
        await sessionService.disconnectAsset(id, session?.assetId ?? null);
      } catch (e) {
        console.error("Error disconnecting session:", e);
      }
    },
    async disconnectSession(id: string) {
      const index = this.sessions.findIndex(s => s.id === id);
      if (index === -1) return;

      // Optimistic update - replace object to ensure reactivity
      const session = this.sessions[index];
      this.sessions[index] = { ...session, status: 'disconnected' };

      try {
        await sessionService.disconnectAsset(id, session.assetId);
      } catch (e) {
        console.error('Failed to disconnect', e);
      }
    },
    async reconnectSession(id: string) {
      const session = this.sessions.find(s => s.id === id);
      if (!session) return;

      const assetStore = useAssetStore();
      const asset = assetStore.assets.find((item) => item.id === session.assetId);

      if (!asset?.id) {
        useNotificationStore().error('Asset configuration not found!');
        return;
      }

      session.status = 'connecting';
      try {
        const result = await sessionService.connectAsset(
          asset.id,
          session.accessEndpointId ?? asset.accessEndpointId ?? null,
          session.id,
          'history',
        );
        session.status = 'connected';
        session.connectedAt = Date.now();
        session.createdAt = result.createdAt;
        session.os = result.osInfo;
        session.assetName = result.assetName;
        session.envId = result.envId ?? asset.envId ?? null;
        session.riskLevel = result.riskLevel ?? asset.criticality ?? 'medium';
        session.healthSummary = result.healthSummary ?? asset.healthSummary ?? null;
        session.accessEndpointId = result.accessEndpointId ?? session.accessEndpointId ?? null;
        session.credentialRefId = result.credentialRefId ?? session.credentialRefId ?? null;
        session.bastionChainId = result.bastionChainId ?? session.bastionChainId ?? null;
      } catch (e) {
        console.error('Failed to reconnect', e);
        session.status = 'disconnected';
        useNotificationStore().error('Failed to reconnect: ' + e);
      }
    },
    setActiveSession(id: string) {
      this.activeSessionId = id;
    },
    updateSessionStatus(id: string, status: 'connected' | 'disconnected' | 'connecting') {
      console.log('updateSessionStatus called', id, status);
      const index = this.sessions.findIndex(s => s.id === id);
      if (index !== -1) {
        const session = this.sessions[index];
        this.sessions[index] = { ...session, status };
        console.log('Session status updated to', status);
      } else {
        console.warn('Session not found for status update', id);
      }
    },
    setActiveTab(tab: 'terminal' | 'files' | 'ai') {
      const session = this.activeSession;
      if (session) {
        session.activeTab = tab;
      }
    },

    // Setup event listeners for connection status updates
    async setupEventListeners() {
      // Listen for connection status changes
      const unlistenStatus = await listen<ConnectionStatusEvent>('connection:status', (event) => {
        const { sessionId, status } = event.payload;
        console.log('Connection status event received:', sessionId, status);

        // Map the detailed status to simplified session status
        const sessionStatus = this.mapConnectionStatusToSessionStatus(status);
        this.updateSessionStatus(sessionId, sessionStatus);
      });

      // Listen for connection errors
      const unlistenError = await listen<ConnectionStatusEvent>('connection:error', (event) => {
        const { sessionId, details } = event.payload;
        console.error('Connection error event received:', sessionId, details);
        this.updateSessionStatus(sessionId, 'disconnected');
        useNotificationStore().error(`Connection error: ${details || 'Unknown error'}`);
      });

      // Listen for reconnection attempts
      const unlistenReconnect = await listen<ReconnectEvent>('connection:reconnect', (event) => {
        const { sessionId, attempt, maxAttempts, delayMs } = event.payload;
        console.log('Reconnection attempt:', sessionId, attempt, maxAttempts);
        this.updateSessionStatus(sessionId, 'connecting');
        useNotificationStore().info(
          `Reconnecting... Attempt ${attempt}/${maxAttempts} (${delayMs}ms delay)`
        );
      });

      // Store unlisten functions for cleanup
      this._unlistenFns = [unlistenStatus, unlistenError, unlistenReconnect];
    },

    // Cleanup event listeners
    cleanupEventListeners() {
      this._unlistenFns.forEach((unlisten) => unlisten());
      this._unlistenFns = [];
    },

    // Map detailed connection status to simplified session status
    mapConnectionStatusToSessionStatus(
      status: 'connecting' | 'connected' | 'authenticating' | 'ready' | 'degraded' | 'reconnecting' | 'disconnected' | 'error'
    ): 'connected' | 'disconnected' | 'connecting' {
      switch (status) {
        case 'connecting':
        case 'authenticating':
        case 'reconnecting':
          return 'connecting';
        case 'connected':
        case 'ready':
        case 'degraded':
          return 'connected';
        case 'disconnected':
        case 'error':
        default:
          return 'disconnected';
      }
    }
  }
});
