
/**
 * Path utilities for handling POSIX-style paths (forward slashes).
 * Typically used for SSH/SFTP file management.
 */
export const pathUtils = {
    /**
     * Checks if a path is absolute (starts with /).
     */
    isAbsolute(path: string): boolean {
        return path.startsWith('/');
    },

    /**
     * Normalizes a path, resolving '..' and '.' segments.
     * Always returns forward slashes.
     * Handles multiple slashes as single slash.
     */
    normalize(path: string): string {
        if (!path) return '';
        
        const isAbs = this.isAbsolute(path);
        // Split by slash and filter out empty strings (caused by multiple slashes) and '.'
        const segments = path.split('/').filter(p => p && p !== '.');
        
        const resolvedSegments: string[] = [];
        
        for (const segment of segments) {
            if (segment === '..') {
                if (resolvedSegments.length > 0) {
                    resolvedSegments.pop();
                } else if (!isAbs) {
                    // If relative, we might need to keep '..'
                    // But for SFTP file manager, we usually want to stop at root if absolute,
                    // or just ignore if we can't go up further.
                    // Let's allow leading '..' for relative paths if needed, 
                    // but in this app we mostly aim for absolute paths.
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
    },

    /**
     * Joins path segments and normalizes the result.
     */
    join(...paths: string[]): string {
        // Filter out empty paths
        const validPaths = paths.filter(p => p && p.length > 0);
        if (validPaths.length === 0) return '.';
        
        return this.normalize(validPaths.join('/'));
    },

    /**
     * Returns the directory name of a path.
     */
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
    },

    /**
     * Returns the base name of a file (the last part of the path).
     */
    basename(path: string): string {
        if (path === '/') return '';
        // Remove trailing slash if exists (unless it is root, handled above)
        let p = path;
        if (p.endsWith('/') && p.length > 1) {
            p = p.slice(0, -1);
        }
        
        const lastSlashIndex = p.lastIndexOf('/');
        if (lastSlashIndex === -1) return p;
        
        return p.substring(lastSlashIndex + 1);
    }
};

// In-source test suite (compatible with Vitest if configured, or for manual verification)
if (import.meta.env?.MODE === 'test') {
    console.log('Running PathUtils tests...');
    const assert = (condition: boolean, msg: string) => {
        if (!condition) console.error(`[FAIL] ${msg}`);
        else console.log(`[PASS] ${msg}`);
    };

    assert(pathUtils.normalize('/a/b/../c') === '/a/c', 'Normalize ..');
    assert(pathUtils.normalize('/a/./b') === '/a/b', 'Normalize .');
    assert(pathUtils.normalize('//a//b///') === '/a/b', 'Normalize slashes');
    assert(pathUtils.normalize('/') === '/', 'Normalize root');
    assert(pathUtils.join('/a', 'b', 'c') === '/a/b/c', 'Join absolute');
    assert(pathUtils.join('/a', '..', 'b') === '/b', 'Join with ..'); // Corrected expectation: /a/../b -> /b
    assert(pathUtils.dirname('/a/b/c') === '/a/b', 'Dirname normal');
    assert(pathUtils.dirname('/a') === '/', 'Dirname at root parent');
    assert(pathUtils.dirname('/') === '/', 'Dirname of root');
}
