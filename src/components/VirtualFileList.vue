<script setup lang="ts">
import { computed, ref, h } from 'vue';
import { useVirtualizer } from '@tanstack/vue-virtual';
import type { FileEntry, FileManagerViewMode, ColumnKey } from '../types';

interface TreeNode {
    entry: FileEntry;
    path: string;
    depth: number;
    parentPath: string | null;
    childrenLoaded: boolean;
    loading: boolean;
}

interface Props {
    items: FileEntry[] | TreeNode[];
    viewMode: FileManagerViewMode;
    selectedFiles: Set<string>;
    selectedTreePaths: Set<string>;
    columnWidths: Record<ColumnKey, number>;
    onSelection: (event: MouseEvent, file: FileEntry, index: number) => void;
    onNavigate: (entry: FileEntry) => void;
    onContextMenu: (event: MouseEvent, file: FileEntry) => void;
    onTreeSelection?: (node: TreeNode) => void;
    onOpenTreeFile?: (node: TreeNode) => void;
    onTreeContextMenu?: (event: MouseEvent, node: TreeNode) => void;
    onToggleDirectory?: (node: TreeNode) => void;
    onDragStart: (event: DragEvent, element: FileEntry | TreeNode) => void;
    expandedPaths?: Set<string>;
    formatSize: (size: number) => string;
    formatDate: (timestamp: number) => string;
}

const props = withDefaults(defineProps<Props>(), {
    viewMode: 'flat'
});

const virtualizerContainerRef = ref<HTMLElement>();

const virtualizerOptions = {
    get count() { return props.items.length; },
    getScrollElement: () => virtualizerContainerRef.value as Element | null,
    estimateSize: () => 32, // 每行高度
    overscan: 5,
};

const virtualizer = useVirtualizer(virtualizerOptions);

const virtualItems = computed(() => virtualizer.value.getVirtualItems());

const totalSize = computed(() => virtualizer.value.getTotalSize());

function renderFileItem(item: FileEntry, index: number) {
    const isSelected = props.selectedFiles.has(item.name);
    
    return h('div', {
        key: item.name,
        'data-file-item': 'true',
        class: [
            'flex items-center p-2 cursor-pointer border-b border-gray-800/50 transition-colors select-none h-full',
            {
                'bg-blue-900/50': isSelected,
                'hover:bg-gray-800': !isSelected
            }
        ],
        draggable: true,
        onDragstart: (e: DragEvent) => props.onDragStart(e, item),
        onClick: (e: MouseEvent) => props.onSelection(e, item, index),
        onDblclick: () => props.onNavigate(item),
        onContextmenu: (e: MouseEvent) => props.onContextMenu(e, item)
    }, [
        h('div', {
            class: 'flex items-center min-w-0',
            style: { width: props.columnWidths.name + 'px' }
        }, [
            item.isDir 
                ? h('svg', { class: 'w-4 h-4 mr-2 text-yellow-400 flex-shrink-0', fill: 'none', stroke: 'currentColor', viewBox: '0 0 24 24' }, [
                    h('path', { 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'stroke-width': '2', d: 'M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z' })
                ])
                : h('svg', { class: 'w-4 h-4 mr-2 text-blue-400 flex-shrink-0', fill: 'none', stroke: 'currentColor', viewBox: '0 0 24 24' }, [
                    h('path', { 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'stroke-width': '2', d: 'M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z' })
                ]),
            h('span', { class: 'text-sm truncate' }, item.name)
        ]),
        h('span', {
            class: 'text-xs text-gray-500 font-mono',
            style: { width: props.columnWidths.size + 'px', paddingLeft: '8px' }
        }, props.formatSize(item.size)),
        h('span', {
            class: 'text-xs text-gray-500 font-mono',
            style: { width: props.columnWidths.date + 'px', paddingLeft: '8px' }
        }, props.formatDate(item.mtime)),
        h('span', {
            class: 'text-xs text-gray-500 font-mono',
            style: { width: props.columnWidths.owner + 'px', paddingLeft: '8px' }
        }, item.owner || '-')
    ]);
}

function renderTreeNode(node: TreeNode) {
    const isSelected = props.selectedTreePaths.has(node.path);
    const isExpanded = props.expandedPaths?.has(node.path);
    
    return h('div', {
        key: node.path,
        'data-file-item': 'true',
        class: [
            'flex items-center p-2 cursor-pointer border-b border-gray-800/50 transition-colors select-none h-full',
            {
                'bg-blue-900/50': isSelected,
                'hover:bg-gray-800': !isSelected
            }
        ],
        draggable: true,
        onDragstart: (e: DragEvent) => props.onDragStart(e, node),
        onClick: () => props.onTreeSelection?.(node),
        onDblclick: () => props.onOpenTreeFile?.(node),
        onContextmenu: (e: MouseEvent) => props.onTreeContextMenu?.(e, node)
    }, [
        h('div', {
            class: 'flex items-center min-w-0',
            style: { 
                width: props.columnWidths.name + 'px', 
                paddingLeft: (node.depth * 16) + 'px' 
            }
        }, [
            node.entry.isDir ? h('button', {
                class: 'mr-1 w-3 h-3 flex items-center justify-center text-xs text-gray-400',
                onClick: (e: MouseEvent) => {
                    e.stopPropagation();
                    props.onToggleDirectory?.(node);
                }
            }, isExpanded ? '-' : '+') : h('span', { class: 'mr-4' }),
            node.entry.isDir 
                ? h('svg', { class: 'w-4 h-4 mr-2 text-yellow-400 flex-shrink-0', fill: 'none', stroke: 'currentColor', viewBox: '0 0 24 24' }, [
                    h('path', { 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'stroke-width': '2', d: 'M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z' })
                ])
                : h('svg', { class: 'w-4 h-4 mr-2 text-blue-400 flex-shrink-0', fill: 'none', stroke: 'currentColor', viewBox: '0 0 24 24' }, [
                    h('path', { 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'stroke-width': '2', d: 'M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z' })
                ]),
            h('span', { class: 'text-sm truncate' }, node.entry.name)
        ]),
        h('span', {
            class: 'text-xs text-gray-500 font-mono',
            style: { width: props.columnWidths.size + 'px', paddingLeft: '8px' }
        }, node.entry.isDir ? '' : props.formatSize(node.entry.size)),
        h('span', {
            class: 'text-xs text-gray-500 font-mono',
            style: { width: props.columnWidths.date + 'px', paddingLeft: '8px' }
        }, props.formatDate(node.entry.mtime)),
        h('span', {
            class: 'text-xs text-gray-500 font-mono',
            style: { width: props.columnWidths.owner + 'px', paddingLeft: '8px' }
        }, node.entry.owner || '-')
    ]);
}
</script>

<template>
    <div ref="virtualizerContainerRef" class="h-full overflow-auto">
        <div 
            :style="{ height: totalSize + 'px', width: '100%', position: 'relative' }"
        >
            <div
                v-for="virtualItem in virtualItems"
                :key="virtualItem.index"
                :style="{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    width: '100%',
                    height: `${virtualItem.size}px`,
                    transform: `translateY(${virtualItem.start}px)`,
                }"
            >
                <template v-if="viewMode === 'flat'">
                    <component :is="renderFileItem(items[virtualItem.index] as FileEntry, virtualItem.index)" />
                </template>
                <template v-else>
                    <component :is="renderTreeNode(items[virtualItem.index] as TreeNode)" />
                </template>
            </div>
        </div>
    </div>
</template>
