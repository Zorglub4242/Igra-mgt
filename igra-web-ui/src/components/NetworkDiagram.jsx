import { useEffect, useState, useCallback } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  MarkerType,
  Panel,
  useReactFlow,
  useStore
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import ELK from 'elkjs/lib/elk.bundled.js';
import './NetworkDiagram.css';
import { api } from '../services/api';

// Custom component to render swim lane backgrounds in viewport coordinates
function SwimLanes({ nodes }) {
  const [viewport, setViewport] = useState({ x: 0, y: 0, zoom: 1 });
  const reactFlowInstance = useStore(state => state);

  // Get viewport transform
  useEffect(() => {
    const transform = reactFlowInstance?.transform;
    if (transform) {
      setViewport({ x: transform[0], y: transform[1], zoom: transform[2] });
    }
  }, [reactFlowInstance?.transform]);

  if (!nodes || nodes.length === 0) return null;

  // Group nodes by layer and calculate bounds
  const layerGroups = {
    internet: { nodes: [], minY: Infinity, maxY: -Infinity, minX: Infinity, maxX: -Infinity },
    firewall: { nodes: [], minY: Infinity, maxY: -Infinity, minX: Infinity, maxX: -Infinity },
    gateway: { nodes: [], minY: Infinity, maxY: -Infinity, minX: Infinity, maxX: -Infinity },
    docker: { nodes: [], minY: Infinity, maxY: -Infinity, minX: Infinity, maxX: -Infinity },
    systemd: { nodes: [], minY: Infinity, maxY: -Infinity, minX: Infinity, maxX: -Infinity },
    management: { nodes: [], minY: Infinity, maxY: -Infinity, minX: Infinity, maxX: -Infinity },
  };

  // Also group by project/category across all layers
  const projectGroups = {};

  nodes.forEach(node => {
    const layer = node.data?.layer || 'docker';
    if (layerGroups[layer]) {
      layerGroups[layer].nodes.push(node);
      const nodeY = node.position?.y || 0;
      const nodeX = node.position?.x || 0;
      const nodeHeight = 80;
      const nodeWidth = 180;
      layerGroups[layer].minY = Math.min(layerGroups[layer].minY, nodeY);
      layerGroups[layer].maxY = Math.max(layerGroups[layer].maxY, nodeY + nodeHeight);
      layerGroups[layer].minX = Math.min(layerGroups[layer].minX, nodeX);
      layerGroups[layer].maxX = Math.max(layerGroups[layer].maxX, nodeX + nodeWidth);
    }

    // Group by project (for containers) or category (for system services)
    // Skip internet and firewall layers
    if (layer === 'internet' || layer === 'firewall') return;

    const groupKey = node.data?.project || node.data?.category;
    if (groupKey) {
      if (!projectGroups[groupKey]) {
        projectGroups[groupKey] = {
          nodes: [],
          minY: Infinity,
          maxY: -Infinity,
          minX: Infinity,
          maxX: -Infinity,
          layer: layer,
          type: node.data?.project ? 'project' : 'category'
        };
      }
      projectGroups[groupKey].nodes.push(node);
      const nodeY = node.position?.y || 0;
      const nodeX = node.position?.x || 0;
      const nodeHeight = 80;
      const nodeWidth = 180;
      projectGroups[groupKey].minY = Math.min(projectGroups[groupKey].minY, nodeY);
      projectGroups[groupKey].maxY = Math.max(projectGroups[groupKey].maxY, nodeY + nodeHeight);
      projectGroups[groupKey].minX = Math.min(projectGroups[groupKey].minX, nodeX);
      projectGroups[groupKey].maxX = Math.max(projectGroups[groupKey].maxX, nodeX + nodeWidth);
    }
  });

  const layerColors = {
    internet: 'rgba(239, 68, 68, 0.08)',
    firewall: 'rgba(249, 115, 22, 0.08)',
    gateway: 'rgba(59, 130, 246, 0.08)',
    docker: 'rgba(16, 185, 129, 0.08)',
    systemd: 'rgba(139, 92, 246, 0.08)',
    management: 'rgba(107, 114, 128, 0.08)',
  };

  const layerBorders = {
    internet: 'rgba(239, 68, 68, 0.3)',
    firewall: 'rgba(249, 115, 22, 0.3)',
    gateway: 'rgba(59, 130, 246, 0.3)',
    docker: 'rgba(16, 185, 129, 0.3)',
    systemd: 'rgba(139, 92, 246, 0.3)',
    management: 'rgba(107, 114, 128, 0.3)',
  };

  const layerLabels = {
    internet: '🌍 Internet',
    firewall: '🛡️ Firewall',
    gateway: '🚪 Gateway (Nginx/Traefik)',
    docker: '🐳 Docker Containers',
    systemd: '⚙️ System Services',
    management: '🔧 Management Tools'
  };

  // Generate dynamic colors for projects/categories
  const generateGroupColor = (groupName, index) => {
    const colors = [
      { bg: 'rgba(139, 92, 246, 0.15)', border: 'rgba(139, 92, 246, 0.5)' },  // Purple
      { bg: 'rgba(6, 182, 212, 0.15)', border: 'rgba(6, 182, 212, 0.5)' },    // Cyan
      { bg: 'rgba(236, 72, 153, 0.15)', border: 'rgba(236, 72, 153, 0.5)' },  // Pink
      { bg: 'rgba(34, 197, 94, 0.15)', border: 'rgba(34, 197, 94, 0.5)' },    // Green
      { bg: 'rgba(251, 146, 60, 0.15)', border: 'rgba(251, 146, 60, 0.5)' },  // Orange
      { bg: 'rgba(168, 85, 247, 0.15)', border: 'rgba(168, 85, 247, 0.5)' },  // Violet
      { bg: 'rgba(14, 165, 233, 0.15)', border: 'rgba(14, 165, 233, 0.5)' },  // Sky blue
    ];
    return colors[index % colors.length];
  };

  const groupNames = Object.keys(projectGroups);

  return (
    <svg style={{ position: 'absolute', top: 0, left: 0, width: '100%', height: '100%', pointerEvents: 'none', zIndex: 0 }}>
      <g transform={`translate(${viewport.x}, ${viewport.y}) scale(${viewport.zoom})`}>
        {/* Render layer backgrounds first */}
        {Object.entries(layerGroups).map(([layer, group]) => {
          if (group.nodes.length === 0 || group.minY === Infinity) return null;

          const padding = 60;
          const y = group.minY - padding;
          const height = group.maxY - group.minY + padding * 2;
          const x = group.minX - padding;
          const width = group.maxX - group.minX + padding * 2;

          return (
            <g key={layer}>
              <rect
                x={x}
                y={y}
                width={width}
                height={height}
                fill={layerColors[layer]}
                stroke={layerBorders[layer]}
                strokeWidth={2}
                strokeDasharray={layer === 'management' ? '10,5' : '0'}
                rx={8}
              />
              <text
                x={x + 20}
                y={y + 30}
                fill="#334155"
                fontSize="16"
                fontWeight="600"
                opacity="0.8"
              >
                {layerLabels[layer]}
              </text>
            </g>
          );
        })}

        {/* Render project/category groups on top */}
        {Object.entries(projectGroups).map(([groupName, group], index) => {
          if (group.nodes.length === 0 || group.minY === Infinity) return null;

          const groupColor = generateGroupColor(groupName, index);
          const padding = 40;  // Increased padding to prevent visual overlap
          const y = group.minY - padding;
          const height = group.maxY - group.minY + padding * 2;
          const x = group.minX - padding;
          const width = group.maxX - group.minX + padding * 2;

          // Format label
          const icon = group.type === 'project' ? '📦' : '🏷️';
          const label = `${icon} ${groupName}`;

          return (
            <g key={`group-${groupName}`}>
              <rect
                x={x}
                y={y}
                width={width}
                height={height}
                fill={groupColor.bg}
                stroke={groupColor.border}
                strokeWidth={2}
                strokeDasharray="5,3"
                rx={6}
              />
              <text
                x={x + 15}
                y={y + 20}
                fill="#334155"
                fontSize="12"
                fontWeight="600"
                opacity="0.9"
              >
                {label}
              </text>
            </g>
          );
        })}
      </g>
    </svg>
  );
}

const elk = new ELK();

// ELK layout options for hierarchical layout with partitioning
const elkOptions = {
  'elk.algorithm': 'layered',
  'elk.direction': 'RIGHT',  // Left to right for horizontal swim lanes
  'elk.partitioning.activate': 'true',  // Enable partitioning for swim lanes
  'elk.layered.spacing.nodeNodeBetweenLayers': '400',  // Dramatically increased spacing between layers
  'elk.spacing.nodeNode': '250',  // Dramatically increased spacing between nodes in same layer
  'elk.layered.nodePlacement.strategy': 'LINEAR_SEGMENTS',  // Better spacing control
  'elk.layered.crossingMinimization.strategy': 'LAYER_SWEEP',  // Reduce edge crossings
  'elk.spacing.componentComponent': '300',  // More space between disconnected components
  'elk.hierarchyHandling': 'INCLUDE_CHILDREN',  // Better handling of partitioned nodes
  'elk.layered.compaction.postCompaction.strategy': 'NONE',  // Prevent aggressive compaction
  'elk.layered.considerModelOrder.strategy': 'PREFER_EDGES',  // Allow more flexible ordering
  'elk.separateConnectedComponents': 'true',  // Keep groups separated
  'elk.layered.spacing.edgeNodeBetweenLayers': '100',  // Edge clearance
  'elk.layered.thoroughness': '10',  // More layout iterations for better results
};

// Layer-based node styles
const layerStyles = {
  internet: { background: '#ef4444', color: 'white', border: '2px solid #dc2626' },
  firewall: { background: '#f97316', color: 'white', border: '2px solid #ea580c' },
  gateway: { background: '#3b82f6', color: 'white', border: '2px solid #2563eb' },
  docker: { background: '#10b981', color: 'white', border: '2px solid #059669' },
  systemd: { background: '#8b5cf6', color: 'white', border: '2px solid #7c3aed' },
  management: { background: '#6b7280', color: 'white', border: '2px dashed #4b5563' },
};

// Project-based colors for containers (override zone color)
const projectColors = {
  'igra-orchestra-testnet': { background: '#8b5cf6', border: '#7c3aed' },  // Purple for IGRA L2
  'kasplex': { background: '#06b6d4', border: '#0891b2' },  // Cyan for Kasplex
  'default': { background: '#10b981', border: '#059669' },  // Green for others
};

// Systemd services use different styling
const systemdStyle = { background: '#3b82f6', border: '#2563eb' };  // Blue for system services

// Protocol-based edge styles
const protocolStyles = {
  http: { stroke: '#10b981', strokeWidth: 2 },
  ws: { stroke: '#3b82f6', strokeWidth: 2, strokeDasharray: '5,5' },
  ipc: { stroke: '#f97316', strokeWidth: 2, strokeDasharray: '2,2' },
  tcp: { stroke: '#6b7280', strokeWidth: 1.5 },
  port_mapping: { stroke: '#ef4444', strokeWidth: 2 },
  network: { stroke: '#6b7280', strokeWidth: 1, strokeDasharray: '5,5' },
  dependency: { stroke: '#8b5cf6', strokeWidth: 1.5, strokeDasharray: '3,3' },
};

// Layer order for swim lanes (left to right)
const layerOrder = ['internet', 'firewall', 'gateway', 'docker', 'systemd', 'management'];

export default function NetworkDiagram() {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedLayers, setSelectedLayers] = useState(new Set(layerOrder));
  const [showWarningsOnly, setShowWarningsOnly] = useState(false);
  const [allNodes, setAllNodes] = useState([]);
  const [allEdges, setAllEdges] = useState([]);
  const [selectedNode, setSelectedNode] = useState(null);
  const [showModal, setShowModal] = useState(false);

  // Helper function to find network path from external to selected node
  const findNetworkPath = useCallback((targetNodeId, nodes, edges) => {
    if (!nodes.length || !edges.length) return [];

    // Build adjacency list (incoming edges)
    const incomingEdges = {};
    edges.forEach(edge => {
      if (!incomingEdges[edge.target]) {
        incomingEdges[edge.target] = [];
      }
      incomingEdges[edge.target].push({ source: edge.source, edge });
    });

    // Find all paths from target back to sources using BFS
    const paths = [];
    const queue = [{ node: targetNodeId, path: [targetNodeId] }];
    const visited = new Set();

    while (queue.length > 0) {
      const { node, path } = queue.shift();

      if (visited.has(node) && path.length > 1) continue;
      visited.add(node);

      const incoming = incomingEdges[node] || [];

      if (incoming.length === 0) {
        // Reached a root node
        paths.push(path.reverse());
      } else {
        incoming.forEach(({ source, edge }) => {
          if (!path.includes(source)) {
            queue.push({
              node: source,
              path: [...path, source]
            });
          }
        });
      }
    }

    // Sort paths by layer order (internet → firewall → gateway → etc.)
    const getLayerIndex = (nodeId) => {
      const node = nodes.find(n => n.id === nodeId);
      return node?.data?.layer ? layerOrder.indexOf(node.data.layer) : 999;
    };

    paths.sort((a, b) => {
      const aMinLayer = Math.min(...a.map(getLayerIndex));
      const bMinLayer = Math.min(...b.map(getLayerIndex));
      return aMinLayer - bMinLayer;
    });

    return paths[0] || [targetNodeId]; // Return the most external path
  }, []);

  // Get relationships for a node
  const getNodeRelationships = useCallback((nodeId, edges) => {
    const incoming = edges.filter(e => e.target === nodeId);
    const outgoing = edges.filter(e => e.source === nodeId);
    return { incoming, outgoing };
  }, []);

  // Apply ELK layout with swim lanes by layer
  const getLayoutedElements = useCallback(async (nodes, edges) => {
    // Group nodes by layer for swim lane layout
    const nodesByLayer = {};
    layerOrder.forEach(layer => { nodesByLayer[layer] = []; });

    nodes.forEach(node => {
      const layer = node.data?.layer || 'docker';
      if (nodesByLayer[layer]) {
        nodesByLayer[layer].push(node);
      }
    });

    // Assign partition IDs based on layer order
    const nodesWithPartitions = nodes.map(node => {
      const layer = node.data?.layer || 'docker';
      const partitionId = layerOrder.indexOf(layer);
      return {
        id: node.id,
        width: 220,  // Increased from 180 to account for content width
        height: 120, // Increased from 60 to account for multi-line content (domains, ports, IPs)
        properties: {
          'elk.partitioning.partition': partitionId >= 0 ? partitionId : 3  // Default to docker layer
        }
      };
    });

    // Validate edges - filter out any that reference non-existent nodes
    const nodeIdSet = new Set(nodesWithPartitions.map(n => n.id));
    const validEdges = edges.filter(edge => {
      const hasSource = nodeIdSet.has(edge.source);
      const hasTarget = nodeIdSet.has(edge.target);
      if (!hasSource || !hasTarget) {
        console.warn('[ELK] Filtering out edge with missing node:', edge.source, '->', edge.target);
      }
      return hasSource && hasTarget;
    });

    const graph = {
      id: 'root',
      layoutOptions: elkOptions,
      children: nodesWithPartitions,
      edges: validEdges.map(edge => ({
        id: edge.id,
        sources: [edge.source],
        targets: [edge.target],
      })),
    };

    console.log('[ELK] Graph for layout:', {
      nodes: graph.children.length,
      edges: graph.edges.length,
      filtered: edges.length - validEdges.length
    });

    const layoutedGraph = await elk.layout(graph);

    const layoutedNodes = nodes.map(node => {
      const layoutedNode = layoutedGraph.children.find(n => n.id === node.id);
      return {
        ...node,
        position: {
          x: layoutedNode.x,
          y: layoutedNode.y,
        },
      };
    });

    return { nodes: layoutedNodes, edges };
  }, []);

  // Fetch topology data from API
  useEffect(() => {
    const fetchTopology = async () => {
      try {
        setLoading(true);
        const topology = await api.request('/api/network-topology');

        console.log('[API] Topology response:', topology);
        console.log('[API] First node:', topology.nodes[0]);

        // Check for edges referencing non-existent nodes
        const nodeIds = new Set(topology.nodes.map(n => n.id));
        const missingNodes = new Set();
        topology.edges.forEach(edge => {
          if (!nodeIds.has(edge.source)) {
            missingNodes.add(edge.source);
            console.error('[API] Edge references missing source node:', edge.source, 'in edge:', edge);
          }
          if (!nodeIds.has(edge.target)) {
            missingNodes.add(edge.target);
            console.error('[API] Edge references missing target node:', edge.target, 'in edge:', edge);
          }
        });
        if (missingNodes.size > 0) {
          console.error('[API] Missing nodes:', Array.from(missingNodes));
        }

        // Transform API nodes to React Flow nodes
        const flowNodes = topology.nodes.map(node => {
          // Determine node styling based on type and project
          let nodeStyle;
          if (node.node_type === 'service') {
            // System services (nginx, kaspa-mainnet, etc.)
            nodeStyle = { background: systemdStyle.background, border: `2px solid ${systemdStyle.border}` };
          } else if (node.node_type === 'container' && node.metadata?.project) {
            // Docker containers - color by project
            const project = node.metadata.project;
            const projColor = projectColors[project] || projectColors.default;
            nodeStyle = { background: projColor.background, border: `2px solid ${projColor.border}` };
          } else {
            // Other nodes - use layer-based coloring
            const layerStyle = layerStyles[node.layer] || layerStyles.docker;
            nodeStyle = layerStyle;
          }

          // Add layer badge
          const layerBadge = {
            internet: '🌍',
            firewall: '🛡️',
            gateway: '🚪',
            docker: '🐳',
            systemd: '⚙️',
            management: '🔧'
          }[node.layer] || '';

          // Add project badge for containers
          const projectBadge = node.metadata?.project ? '📦' : '';

          return {
            id: node.id,
            type: 'default',
            data: {
              layer: node.layer,  // Store layer for layout algorithm
              project: node.metadata?.project,  // Store project for grouping
              category: node.metadata?.category,  // Store category for system services
              warnings: node.warnings,  // Store warnings for filter
              domains: node.domains,  // Store domains for search
              ip_address: node.ip_address,  // Store IP for search
              name: node.label,  // Store name as string for search
              label: (
                <div style={{ textAlign: 'center' }}>
                  <div style={{ fontWeight: 'bold', fontSize: '18px', marginBottom: '4px', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '4px' }}>
                    <span>{layerBadge}</span>
                    {projectBadge && <span>{projectBadge}</span>}
                    <span>{node.label}</span>
                    {node.warnings && node.warnings.length > 0 && (
                      <span title={node.warnings.join('\n')} style={{ cursor: 'help' }}>⚠️</span>
                    )}
                  </div>
                  {node.domains && node.domains.length > 0 && (
                    <div style={{ fontSize: '9px', opacity: 0.7, fontStyle: 'italic', color: '#fbbf24' }}>
                      🌐 {node.domains.slice(0, 2).join(', ')}
                      {node.domains.length > 2 && ` +${node.domains.length - 2}`}
                    </div>
                  )}
                  {node.metadata?.project && (
                    <div style={{ fontSize: '9px', opacity: 0.6, fontStyle: 'italic' }}>
                      {node.metadata.project}
                    </div>
                  )}
                  {node.ports.length > 0 && (
                    <div style={{ fontSize: '10px', opacity: 0.8 }}>
                      {node.ports.slice(0, 3).join(', ')}
                      {node.ports.length > 3 && '...'}
                    </div>
                  )}
                  {node.ip_address && (
                    <div style={{ fontSize: '9px', opacity: 0.7 }}>
                      {node.ip_address}
                    </div>
                  )}
                </div>
              ),
            },
            position: { x: 0, y: 0 }, // Will be set by ELK
            style: {
              ...nodeStyle,
              color: 'white',
              padding: '10px',
              borderRadius: '8px',
              minWidth: '180px',
              fontSize: '12px',
            },
          };
        });

        // Transform API edges to React Flow edges
        const flowEdges = topology.edges.map((edge, idx) => {
          // Use protocol for styling if available, otherwise fall back to edge_type
          const styleKey = edge.protocol || edge.edge_type;
          const edgeStyle = protocolStyles[styleKey] || { stroke: '#94a3b8', strokeWidth: 1 };

          // Build label with protocol info
          let label = edge.label || '';

          // Truncate long labels (safety measure)
          if (label.length > 50) {
            label = label.substring(0, 47) + '...';
          }

          if (edge.protocol && edge.protocol !== 'tcp') {
            label = label ? `${label} (${edge.protocol.toUpperCase()})` : edge.protocol.toUpperCase();
          }

          // Reduce label clutter for network and dependency connections
          if (edge.edge_type === 'network' || edge.edge_type === 'depends_on') {
            label = '';
          }

          return {
            id: `${edge.source}-${edge.target}-${idx}`,
            source: edge.source,
            target: edge.target,
            label,
            type: 'default',
            animated: edge.protocol === 'ws' || edge.protocol === 'ipc',
            style: edgeStyle,
            markerEnd: {
              type: MarkerType.ArrowClosed,
              color: edgeStyle.stroke,
            },
          };
        });

        // Apply ELK layout
        const { nodes: layoutedNodes, edges: layoutedEdges } = await getLayoutedElements(
          flowNodes,
          flowEdges
        );

        setAllNodes(layoutedNodes);
        setAllEdges(layoutedEdges);
        setNodes(layoutedNodes);
        setEdges(layoutedEdges);
        setError(null);
      } catch (err) {
        console.error('Error fetching network topology:', err);
        setError(err.message);
      } finally {
        setLoading(false);
      }
    };

    fetchTopology();
  }, [getLayoutedElements, setNodes, setEdges]);

  // Apply filters whenever search/filter options change
  useEffect(() => {
    if (allNodes.length === 0) return;

    let filteredNodes = allNodes.filter(node => {
      // Layer filter
      if (!selectedLayers.has(node.data?.layer)) {
        return false;
      }

      // Warnings filter
      if (showWarningsOnly && (!node.data?.warnings || node.data.warnings.length === 0)) {
        return false;
      }

      // Search filter
      if (searchQuery.trim()) {
        const query = searchQuery.toLowerCase();
        const matchesName = node.data?.name?.toLowerCase().includes(query);
        const matchesDomain = node.data?.domains?.some(d => d.toLowerCase().includes(query));
        const matchesIP = node.data?.ip_address?.toLowerCase().includes(query);
        return matchesName || matchesDomain || matchesIP;
      }

      return true;
    });

    // Filter edges to only show connections between visible nodes
    const visibleNodeIds = new Set(filteredNodes.map(n => n.id));
    let filteredEdges = allEdges.filter(edge =>
      visibleNodeIds.has(edge.source) && visibleNodeIds.has(edge.target)
    );

    setNodes(filteredNodes);
    setEdges(filteredEdges);
  }, [searchQuery, selectedLayers, showWarningsOnly, allNodes, allEdges, setNodes, setEdges]);

  const toggleLayer = (layer) => {
    setSelectedLayers(prev => {
      const newSet = new Set(prev);
      if (newSet.has(layer)) {
        newSet.delete(layer);
      } else {
        newSet.add(layer);
      }
      return newSet;
    });
  };

  if (loading) {
    return (
      <div style={{ padding: '2rem', textAlign: 'center' }}>
        <p>Loading network topology...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div style={{ padding: '2rem', textAlign: 'center', color: '#ef4444' }}>
        <p>Error: {error}</p>
      </div>
    );
  }

  return (
    <div style={{ width: '100%', height: '100%' }}>
      <div style={{
        background: '#1e293b',
        padding: '1rem',
        borderBottom: '1px solid #334155',
        color: 'white'
      }}>
        <h2 style={{ margin: 0, fontSize: '1.5rem' }}>Network Topology</h2>
        <p style={{ margin: '0.5rem 0 0 0', fontSize: '0.875rem', opacity: 0.7 }}>
          Left-to-right flow: Internet → Firewall → Gateway → Internal Services
        </p>

        {/* Filter Controls */}
        <div style={{ marginTop: '1rem', display: 'flex', gap: '1rem', flexWrap: 'wrap', alignItems: 'center' }}>
          {/* Search Box */}
          <input
            type="text"
            placeholder="Search nodes (name, domain, IP)..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            style={{
              padding: '0.5rem',
              borderRadius: '4px',
              border: '1px solid #475569',
              background: '#334155',
              color: 'white',
              fontSize: '0.875rem',
              minWidth: '250px',
              outline: 'none'
            }}
          />

          {/* Warnings Filter */}
          <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer', fontSize: '0.875rem' }}>
            <input
              type="checkbox"
              checked={showWarningsOnly}
              onChange={(e) => setShowWarningsOnly(e.target.checked)}
              style={{ cursor: 'pointer' }}
            />
            Show Warnings Only
          </label>

          {/* Layer Filters */}
          <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap', marginLeft: '1rem' }}>
            {layerOrder.map(layer => {
              const layerLabels = {
                internet: '🌍 Internet',
                firewall: '🛡️ Firewall',
                gateway: '🚪 Gateway',
                docker: '🐳 Docker',
                systemd: '⚙️ System',
                management: '🔧 Mgmt'
              };
              return (
                <button
                  key={layer}
                  onClick={() => toggleLayer(layer)}
                  style={{
                    padding: '0.25rem 0.5rem',
                    borderRadius: '4px',
                    border: '1px solid #475569',
                    background: selectedLayers.has(layer) ? '#3b82f6' : '#334155',
                    color: 'white',
                    fontSize: '0.75rem',
                    cursor: 'pointer',
                    opacity: selectedLayers.has(layer) ? 1 : 0.5
                  }}
                >
                  {layerLabels[layer] || layer}
                </button>
              );
            })}
          </div>
        </div>

        <div style={{ marginTop: '0.75rem', display: 'flex', gap: '1.5rem', flexWrap: 'wrap', alignItems: 'center' }}>
          {/* Layer Legend */}
          <div style={{ fontSize: '0.75rem', fontWeight: 'bold', opacity: 0.8 }}>Layers:</div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <div style={{ width: '20px', height: '12px', background: '#ef4444', borderRadius: '2px' }}></div>
            <span style={{ fontSize: '0.75rem' }}>🌍 Internet</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <div style={{ width: '20px', height: '12px', background: '#f97316', borderRadius: '2px' }}></div>
            <span style={{ fontSize: '0.75rem' }}>🛡️ Firewall</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <div style={{ width: '20px', height: '12px', background: '#3b82f6', borderRadius: '2px' }}></div>
            <span style={{ fontSize: '0.75rem' }}>🚪 Gateway</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <div style={{ width: '20px', height: '12px', background: '#10b981', borderRadius: '2px' }}></div>
            <span style={{ fontSize: '0.75rem' }}>🐳 Docker</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <div style={{ width: '20px', height: '12px', background: '#8b5cf6', borderRadius: '2px' }}></div>
            <span style={{ fontSize: '0.75rem' }}>⚙️ System</span>
          </div>

          {/* Indicators */}
          <div style={{ fontSize: '0.75rem', fontWeight: 'bold', opacity: 0.8, marginLeft: '1rem' }}>Indicators:</div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <span style={{ fontSize: '1rem' }}>⚠️</span>
            <span style={{ fontSize: '0.75rem' }}>Security Warning</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <span style={{ fontSize: '1rem' }}>🌐</span>
            <span style={{ fontSize: '0.75rem' }}>Has Domains</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <span style={{ fontSize: '1rem' }}>📦</span>
            <span style={{ fontSize: '0.75rem' }}>Project</span>
          </div>

          {/* Edge Types */}
          <div style={{ fontSize: '0.75rem', fontWeight: 'bold', opacity: 0.8, marginLeft: '1rem' }}>Connections:</div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <div style={{ width: '30px', height: '2px', background: '#10b981' }}></div>
            <span style={{ fontSize: '0.75rem' }}>HTTP</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <div style={{ width: '30px', height: '2px', background: '#f97316', borderTop: '2px dashed #f97316' }}></div>
            <span style={{ fontSize: '0.75rem' }}>IPC</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <div style={{ width: '30px', height: '2px', background: '#8b5cf6', borderTop: '2px dashed #8b5cf6' }}></div>
            <span style={{ fontSize: '0.75rem' }}>Depends On</span>
          </div>
        </div>
      </div>
      <div style={{ width: '100%', height: 'calc(100% - 140px)', position: 'relative' }}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onNodeClick={(event, node) => {
            setSelectedNode(node);
            setShowModal(true);
          }}
          fitView
          minZoom={0.1}
          maxZoom={2}
        >
          <SwimLanes nodes={nodes} />
          <Background />
          <Controls />
          <MiniMap
            nodeColor={node => {
              const fullNode = nodes.find(n => n.id === node.id);
              if (!fullNode || !fullNode.style) return '#6b7280';
              return fullNode.style.background || '#6b7280';
            }}
            maskColor="rgba(0, 0, 0, 0.6)"
          />
        </ReactFlow>
      </div>

      {/* Node Details Modal */}
      {showModal && selectedNode && (
        <div style={{
          position: 'fixed',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          backgroundColor: 'rgba(0, 0, 0, 0.7)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          zIndex: 1000
        }} onClick={() => setShowModal(false)}>
          <div style={{
            backgroundColor: '#1e293b',
            borderRadius: '8px',
            padding: '2rem',
            maxWidth: '800px',
            maxHeight: '80vh',
            overflow: 'auto',
            color: 'white',
            boxShadow: '0 20px 25px -5px rgba(0, 0, 0, 0.3)'
          }} onClick={(e) => e.stopPropagation()}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'start', marginBottom: '1.5rem' }}>
              <h2 style={{ margin: 0, fontSize: '1.5rem', fontWeight: 'bold' }}>{selectedNode.data.name}</h2>
              <button
                onClick={() => setShowModal(false)}
                style={{
                  background: 'none',
                  border: 'none',
                  color: '#94a3b8',
                  fontSize: '1.5rem',
                  cursor: 'pointer',
                  padding: '0 0.5rem'
                }}
              >×</button>
            </div>

            {/* Node Information */}
            <div style={{ marginBottom: '1.5rem' }}>
              <h3 style={{ fontSize: '1.1rem', marginBottom: '0.75rem', color: '#94a3b8' }}>Node Information</h3>
              <div style={{ display: 'grid', gridTemplateColumns: 'auto 1fr', gap: '0.5rem', fontSize: '0.9rem' }}>
                <strong>Layer:</strong>
                <span>{selectedNode.data.layer}</span>
                {selectedNode.data.ip_address && (
                  <>
                    <strong>IP Address:</strong>
                    <span>{selectedNode.data.ip_address}</span>
                  </>
                )}
                {selectedNode.data.project && (
                  <>
                    <strong>Project:</strong>
                    <span>{selectedNode.data.project}</span>
                  </>
                )}
                {selectedNode.data.domains && selectedNode.data.domains.length > 0 && (
                  <>
                    <strong>Domains:</strong>
                    <span>{selectedNode.data.domains.join(', ')}</span>
                  </>
                )}
                {selectedNode.data.warnings && selectedNode.data.warnings.length > 0 && (
                  <>
                    <strong>Warnings:</strong>
                    <span style={{ color: '#fbbf24' }}>{selectedNode.data.warnings.join(', ')}</span>
                  </>
                )}
              </div>
            </div>

            {/* Network Path */}
            <div style={{ marginBottom: '1.5rem' }}>
              <h3 style={{ fontSize: '1.1rem', marginBottom: '0.75rem', color: '#94a3b8' }}>Network Path (from External)</h3>
              <div style={{
                backgroundColor: '#0f172a',
                padding: '1rem',
                borderRadius: '4px',
                fontSize: '0.9rem',
                fontFamily: 'monospace'
              }}>
                {(() => {
                  const path = findNetworkPath(selectedNode.id, allNodes, allEdges);
                  return path.map((nodeId, index) => {
                    const node = allNodes.find(n => n.id === nodeId);
                    return (
                      <div key={nodeId} style={{ display: 'flex', alignItems: 'center', marginBottom: index < path.length - 1 ? '0.5rem' : 0 }}>
                        <span style={{
                          padding: '0.25rem 0.75rem',
                          backgroundColor: nodeId === selectedNode.id ? '#3b82f6' : '#334155',
                          borderRadius: '4px',
                          fontWeight: nodeId === selectedNode.id ? 'bold' : 'normal'
                        }}>
                          {node?.data?.name || nodeId}
                        </span>
                        {index < path.length - 1 && (
                          <span style={{ margin: '0 0.5rem', color: '#64748b' }}>→</span>
                        )}
                      </div>
                    );
                  });
                })()}
              </div>
            </div>

            {/* Relationships */}
            {(() => {
              const { incoming, outgoing } = getNodeRelationships(selectedNode.id, allEdges);
              return (
                <>
                  {incoming.length > 0 && (
                    <div style={{ marginBottom: '1.5rem' }}>
                      <h3 style={{ fontSize: '1.1rem', marginBottom: '0.75rem', color: '#94a3b8' }}>
                        Incoming Connections ({incoming.length})
                      </h3>
                      <div style={{ fontSize: '0.85rem' }}>
                        {incoming.map((edge, idx) => {
                          const sourceNode = allNodes.find(n => n.id === edge.source);
                          return (
                            <div key={idx} style={{
                              padding: '0.5rem',
                              backgroundColor: '#0f172a',
                              marginBottom: '0.5rem',
                              borderRadius: '4px'
                            }}>
                              <strong>{sourceNode?.data?.name || edge.source}</strong>
                              {edge.label && <span style={{ color: '#94a3b8' }}> ({edge.label})</span>}
                              {edge.protocol && <span style={{ color: '#64748b', fontSize: '0.75rem' }}> [{edge.protocol.toUpperCase()}]</span>}
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  )}

                  {outgoing.length > 0 && (
                    <div>
                      <h3 style={{ fontSize: '1.1rem', marginBottom: '0.75rem', color: '#94a3b8' }}>
                        Outgoing Connections ({outgoing.length})
                      </h3>
                      <div style={{ fontSize: '0.85rem' }}>
                        {outgoing.map((edge, idx) => {
                          const targetNode = allNodes.find(n => n.id === edge.target);
                          return (
                            <div key={idx} style={{
                              padding: '0.5rem',
                              backgroundColor: '#0f172a',
                              marginBottom: '0.5rem',
                              borderRadius: '4px'
                            }}>
                              <strong>{targetNode?.data?.name || edge.target}</strong>
                              {edge.label && <span style={{ color: '#94a3b8' }}> ({edge.label})</span>}
                              {edge.protocol && <span style={{ color: '#64748b', fontSize: '0.75rem' }}> [{edge.protocol.toUpperCase()}]</span>}
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  )}
                </>
              );
            })()}
          </div>
        </div>
      )}
    </div>
  );
}
