
/**
 * Path utilities for handling paths in different OS environments.
 * Provides strategies for POSIX (Linux/macOS) and Windows.
 */

export interface PathUtils {
    isAbsolute(path: string): boolean;
    normalize(path: string): string;
    join(...paths: string[]): string;
    dirname(path: string): string;
    basename(path: string): string;
    separator: string;
}

class PosixPathUtils implements PathUtils {
    separator = '/';

    isAbsolute(path: string): boolean {
        return path.startsWith('/');
    }

    normalize(path: string): string {
        if (!path) return '';
        
        const isAbs = this.isAbsolute(path);
        const segments = path.split('/').filter(p => p && p !== '.');
        
        const resolvedSegments: string[] = [];
        
        for (const segment of segments) {
            if (segment === '..') {
                if (resolvedSegments.length > 0) {
                    resolvedSegments.pop();
                } else if (!isAbs) {
                    resolvedSegments.push('..');
                }
            } else {
                resolvedSegments.push(segment);
            }
        }
        
        const joined = resolvedSegments.join('/');
        
        if (isAbs) {
            return '/' + joined;
        }
        
        return joined || '.';
    }

    join(...paths: string[]): string {
        const validPaths = paths.filter(p => p && p.length > 0);
        if (validPaths.length === 0) return '.';
        return this.normalize(validPaths.join('/'));
    }

    dirname(path: string): string {
        const normalized = this.normalize(path);
        if (normalized === '/') return '/';
        
        const lastSlashIndex = normalized.lastIndexOf('/');
        
        if (lastSlashIndex === -1) {
            return '.';
        }
        
        if (lastSlashIndex === 0) {
            return '/';
        }
        
        return normalized.substring(0, lastSlashIndex);
    }

    basename(path: string): string {
        if (path === '/') return '';
        let p = path;
        if (p.endsWith('/') && p.length > 1) {
            p = p.slice(0, -1);
        }
        
        const lastSlashIndex = p.lastIndexOf('/');
        if (lastSlashIndex === -1) return p;
        
        return p.substring(lastSlashIndex + 1);
    }
}

class WindowsPathUtils implements PathUtils {
    separator = '/'; // We prefer forward slashes for internal representation even on Windows SFTP

    isAbsolute(path: string): boolean {
        // Check for drive letter (e.g., C:) or UNC path (\\) or root slash (/)
        // For SFTP, we often see /C:/Users or C:/Users
        return /^[a-zA-Z]:/.test(path) || path.startsWith('/') || path.startsWith('\\');
    }

    normalize(path: string): string {
        if (!path) return '';

        // Convert backslashes to forward slashes
        let normalized = path.replace(/\\/g, '/');
        
        // Handle drive letter normalization
        // If it starts with C:, keep it.
        // If it starts with /C:, keep it.
        
        const isAbs = this.isAbsolute(normalized);
        
        // Split by slash
        const segments = normalized.split('/').filter(p => p && p !== '.');
        
        const resolvedSegments: string[] = [];
        
        for (const segment of segments) {
            if (segment === '..') {
                // If we are at root (e.g. C:/), we shouldn't pop the drive letter
                if (resolvedSegments.length > 0) {
                    const last = resolvedSegments[resolvedSegments.length - 1];
                    // Don't pop drive letter if it's the only thing left
                    if (/^[a-zA-Z]:$/.test(last) && resolvedSegments.length === 1) {
                        // Do nothing, can't go up from C:
                    } else {
                        resolvedSegments.pop();
                    }
                } else if (!isAbs) {
                    resolvedSegments.push('..');
                }
            } else {
                resolvedSegments.push(segment);
            }
        }
        
        let joined = resolvedSegments.join('/');
        
        // Re-attach prefix if absolute
        if (isAbs) {
            // Check if original had a slash at start or was a drive letter
            // If it started with '/', add it back. 
            // Exception: if it was 'C:/', split removed 'C:'. Wait, split keeps 'C:'.
            // 'C:/foo' -> ['C:', 'foo'] -> joined 'C:/foo'
            
            // If the original started with / and the first segment is NOT a drive letter, add /
            // If the original started with C:, don't add /
            
            // Case 1: /foo -> /foo
            // Case 2: C:/foo -> C:/foo
            // Case 3: /C:/foo -> /C:/foo (Cygwin style)
            
            const startsWithSlash = path.startsWith('/') || path.startsWith('\\');
            const startsWithDrive = /^[a-zA-Z]:/.test(path);
            
            if (startsWithSlash && !startsWithDrive) {
                 // e.g. /foo or /C:/foo (if C: is treated as folder name in first split check, but usually drive letter is distinct)
                 // Actually, /C:/foo -> segments ['C:', 'foo']. joined 'C:/foo'. We need to prepend '/'
                 return '/' + joined;
            }
            // If startsWithDrive, joined is 'C:/foo'. Correct.
        }
        
        return joined || '.';
    }

    join(...paths: string[]): string {
        const validPaths = paths.filter(p => p && p.length > 0);
        if (validPaths.length === 0) return '.';
        // Join with slash
        return this.normalize(validPaths.join('/'));
    }

    dirname(path: string): string {
        const normalized = this.normalize(path);
        // Handle root cases
        if (normalized === '/') return '/';
        if (/^[a-zA-Z]:\/?$/.test(normalized)) return normalized; // C: or C:/
        
        const lastSlashIndex = normalized.lastIndexOf('/');
        
        if (lastSlashIndex === -1) {
             if (/^[a-zA-Z]:$/.test(normalized)) return normalized;
             return '.';
        }
        
        // If index is 0, it's /foo -> /
        if (lastSlashIndex === 0) {
            return '/';
        }
        
        // If it looks like C:/foo, lastSlash is after C:
        // C:/foo -> C:/
        const candidate = normalized.substring(0, lastSlashIndex);
        
        // If candidate is 'C:', return 'C:/'
        if (/^[a-zA-Z]:$/.test(candidate)) {
            return candidate + '/';
        }
        
        return candidate;
    }

    basename(path: string): string {
        if (path === '/') return '';
        let p = path.replace(/\\/g, '/');
        if (p.endsWith('/') && p.length > 1) {
             // Don't strip if it is C:/
             if (!/^[a-zA-Z]:\/$/.test(p)) {
                 p = p.slice(0, -1);
             }
        }
        
        if (/^[a-zA-Z]:\/?$/.test(p)) return ''; // Root of drive has no basename? Or empty?
        
        const lastSlashIndex = p.lastIndexOf('/');
        if (lastSlashIndex === -1) return p;
        
        return p.substring(lastSlashIndex + 1);
    }
}

export const posixPathUtils = new PosixPathUtils();
export const windowsPathUtils = new WindowsPathUtils();

// Default to Posix for backward compatibility
export const pathUtils = posixPathUtils;

export function getPathUtils(os: string | undefined): PathUtils {
    if (!os) return posixPathUtils;
    const lowerOs = os.toLowerCase();
    if (lowerOs.includes('win') || lowerOs.includes('windows')) {
        return windowsPathUtils;
    }
    return posixPathUtils;
}

// In-source test suite
if (import.meta.env?.MODE === 'test') {
    console.log('Running PathUtils tests...');
    const assert = (condition: boolean, msg: string) => {
        if (!condition) console.error(`[FAIL] ${msg}`);
        else console.log(`[PASS] ${msg}`);
    };

    // Posix Tests
    assert(posixPathUtils.normalize('/a/b/../c') === '/a/c', 'Posix: Normalize ..');
    assert(posixPathUtils.dirname('/a/b/c') === '/a/b', 'Posix: Dirname normal');

    // Windows Tests
    const win = windowsPathUtils;
    assert(win.normalize('C:\\Users\\Admin') === 'C:/Users/Admin', 'Win: Normalize backslash');
    assert(win.normalize('C:/Users/../Admin') === 'C:/Admin', 'Win: Normalize ..');
    assert(win.join('C:\\Users', 'Admin') === 'C:/Users/Admin', 'Win: Join mixed');
    assert(win.dirname('C:/Users/Admin') === 'C:/Users', 'Win: Dirname');
    assert(win.dirname('C:/Users') === 'C:/', 'Win: Dirname of drive root child');
    assert(win.isAbsolute('C:/Users'), 'Win: isAbsolute drive');
    assert(win.isAbsolute('\\Users'), 'Win: isAbsolute slash');
}

