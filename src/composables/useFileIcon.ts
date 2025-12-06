import { invoke } from '@tauri-apps/api/core';

const iconCache = new Map<string, string>();
const pendingRequests = new Map<string, Promise<string | undefined>>();

export function useFileIcon() {
    
    async function getFileIcon(filename: string, isDir: boolean): Promise<string | undefined> {
        if (isDir) return undefined; // Let component handle directory icons for now, or fetch folder icon

        const ext = filename.split('.').pop()?.toLowerCase();
        if (!ext || ext === filename.toLowerCase()) {
             // No extension
             return undefined;
        }

        const cacheKey = ext;
        if (iconCache.has(cacheKey)) {
            return iconCache.get(cacheKey);
        }

        if (pendingRequests.has(cacheKey)) {
            return pendingRequests.get(cacheKey);
        }

        const request = (async () => {
            try {
                const icon = await invoke<string>('get_file_icon', { extension: ext });
                iconCache.set(cacheKey, icon);
                return icon;
            } catch (e) {
                console.error(`Failed to load icon for ${ext}`, e);
                return undefined;
            } finally {
                pendingRequests.delete(cacheKey);
            }
        })();

        pendingRequests.set(cacheKey, request);
        return request;
    }

    return {
        getFileIcon
    };
}
