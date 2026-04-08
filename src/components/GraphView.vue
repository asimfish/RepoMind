<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import Graph from 'graphology'
import { Sigma } from 'sigma'
import forceAtlas2 from 'graphology-layout-forceatlas2'
import type { GraphNode, GraphEdge } from '@/types'

const props = defineProps<{
  nodes: GraphNode[]
  edges: GraphEdge[]
  focusNodeId?: string
}>()

const emit = defineEmits<{
  nodeClick: [nodeId: string]
}>()

const containerRef = ref<HTMLDivElement>()
let sigmaInstance: Sigma | null = null
let graph: Graph | null = null

// Color palette per node type
const typeColors: Record<string, string> = {
  file: '#484f58',
  function: '#388bfd',
  class: '#3fb950',
  method: '#d29922',
  community: '#a371f7',
  process: '#f85149',
  variable: '#8b949e',
  interface: '#56d364',
}

const buildGraph = () => {
  if (!containerRef.value || !props.nodes.length) return

  graph = new Graph({ multi: false, type: 'directed' })

  // Add nodes with visual attributes
  props.nodes.forEach((node, i) => {
    const angle = (i / props.nodes.length) * 2 * Math.PI
    const radius = 200 + Math.random() * 300
    graph!.addNode(node.id, {
      label: node.label,
      x: Math.cos(angle) * radius,
      y: Math.sin(angle) * radius,
      size: node.type === 'community' ? 14 : node.type === 'class' ? 10 : 7,
      color: typeColors[node.type] ?? '#8b949e',
      type: 'circle',
    })
  })

  // Add edges
  props.edges.forEach((edge) => {
    if (graph!.hasNode(edge.source) && graph!.hasNode(edge.target)) {
      try {
        graph!.addEdge(edge.source, edge.target, {
          size: Math.max(1, edge.confidence * 2),
          color: edge.confidence > 0.8 ? '#388bfd40' : '#30363d',
          type: 'arrow',
        })
      } catch { /* ignore duplicate edges */ }
    }
  })

  // Run ForceAtlas2 layout
  forceAtlas2.assign(graph, {
    iterations: 100,
    settings: {
      gravity: 1,
      scalingRatio: 2,
      strongGravityMode: false,
      barnesHutOptimize: props.nodes.length > 500,
    },
  })

  // Create Sigma renderer
  sigmaInstance = new Sigma(graph, containerRef.value, {
    renderEdgeLabels: false,
    defaultEdgeType: 'arrow',
    allowInvalidContainer: true,
    labelFont: 'system-ui',
    labelSize: 11,
    labelColor: { color: '#8b949e' },
    labelRenderedSizeThreshold: 6,
  })

  // Node click → emit event
  sigmaInstance.on('clickNode', ({ node }) => {
    emit('nodeClick', node)
  })

  // Hover highlight
  sigmaInstance.on('enterNode', ({ node }) => {
    graph!.setNodeAttribute(node, 'highlighted', true)
  })
  sigmaInstance.on('leaveNode', ({ node }) => {
    graph!.setNodeAttribute(node, 'highlighted', false)
  })
}

// Focus on a specific node
watch(() => props.focusNodeId, (id) => {
  if (!id || !sigmaInstance || !graph || !graph.hasNode(id)) return
  const camera = sigmaInstance.getCamera()
  const nodePos = sigmaInstance.getNodeDisplayData(id)
  if (nodePos) {
    camera.animate({ x: nodePos.x, y: nodePos.y, ratio: 0.5 }, { duration: 600 })
  }
  // Highlight the node
  graph.setNodeAttribute(id, 'color', '#f8b500')
  graph.setNodeAttribute(id, 'size', 16)
})

watch(() => [props.nodes, props.edges], () => {
  sigmaInstance?.kill()
  sigmaInstance = null
  buildGraph()
}, { deep: true })

onMounted(buildGraph)

onUnmounted(() => {
  sigmaInstance?.kill()
  sigmaInstance = null
})
</script>

<template>
  <div class="relative h-full w-full">
    <div ref="containerRef" class="h-full w-full" />

    <!-- Legend -->
    <div class="absolute bottom-3 left-3 rounded-lg border border-[#30363d] bg-[#0d1117]/90 p-3 backdrop-blur-sm">
      <p class="mb-2 text-xs font-medium text-[#8b949e]">图例</p>
      <div class="space-y-1">
        <div v-for="(color, type) in { function: '#388bfd', class: '#3fb950', method: '#d29922', community: '#a371f7' }"
          :key="type" class="flex items-center gap-2">
          <span class="h-2.5 w-2.5 rounded-full" :style="{ background: color }" />
          <span class="text-xs text-[#8b949e]">{{ type }}</span>
        </div>
      </div>
    </div>

    <!-- Controls -->
    <div class="absolute right-3 top-3 flex flex-col gap-1">
      <button
        class="flex h-7 w-7 items-center justify-center rounded border border-[#30363d] bg-[#161b22] text-[#8b949e] hover:text-[#e6edf3] transition-colors text-sm"
        @click="sigmaInstance?.getCamera().animatedZoom({ duration: 300 })"
        title="放大"
      >+</button>
      <button
        class="flex h-7 w-7 items-center justify-center rounded border border-[#30363d] bg-[#161b22] text-[#8b949e] hover:text-[#e6edf3] transition-colors text-sm"
        @click="sigmaInstance?.getCamera().animatedUnzoom({ duration: 300 })"
        title="缩小"
      >−</button>
      <button
        class="flex h-7 w-7 items-center justify-center rounded border border-[#30363d] bg-[#161b22] text-[#8b949e] hover:text-[#e6edf3] transition-colors text-xs"
        @click="sigmaInstance?.getCamera().animatedReset({ duration: 300 })"
        title="重置视图"
      >⌂</button>
    </div>

    <!-- Empty state -->
    <div v-if="!nodes.length" class="absolute inset-0 flex items-center justify-center text-sm text-[#484f58]">
      暂无图谱数据，请先索引仓库
    </div>
  </div>
</template>
