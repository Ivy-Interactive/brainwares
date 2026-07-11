import React, { useState, useEffect, useRef } from 'react';
import { marked } from 'marked';
import { 
  BookOpen, Search, ShieldCheck, AlertCircle, RefreshCw, 
  Tag, Link2, Share2, Compass, Network, FileCode, CheckCircle,
  ChevronUp, Zap
} from 'lucide-react';
import data from './data.json';

const renderer = new marked.Renderer();
renderer.link = (href, title, text) => {
  if (href && href.startsWith('wiki:')) {
    const noteName = href.replace('wiki:', '');
    return `<a href="#" class="wiki-link text-indigo-400 hover:text-indigo-300 font-semibold underline decoration-indigo-500/40" data-note="${noteName}">${text}</a>`;
  }
  let out = `<a href="${href || '#'}"`;
  if (title) {
    out += ` title="${title}"`;
  }
  out += `>${text}</a>`;
  return out;
};
marked.use({ renderer });

export default function App() {
  const [memories, setMemories] = useState(data.memories || []);
  const [selectedNoteName, setSelectedNoteName] = useState('index');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedTag, setSelectedTag] = useState(null);
  const [viewMode, setViewMode] = useState('doc');
  const [graphMode, setGraphMode] = useState(data.memories && data.memories.length > 150 ? 'local' : 'global');
  const [maxDepth, setMaxDepth] = useState(3);
  const [physicsEnabled, setPhysicsEnabled] = useState(false);
  const canvasRef = useRef(null);
  const zoomInButtonRef = useRef(null);
  const zoomOutButtonRef = useRef(null);
  const zoomResetButtonRef = useRef(null);
  const selectedNoteNameRef = useRef(selectedNoteName);
  const prevSelectedNoteNameRef = useRef(selectedNoteName);
  const prevGraphModeRef = useRef(graphMode);
  const zoomRef = useRef(null);
  const panRef = useRef(null);
  const nodePositionsRef = useRef({});

  useEffect(() => {
    selectedNoteNameRef.current = selectedNoteName;
  }, [selectedNoteName]);

  const getParentNodeName = (name) => {
    if (!name || name.toLowerCase() === 'index') return null;
    const parts = name.split('-');
    if (parts.length <= 1) {
      return 'index';
    }
    const parentId = parts.slice(0, -1).join('-');
    const parentMemory = memories.find(m => m && m.name && m.name.toLowerCase() === parentId);
    if (parentMemory) {
      return parentMemory.name;
    }
    for (let i = parts.length - 2; i >= 1; i--) {
      const ancestorId = parts.slice(0, i).join('-');
      const ancestorMemory = memories.find(m => m && m.name && m.name.toLowerCase() === ancestorId);
      if (ancestorMemory) {
        return ancestorMemory.name;
      }
    }
    return 'index';
  };

  const workspaceRoot = data.vault_path ? data.vault_path.replace(/\/\.brainwares\/?$/, '') : '';

  const openLocalFile = (filePath) => {
    if (!filePath) return;
    const fullPath = filePath.startsWith('/') ? filePath : `${workspaceRoot}/${filePath}`;
    fetch(`/api/open-file?path=${encodeURIComponent(fullPath)}`)
      .then(res => res.json())
      .then(resData => {
        if (!resData.success) {
          console.error('Failed to open file:', resData.error);
        }
      })
      .catch(err => {
        console.error('Error calling open-file API:', err);
      });
  };

  const preprocessMarkdown = (text) => {
    if (!text) return '';
    return text.replace(/\[\[(.*?)\]\]/g, (match, note) => {
      const parts = note.split('|');
      const target = parts[0].trim();
      const label = parts[1] ? parts[1].trim() : target;
      const normTarget = target.toLowerCase().replace(/ /g, '-').replace(/_/g, '-');
      return `[${label}](wiki:${normTarget})`;
    });
  };

  const selectedNote = memories.find(m => m && m.name && m.name.toLowerCase() === selectedNoteName.toLowerCase()) 
    || memories.find(m => m && m.name && m.name.toLowerCase() === 'index') 
    || memories[0];

  useEffect(() => {
    if (selectedNote) {
      setSelectedNoteName(selectedNote.name);
    }
  }, [selectedNote]);

  const handleHtmlClick = (e) => {
    const target = e.target.closest('[data-note]');
    if (target) {
      e.preventDefault();
      const noteName = target.getAttribute('data-note');
      setSelectedNoteName(noteName);
    }
  };

  const allTags = Array.from(new Set(memories.flatMap(m => m?.frontmatter?.tags || [])));

  const filteredNotes = memories.filter(m => {
    if (!m) return false;
    const nameLower = (m.name || '').toLowerCase();
    const titleLower = (m.frontmatter?.title || '').toLowerCase();
    const bodyLower = (m.body || '').toLowerCase();
    const query = searchQuery.toLowerCase();
    
    const matchesSearch = searchQuery === '' || 
      nameLower.includes(query) ||
      titleLower.includes(query) ||
      bodyLower.includes(query);
    
    const matchesTag = !selectedTag || (m.frontmatter?.tags || []).includes(selectedTag);
    return matchesSearch && matchesTag;
  });

  useEffect(() => {
    if (viewMode !== 'graph' || !canvasRef.current) return;
    const canvas = canvasRef.current;
    const ctx = canvas.getContext('2d');
    
    const resizeCanvas = () => {
      canvas.width = canvas.parentElement.clientWidth;
      canvas.height = canvas.parentElement.clientHeight || 500;
    };
    resizeCanvas();
    window.addEventListener('resize', () => {
      resizeCanvas();
      triggerRedraw();
    });

    // Restore or reset pan and zoom values
    const prevSelectedNoteName = prevSelectedNoteNameRef.current;
    prevSelectedNoteNameRef.current = selectedNoteName;

    if (prevGraphModeRef.current !== graphMode) {
      zoomRef.current = null;
      panRef.current = null;
      prevGraphModeRef.current = graphMode;
    } else if (graphMode === 'local' && prevSelectedNoteName !== selectedNoteName) {
      zoomRef.current = null;
      panRef.current = null;
    }

    let zoom = zoomRef.current !== null ? zoomRef.current : (graphMode === 'local' ? 1.0 : 0.25);
    let pan = panRef.current !== null ? panRef.current : { x: canvas.width / 2, y: canvas.height / 2 };
    let isPanning = false;
    let panStart = { x: 0, y: 0 };
    let draggedNode = null;
    let hoveredNode = null;

    // Dominant workspace prefix and branch computation helper
    const segmentCounts = {};
    let nonIndexCount = 0;
    memories.forEach(m => {
      if (!m) return;
      const id = (m.name || '').toLowerCase();
      if (id !== 'index') {
        const seg = id.split('-')[0];
        segmentCounts[seg] = (segmentCounts[seg] || 0) + 1;
        nonIndexCount++;
      }
    });
    
    let workspacePrefix = null;
    if (nonIndexCount > 0) {
      for (const [seg, count] of Object.entries(segmentCounts)) {
        if (count / nonIndexCount > 0.5) {
          workspacePrefix = seg;
          break;
        }
      }
    }

    const getBranchOfId = (id) => {
      if (id === 'index') return 'index';
      const parts = id.split('-');
      if (workspacePrefix && parts[0] === workspacePrefix && parts.length > 1) {
        return parts[1];
      }
      return parts[0];
    };

    const activeNoteName = selectedNoteNameRef.current.toLowerCase();
    const hasSavedPositions = Object.keys(nodePositionsRef.current).length > 0;

    // 1. Calculate nodes
    const nodes = memories.map((m) => {
      const id = (m.name || '').toLowerCase();
      const name = m.frontmatter?.title || m.name || '';
      const isGlobal = (m.file_path || '').includes('.config');
      
      let nodeRadius = 32;
      if (graphMode === 'local') {
        nodeRadius = id === activeNoteName ? 32 : 16;
      } else {
        const depth = id === 'index' ? 0 : id.split('-').length;
        if (depth === 0) {
          nodeRadius = 96;
        } else if (depth === 1) {
          nodeRadius = 64;
        } else if (depth === 2) {
          nodeRadius = 48;
        }
      }
      
      const saved = nodePositionsRef.current[id] || {};
      
      return {
        id,
        name,
        x: saved.x !== undefined ? saved.x : 0,
        y: saved.y !== undefined ? saved.y : 0,
        vx: saved.vx || 0,
        vy: saved.vy || 0,
        radius: nodeRadius,
        isGlobal,
      };
    });

    // 2. Build Tree Structure from Node IDs and run Radial Dendrogram Algorithm
    const root = { id: 'index', children: [], parent: null };
    const nodeMap = { 'index': root };
    
    nodes.forEach(n => {
      if (n.id !== 'index') {
        nodeMap[n.id] = { id: n.id, children: [], parent: null, node: n };
      }
    });
    
    nodes.forEach(n => {
      if (n.id === 'index') return;
      const parts = n.id.split('-');
      let parentId = 'index';
      if (parts.length > 1) {
        parentId = parts.slice(0, -1).join('-');
      }
      const parentNode = nodeMap[parentId] || root;
      parentNode.children.push(nodeMap[n.id]);
      nodeMap[n.id].parent = parentNode;
    });

    const countLeaves = (node) => {
      if (node.children.length === 0) return 1;
      return node.children.reduce((sum, child) => sum + countLeaves(child), 0);
    };

    const assignRadialLayout = (node, startAngle, endAngle, depth, siblingIndex = 0, siblingCount = 1, parentRadius = 0) => {
      const baseRadialStep = 450; // Increased base step to accommodate 4x larger circles
      
      const angleRange = endAngle - startAngle;
      const activeRange = angleRange * 0.95; // leave buffer to prevent sibling branch overlaps
      const midAngle = (startAngle + endAngle) / 2;
      
      // Calculate how much space is needed to fit siblingCount nodes with zero overlap along the arc.
      // Sibling nodes need to be spaced along the arc.
      // We look up the node radius or fallback to 48px.
      const nodeSize = node.node ? node.node.radius : 48;
      const requiredSpacing = nodeSize * 2.5; // Circle diameter + spacing buffer
      
      // Arc length L = Radius * activeRange. We want L >= requiredSpacing * siblingCount.
      // Therefore, the required radius R >= (requiredSpacing * siblingCount) / activeRange.
      const requiredRadius = (requiredSpacing * siblingCount) / Math.max(activeRange, 0.12);
      
      // The separation step is the distance from parentRadius needed to achieve this requiredRadius.
      const step = Math.max(baseRadialStep, requiredRadius - parentRadius);
      const radius = depth === 0 ? 0 : parentRadius + step;
      
      // Stagger leaf nodes of large branches to prevent overlapping on dense concentric arcs.
      // Leaf nodes (nodes with 0 children) will alternate depth slightly (+0px, +80px, +160px)
      // to distribute physical presence in a clean spiral/starburst pattern.
      let staggerOffset = 0;
      if (siblingCount > 5 && node.children.length === 0) {
        staggerOffset = (siblingIndex % 3) * 80;
      }
      
      const finalRadius = radius + staggerOffset;
      
      if (node.node) {
        node.node.x = Math.cos(midAngle) * finalRadius;
        node.node.y = Math.sin(midAngle) * finalRadius;
      } else if (node.id === 'index') {
        const idxNode = nodes.find(n => n.id === 'index');
        if (idxNode) {
          idxNode.x = 0;
          idxNode.y = 0;
        }
      }
      
      if (node.children.length === 0) return;
      
      const rangeStart = midAngle - activeRange / 2;
      
      node.children.sort((a, b) => a.id.localeCompare(b.id));
      
      const subtreeSizes = node.children.map(child => countLeaves(child));
      const totalSubtree = subtreeSizes.reduce((sum, s) => sum + s, 0) || 1;
      
      let currentAngle = rangeStart;
      node.children.forEach((child, idx) => {
        const slice = (subtreeSizes[idx] / totalSubtree) * activeRange;
        assignRadialLayout(child, currentAngle, currentAngle + slice, depth + 1, idx, node.children.length, radius);
        currentAngle += slice;
      });
    };

    // Only run layout if in global mode and positions haven't been saved yet
    if (graphMode === 'global' && !hasSavedPositions) {
      assignRadialLayout(root, 0, Math.PI * 2, 0, 0, 1, 0);
    }

    // 3. Calculate edges
    const edges = [];
    memories.forEach(m => {
      if (!m) return;
      const body = m.body || '';
      const matches = body.match(/\[\[(.*?)\]\]/g) || [];
      matches.forEach(match => {
        const target = match.replace(/\[\[|\]\]/g, '').split('|')[0].trim()
          .toLowerCase().replace(/ /g, '-').replace(/_/g, '-');
        
        const source_node = nodes.find(n => n.id === m.name.toLowerCase());
        const target_node = nodes.find(n => n.id === target);
        if (source_node && target_node) {
          edges.push({
            source: source_node,
            target: target_node,
          });
        }
      });
    });

    // 4. Filter activeNodes and activeEdges based on graphMode
    let activeNodes = [];
    let activeEdges = [];
    
    if (graphMode === 'local' && activeNoteName) {
      const connectedIds = new Set([activeNoteName]);
      
      edges.forEach(edge => {
        if (edge.source.id === activeNoteName) {
          connectedIds.add(edge.target.id);
        }
        if (edge.target.id === activeNoteName) {
          connectedIds.add(edge.source.id);
        }
      });
      
      activeNodes = nodes.filter(n => connectedIds.has(n.id));
      activeEdges = edges.filter(e => connectedIds.has(e.source.id) && connectedIds.has(e.target.id));
      
      // In local mode, override positions to layout neighbors in a simple circle around the selected node at (0,0)
      const selectedNode = activeNodes.find(n => n.id === activeNoteName);
      if (selectedNode) {
        selectedNode.x = 0;
        selectedNode.y = 0;
      }
      const neighbors = activeNodes.filter(n => n.id !== activeNoteName);
      neighbors.forEach((node, index) => {
        const angle = (index / neighbors.length) * Math.PI * 2;
        node.x = Math.cos(angle) * 150;
        node.y = Math.sin(angle) * 150;
      });
    } else {
      activeNodes = nodes;
      // In global graph view, filter out lines that cross different namespaces/branches
      activeEdges = edges.filter(edge => {
        const srcBranch = getBranchOfId(edge.source.id);
        const tgtBranch = getBranchOfId(edge.target.id);
        return srcBranch === tgtBranch || srcBranch === 'index' || tgtBranch === 'index';
      });
    }

    let animationId;
    let needsRedraw = true; // State tracking for dirty render strategy

    const triggerRedraw = () => {
      needsRedraw = true;
    };

    const step = () => {
      if (physicsEnabled) {
        // 1. Reset forces
        activeNodes.forEach(n => {
          n.fx = 0;
          n.fy = 0;
        });

        // 2. Repulsion (Coulomb's Law + overlapping spring)
        const kRepulsion = 1500;
        for (let i = 0; i < activeNodes.length; i++) {
          const n1 = activeNodes[i];
          for (let j = i + 1; j < activeNodes.length; j++) {
            const n2 = activeNodes[j];
            const dx = n1.x - n2.x;
            const dy = n1.y - n2.y;
            const distSq = dx * dx + dy * dy + 0.1;
            const dist = Math.sqrt(distSq);
            const minDist = n1.radius + n2.radius + 30;
            
            if (dist < minDist) {
              const force = (minDist - dist) * 0.15;
              const fx = (dx / dist) * force;
              const fy = (dy / dist) * force;
              n1.fx += fx;
              n1.fy += fy;
              n2.fx -= fx;
              n2.fy -= fy;
            } else {
              const force = kRepulsion / distSq;
              const fx = (dx / dist) * force;
              const fy = (dy / dist) * force;
              n1.fx += fx;
              n1.fy += fy;
              n2.fx -= fx;
              n2.fy -= fy;
            }
          }
        }

        // 3. Attraction along active edges (Hooke's spring)
        const kAttraction = 0.05;
        const desiredLength = 120;
        activeEdges.forEach(edge => {
          const n1 = edge.source;
          const n2 = edge.target;
          const dx = n1.x - n2.x;
          const dy = n1.y - n2.y;
          const dist = Math.sqrt(dx * dx + dy * dy) || 0.1;
          
          const force = (dist - desiredLength) * kAttraction;
          const fx = (dx / dist) * force;
          const fy = (dy / dist) * force;
          n1.fx -= fx;
          n1.fy -= fy;
          n2.fx += fx;
          n2.fy += fy;
        });

        // 4. Gravity pull to center (0,0)
        const kGravity = 0.012;
        activeNodes.forEach(n => {
          if (n.id === 'index') return;
          n.fx -= n.x * kGravity;
          n.fy -= n.y * kGravity;
        });

        // 5. Update positions & velocities
        activeNodes.forEach(n => {
          if (n.id === 'index' && graphMode === 'global') {
            n.x = 0;
            n.y = 0;
            return;
          }
          if (n === draggedNode) return;
          
          n.vx = (n.vx || 0) * 0.8 + n.fx;
          n.vy = (n.vy || 0) * 0.8 + n.fy;
          
          const speed = Math.sqrt(n.vx * n.vx + n.vy * n.vy);
          const maxSpeed = 12;
          if (speed > maxSpeed) {
            n.vx = (n.vx / speed) * maxSpeed;
            n.vy = (n.vy / speed) * maxSpeed;
          }
          
          n.x += n.vx;
          n.y += n.vy;
        });

        triggerRedraw();
      }

      if (needsRedraw) {
        needsRedraw = false;

        // Clear canvas context
        ctx.clearRect(0, 0, canvas.width, canvas.height);

        ctx.save();
        ctx.translate(pan.x, pan.y);
        ctx.scale(zoom, zoom);

        // 1. Frustum Bounding Box Calculation for dynamic screen space clipping (huge performance win)
        const pad = 80;
        const viewLeft = -pan.x / zoom - pad;
        const viewRight = (canvas.width - pan.x) / zoom + pad;
        const viewTop = -pan.y / zoom - pad;
        const viewBottom = (canvas.height - pan.y) / zoom + pad;

        // 2. Grid background drawn dynamically in visible bounding box
        ctx.strokeStyle = '#18181b';
        ctx.lineWidth = 1 / zoom;
        const gridSize = 40;
        const startX = Math.floor(viewLeft / gridSize) * gridSize;
        const endX = Math.ceil(viewRight / gridSize) * gridSize;
        const startY = Math.floor(viewTop / gridSize) * gridSize;
        const endY = Math.ceil(viewBottom / gridSize) * gridSize;

        ctx.beginPath();
        for (let x = startX; x <= endX; x += gridSize) {
          ctx.moveTo(x, startY);
          ctx.lineTo(x, endY);
        }
        for (let y = startY; y <= endY; y += gridSize) {
          ctx.moveTo(startX, y);
          ctx.lineTo(endX, y);
        }
        ctx.stroke();

        const activeId = selectedNoteNameRef.current.toLowerCase();

        // Filter and clip visible edges / nodes for viewport culling
        const visibleNodes = activeNodes.filter(node => 
          node.x >= viewLeft && node.x <= viewRight && 
          node.y >= viewTop && node.y <= viewBottom
        );

        // 3. Batched Draw Calls for Edges (Reduces web draw calls from 3,000 to exactly 3!)
        // Pass 1: Standard faint gray edges
        ctx.beginPath();
        ctx.strokeStyle = '#27272a';
        ctx.lineWidth = 1 / zoom;
        activeEdges.forEach(edge => {
          // Viewport clipping: draw edge only if at least one node is visible
          const isVisible = (edge.source.x >= viewLeft && edge.source.x <= viewRight && edge.source.y >= viewTop && edge.source.y <= viewBottom) ||
                            (edge.target.x >= viewLeft && edge.target.x <= viewRight && edge.target.y >= viewTop && edge.target.y <= viewBottom);
          if (!isVisible) return;

          const isConnectedToSelected = activeId && (
            edge.source.id === activeId || 
            edge.target.id === activeId
          );
          const isConnectedToHovered = hoveredNode && (
            edge.source.id === hoveredNode.id ||
            edge.target.id === hoveredNode.id
          );

          if (!isConnectedToSelected && !isConnectedToHovered) {
            ctx.moveTo(edge.source.x, edge.source.y);
            ctx.lineTo(edge.target.x, edge.target.y);
          }
        });
        ctx.stroke();

        // Pass 2: Hovered edge connections
        ctx.beginPath();
        ctx.strokeStyle = '#a5b4fc';
        ctx.lineWidth = 2 / zoom;
        activeEdges.forEach(edge => {
          const isConnectedToSelected = activeId && (
            edge.source.id === activeId || 
            edge.target.id === activeId
          );
          const isConnectedToHovered = hoveredNode && (
            edge.source.id === hoveredNode.id ||
            edge.target.id === hoveredNode.id
          );

          if (isConnectedToHovered && !isConnectedToSelected) {
            ctx.moveTo(edge.source.x, edge.source.y);
            ctx.lineTo(edge.target.x, edge.target.y);
          }
        });
        ctx.stroke();

        // Pass 3: Active selected edge connections
        ctx.beginPath();
        ctx.strokeStyle = '#818cf8';
        ctx.lineWidth = 2.5 / zoom;
        activeEdges.forEach(edge => {
          const isConnectedToSelected = activeId && (
            edge.source.id === activeId || 
            edge.target.id === activeId
          );

          if (isConnectedToSelected) {
            ctx.moveTo(edge.source.x, edge.source.y);
            ctx.lineTo(edge.target.x, edge.target.y);
          }
        });
        ctx.stroke();

        // 4. Batch drawing nodes to prevent styling canvas context switches
        // Determine if there is a dominant workspace prefix shared by most nodes
        const segmentCounts = {};
        let nonIndexCount = 0;
        activeNodes.forEach(n => {
          if (n.id !== 'index') {
            const seg = n.id.split('-')[0];
            segmentCounts[seg] = (segmentCounts[seg] || 0) + 1;
            nonIndexCount++;
          }
        });
        
        let workspacePrefix = null;
        if (nonIndexCount > 0) {
          for (const [seg, count] of Object.entries(segmentCounts)) {
            if (count / nonIndexCount > 0.5) {
              workspacePrefix = seg;
              break;
            }
          }
        }

        const getColorSegment = (id) => {
          if (id === 'index') return 'index';
          const parts = id.split('-');
          if (workspacePrefix && parts[0] === workspacePrefix && parts.length > 1) {
            return parts[1];
          }
          return parts[0];
        };

        const branchColors = {
          'index': '#e2e8f0',       // Zinc White for central index
          'framework': '#0ea5e9',   // Ocean Blue
          'ivy.internals': '#ec4899', // Hot Pink
          'internals': '#ec4899',   // Hot Pink
          'web': '#10b981',         // Emerald Green
          'tendril': '#f59e0b',     // Amber Gold
          'agent': '#6366f1',       // Indigo Purple
          'examples': '#8b5cf6',    // Deep Violet
          'connections': '#14b8a6', // Teal
          'src': '#a855f7',         // Purple
          'docs': '#f43f5e',        // Rose
        };
        
        const palette = [
          '#10b981', // Emerald
          '#f59e0b', // Amber
          '#ec4899', // Pink
          '#06b6d4', // Cyan
          '#8b5cf6', // Violet
          '#f43f5e', // Rose
          '#3b82f6', // Blue
          '#a855f7', // Purple
          '#14b8a6', // Teal
        ];
        
        const activeTopSegments = Array.from(new Set(
          activeNodes
            .map(n => getColorSegment(n.id))
            .filter(s => s !== 'index')
        ));

        // Group nodes by colors for grouped batch fills
        const nodesByColor = {};
        visibleNodes.forEach(node => {
          const isCurrent = node.id === activeId;
          const isHovered = hoveredNode && node.id === hoveredNode.id;
          if (isCurrent || isHovered) return; // Draw interacting nodes separately with stroke

          const colorSegment = getColorSegment(node.id);
          const segmentIndex = activeTopSegments.indexOf(colorSegment);
          const nodeColor = colorSegment === 'index' 
            ? '#e2e8f0' 
            : (branchColors[colorSegment] || palette[segmentIndex % palette.length]);

          if (!nodesByColor[nodeColor]) {
            nodesByColor[nodeColor] = [];
          }
          nodesByColor[nodeColor].push(node);
        });

        // Group 1: Zinc background shadows for all visible nodes
        ctx.beginPath();
        ctx.fillStyle = 'rgba(39, 39, 42, 0.2)';
        visibleNodes.forEach(node => {
          const isCurrent = node.id === activeId;
          const isHovered = hoveredNode && node.id === hoveredNode.id;
          if (isCurrent || isHovered) return;
          ctx.moveTo(node.x + node.radius + 4, node.y);
          ctx.arc(node.x, node.y, node.radius + 4, 0, Math.PI * 2);
        });
        ctx.fill();

        // Group 2: Draw the inner nodes in batch groups
        Object.entries(nodesByColor).forEach(([color, nodeList]) => {
          ctx.beginPath();
          ctx.fillStyle = color;
          nodeList.forEach(node => {
            ctx.moveTo(node.x + node.radius, node.y);
            ctx.arc(node.x, node.y, node.radius, 0, Math.PI * 2);
          });
          ctx.fill();
        });

        // Group 3: Interacting nodes (Selected, Hovered) drawn with individual strokes
        visibleNodes.forEach(node => {
          const isCurrent = node.id === activeId;
          const isHovered = hoveredNode && node.id === hoveredNode.id;
          if (!isCurrent && !isHovered) return;

          const colorSegment = getColorSegment(node.id);
          const segmentIndex = activeTopSegments.indexOf(colorSegment);
          const nodeColor = colorSegment === 'index' 
            ? '#e2e8f0' 
            : (branchColors[colorSegment] || palette[segmentIndex % palette.length]);

          // Glow shadow ring
          ctx.beginPath();
          const displayRadius = node.radius + (isCurrent ? 6 : 5);
          ctx.arc(node.x, node.y, displayRadius, 0, Math.PI * 2);
          ctx.fillStyle = isCurrent ? 'rgba(99, 102, 241, 0.25)' : 'rgba(165, 180, 252, 0.2)';
          ctx.fill();

          // Circle fill & outline stroke
          ctx.beginPath();
          ctx.arc(node.x, node.y, node.radius, 0, Math.PI * 2);
          
          if (isCurrent) {
            ctx.fillStyle = '#ffffff';
            ctx.strokeStyle = '#818cf8';
            ctx.lineWidth = 2.5 / zoom;
            ctx.stroke();
          } else {
            ctx.fillStyle = '#ffffff';
            ctx.strokeStyle = nodeColor;
            ctx.lineWidth = 2 / zoom;
            ctx.stroke();
          }
          ctx.fill();
        });

        // 5. Draw text labels
        visibleNodes.forEach(node => {
          const isCurrent = node.id === activeId;
          const isHovered = hoveredNode && node.id === hoveredNode.id;

          const shouldShowLabel = 
            zoom > 0.8 ||
            graphMode === 'local' || 
            isCurrent || 
            isHovered ||
            node.id === 'index' || 
            (selectedNoteNameRef.current && selectedNoteNameRef.current.toLowerCase().startsWith(node.id + '-'));

          if (shouldShowLabel) {
            ctx.font = `${(isCurrent || isHovered) ? 'bold' : ''} ${12 / zoom}px sans-serif`;
            ctx.fillStyle = (isCurrent || isHovered) ? '#ffffff' : '#a1a1aa';
            ctx.textAlign = 'center';
            ctx.fillText(node.name, node.x, node.y - node.radius - (8 / zoom));
          }
        });

        ctx.restore();
      }

      // Persist values to React refs so they survive useEffect teardowns on state changes
      zoomRef.current = zoom;
      panRef.current = pan;
      nodes.forEach(n => {
        nodePositionsRef.current[n.id] = { x: n.x, y: n.y, vx: n.vx, vy: n.vy };
      });

      animationId = requestAnimationFrame(step);
    };

    const getMousePos = (e) => {
      const rect = canvas.getBoundingClientRect();
      return {
        x: e.clientX - rect.left,
        y: e.clientY - rect.top,
      };
    };

    // Calculate mouse position inside virtual coordinate space
    const getVirtualMousePos = (e) => {
      const mousePos = getMousePos(e);
      return {
        x: (mousePos.x - pan.x) / zoom,
        y: (mousePos.y - pan.y) / zoom,
      };
    };

    const handleMouseDown = (e) => {
      const mousePos = getMousePos(e);
      const virtualPos = getVirtualMousePos(e);
      
      const clicked = activeNodes.find(node => {
        const dx = node.x - virtualPos.x;
        const dy = node.y - virtualPos.y;
        return Math.sqrt(dx * dx + dy * dy) < node.radius + 15;
      });

      if (clicked) {
        draggedNode = clicked;
        setSelectedNoteName(clicked.id);
        triggerRedraw();
      } else {
        isPanning = true;
        panStart.x = mousePos.x - pan.x;
        panStart.y = mousePos.y - pan.y;
      }
    };

    const handleMouseMove = (e) => {
      const mousePos = getMousePos(e);
      const virtualPos = getVirtualMousePos(e);
      
      // Update hovered node tracking dynamically
      const oldHover = hoveredNode;
      hoveredNode = activeNodes.find(node => {
        const dx = node.x - virtualPos.x;
        const dy = node.y - virtualPos.y;
        return Math.sqrt(dx * dx + dy * dy) < node.radius + 12;
      }) || null;
      if (hoveredNode !== oldHover) {
        triggerRedraw();
      }
      
      if (draggedNode) {
        draggedNode.x = virtualPos.x;
        draggedNode.y = virtualPos.y;
        triggerRedraw();
      } else if (isPanning) {
        pan.x = mousePos.x - panStart.x;
        pan.y = mousePos.y - panStart.y;
        triggerRedraw();
      }
    };

    const handleMouseUp = () => {
      draggedNode = null;
      isPanning = false;
    };

    // Mouse scroll wheel / trackpad zoom-around-cursor
    const handleWheel = (e) => {
      e.preventDefault();
      const zoomIntensity = 0.05;
      const mousePos = getMousePos(e);
      const zoomFactor = e.deltaY < 0 ? (1 + zoomIntensity) : (1 - zoomIntensity);
      const nextZoom = Math.max(0.1, Math.min(8, zoom * zoomFactor));
      
      pan.x = mousePos.x - (mousePos.x - pan.x) * (nextZoom / zoom);
      pan.y = mousePos.y - (mousePos.y - pan.y) * (nextZoom / zoom);
      triggerRedraw();
      zoom = nextZoom;
      triggerRedraw();
    };

    const handleDblClick = (e) => {
      const virtualPos = getVirtualMousePos(e);
      const clicked = activeNodes.find(node => {
        const dx = node.x - virtualPos.x;
        const dy = node.y - virtualPos.y;
        return Math.sqrt(dx * dx + dy * dy) < node.radius + 15;
      });

      if (clicked) {
        setSelectedNoteName(clicked.id);
        setViewMode('doc');
      }
    };

    // Zoom Overlay Controls (Ref Bound Click Listeners)
    const handleZoomIn = () => {
      const cx = canvas.width / 2;
      const cy = canvas.height / 2;
      const nextZoom = Math.min(8, zoom * 1.25);
      pan.x = cx - (cx - pan.x) * (nextZoom / zoom);
      pan.y = cy - (cy - pan.y) * (nextZoom / zoom);
      zoom = nextZoom;
      triggerRedraw();
      triggerRedraw();
      triggerRedraw();
    };
    const handleZoomOut = () => {
      const cx = canvas.width / 2;
      const cy = canvas.height / 2;
      const nextZoom = Math.max(0.1, zoom / 1.25);
      pan.x = cx - (cx - pan.x) * (nextZoom / zoom);
      pan.y = cy - (cy - pan.y) * (nextZoom / zoom);
      zoom = nextZoom;
      triggerRedraw();
      triggerRedraw();
      triggerRedraw();
    };
    const handleZoomReset = () => {
      zoom = graphMode === 'local' ? 1.0 : 0.25;
      pan = { x: canvas.width / 2, y: canvas.height / 2 };
      triggerRedraw();
    };

    canvas.addEventListener('mousedown', handleMouseDown);
    canvas.addEventListener('dblclick', handleDblClick);
    canvas.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    canvas.addEventListener('wheel', handleWheel, { passive: false });

    const zoomInBtn = zoomInButtonRef.current;
    const zoomOutBtn = zoomOutButtonRef.current;
    const zoomResetBtn = zoomResetButtonRef.current;
    if (zoomInBtn) zoomInBtn.addEventListener('click', handleZoomIn);
    if (zoomOutBtn) zoomOutBtn.addEventListener('click', handleZoomOut);
    if (zoomResetBtn) zoomResetBtn.addEventListener('click', handleZoomReset);

    animationId = requestAnimationFrame(step);

    return () => {
      cancelAnimationFrame(animationId);
      window.removeEventListener('resize', resizeCanvas);
      canvas.removeEventListener('mousedown', handleMouseDown);
      canvas.removeEventListener('dblclick', handleDblClick);
      canvas.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
      canvas.removeEventListener('wheel', handleWheel);
      if (zoomInBtn) zoomInBtn.removeEventListener('click', handleZoomIn);
      if (zoomOutBtn) zoomOutBtn.removeEventListener('click', handleZoomOut);
      if (zoomResetBtn) zoomResetBtn.removeEventListener('click', handleZoomReset);
    };
  }, [viewMode, memories, graphMode, maxDepth, selectedNoteName]);

  const totalNotes = memories.length;
  const globalNotesCount = memories.filter(m => m && (m.file_path || '').includes('.config')).length;
  const outdatedNotesCount = memories.filter(m => m && (m.frontmatter?.references || []).some(r => r.status && r.status !== 'OK')).length;

  return (
    <div className="flex h-screen bg-zinc-950 text-zinc-100 overflow-hidden font-sans">
      <div className="w-80 border-r border-zinc-900 bg-zinc-900/20 backdrop-blur-xl flex flex-col h-full select-none">
        <div className="p-5 border-b border-zinc-900 flex items-center space-x-3">
          <div className="p-2 bg-indigo-600/10 border border-indigo-500/20 rounded-xl text-indigo-400">
            <Compass size={22} className="animate-pulse" />
          </div>
          <div>
            <h1 className="text-md font-bold tracking-tight bg-gradient-to-r from-indigo-200 to-indigo-400 bg-clip-text text-transparent">
              Brainwares Vault
            </h1>
            <p className="text-xs text-zinc-500 font-mono">CLI UI v0.1.0</p>
          </div>
        </div>

        <div className="p-4 bg-zinc-900/40 border-b border-zinc-900 grid grid-cols-3 gap-2 text-center">
          <div className="p-2 bg-zinc-950/40 rounded-lg border border-zinc-900">
            <div className="text-xs text-zinc-500">Total</div>
            <div className="text-lg font-bold font-mono text-zinc-200">{totalNotes}</div>
          </div>
          <div className="p-2 bg-zinc-950/40 rounded-lg border border-zinc-900">
            <div className="text-xs text-zinc-500">Global</div>
            <div className="text-lg font-bold font-mono text-orange-500">{globalNotesCount}</div>
          </div>
          <div className="p-2 bg-zinc-950/40 rounded-lg border border-zinc-900">
            <div className="text-xs text-zinc-500">Outdated</div>
            <div className="text-lg font-bold font-mono text-red-400">{outdatedNotesCount}</div>
          </div>
        </div>

        <div className="p-4 space-y-3">
          <div className="relative">
            <Search className="absolute left-3 top-2.5 text-zinc-500" size={16} />
            <input
              type="text"
              placeholder="Search memories..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full bg-zinc-950 border border-zinc-900 rounded-lg pl-9 pr-4 py-2 text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-indigo-500/50 transition-colors"
            />
          </div>

          <div className="flex flex-wrap gap-1 items-center py-1">
            <button
              onClick={() => setSelectedTag(null)}
              className={`px-2 py-1 rounded text-xs transition-colors flex items-center space-x-1 ${!selectedTag ? 'bg-indigo-600/20 text-indigo-400 border border-indigo-500/20' : 'bg-zinc-950 text-zinc-500 hover:text-zinc-300'}`}
            >
              <span>All</span>
            </button>
            {allTags.map(tag => (
              <button
                key={tag}
                onClick={() => setSelectedTag(selectedTag === tag ? null : tag)}
                className={`px-2 py-1 rounded text-xs transition-colors flex items-center space-x-1 ${selectedTag === tag ? 'bg-indigo-600/20 text-indigo-400 border border-indigo-500/20' : 'bg-zinc-950 text-zinc-500 hover:text-zinc-300'}`}
              >
                <Tag size={10} />
                <span>{tag}</span>
              </button>
            ))}
          </div>
        </div>

        <div className="flex-1 overflow-y-auto px-4 pb-4 space-y-1">
          {filteredNotes.map(m => {
            const isCurrent = (m.name || '').toLowerCase() === (selectedNoteName || '').toLowerCase();
            const isGlobal = (m.file_path || '').includes('.config');
            const hasOutdated = (m.frontmatter?.references || []).some(r => r.status && r.status !== 'OK');

            return (
              <button
                key={m.name}
                onClick={() => setSelectedNoteName(m.name)}
                className={`w-full text-left p-3 rounded-xl transition-all duration-200 flex flex-col space-y-1 border ${isCurrent ? 'bg-indigo-600/10 border-indigo-500/40 text-indigo-200 shadow-lg shadow-indigo-500/5' : 'bg-transparent border-transparent hover:bg-zinc-900/40 hover:border-zinc-900 text-zinc-400 hover:text-zinc-200'}`}
              >
                <div className="flex justify-between items-start w-full">
                  <span className="font-medium text-sm truncate">{m.frontmatter.title || m.name}</span>
                  <div className="flex space-x-1 items-center flex-shrink-0">
                    {isGlobal && (
                      <span className="px-1.5 py-0.5 rounded text-[9px] bg-orange-950 border border-orange-500/20 text-orange-400 font-semibold font-mono">G</span>
                    )}
                    {hasOutdated && (
                      <AlertCircle size={12} className="text-red-400" />
                    )}
                  </div>
                </div>
                <div className="flex justify-between items-center w-full text-[10px] text-zinc-600 font-mono">
                  <span>[[{m.name}]]</span>
                  {m.frontmatter.tags && m.frontmatter.tags.length > 0 && (
                    <span className="truncate max-w-[120px]">#{m.frontmatter.tags[0]}</span>
                  )}
                </div>
              </button>
            );
          })}

          {filteredNotes.length === 0 && (
            <div className="p-8 text-center text-xs text-zinc-600">
              No matching memory notes.
            </div>
          )}
        </div>
      </div>

      <div className="flex-1 flex flex-col h-full bg-zinc-950 overflow-hidden relative">
        <div className="h-16 border-b border-zinc-900 px-6 flex justify-between items-center bg-zinc-900/10 backdrop-blur-md z-10 select-none">
          <div className="flex items-center space-x-4">
            <button
              onClick={() => setViewMode('doc')}
              className={`flex items-center space-x-2 px-3 py-1.5 rounded-lg text-sm transition-colors border ${viewMode === 'doc' ? 'bg-zinc-900 border-zinc-800 text-zinc-100' : 'bg-transparent border-transparent text-zinc-500 hover:text-zinc-300'}`}
            >
              <BookOpen size={16} />
              <span>Document</span>
            </button>
            <button
              onClick={() => setViewMode('graph')}
              className={`flex items-center space-x-2 px-3 py-1.5 rounded-lg text-sm transition-colors border ${viewMode === 'graph' ? 'bg-zinc-900 border-zinc-800 text-zinc-100' : 'bg-transparent border-transparent text-zinc-500 hover:text-zinc-300'}`}
            >
              <Network size={16} />
              <span>Visualizer</span>
            </button>
          </div>

          <div className="text-xs text-zinc-500 font-mono flex items-center space-x-2">
            <span>Workspace:</span>
            <span className="text-zinc-300 bg-zinc-900 px-2 py-1 rounded border border-zinc-800 truncate max-w-xs">
              {data.vault_path}
            </span>
          </div>
        </div>

        <div className="flex-1 overflow-hidden relative">
          {viewMode === 'doc' ? (
            <div className="flex h-full overflow-hidden">
              <div className="flex-1 overflow-y-auto px-10 py-8">
                {selectedNote ? (
                  <article className="max-w-3xl mx-auto prose prose-invert prose-indigo">
                    <div className="mb-8 border-b border-zinc-900 pb-6">
                      <div className="flex flex-wrap gap-2 mb-3">
                        {(selectedNote.frontmatter?.tags || []).map(t => (
                          <span key={t} className="px-2 py-0.5 rounded-full text-xs bg-zinc-900 border border-zinc-800 text-zinc-400 flex items-center space-x-1">
                            <Tag size={10} />
                            <span>{t}</span>
                          </span>
                        ))}
                        {(selectedNote.file_path || '').includes('.config') && (
                          <span className="px-2 py-0.5 rounded-full text-xs bg-orange-950 border border-orange-500/20 text-orange-400 font-semibold font-mono">
                            Global User Preference
                          </span>
                        )}
                      </div>

                      <h1 className="text-3xl font-bold tracking-tight text-zinc-100 mb-2">
                        {selectedNote.frontmatter?.title || selectedNote.name}
                      </h1>
                      
                      <div className="text-xs text-zinc-500 font-mono">
                        Last Updated: {selectedNote.frontmatter?.last_updated || 'Unknown'}
                      </div>
                    </div>

                    <div 
                      onClick={handleHtmlClick}
                      className="markdown-body text-zinc-300 leading-relaxed space-y-4"
                      dangerouslySetInnerHTML={{ __html: marked.parse(preprocessMarkdown(selectedNote.body)) }}
                    />
                  </article>
                ) : (
                  <div className="flex items-center justify-center h-full text-zinc-500">
                    No note selected. Select a note from the sidebar.
                  </div>
                )}
              </div>

              <div className="w-80 border-l border-zinc-900 bg-zinc-900/10 flex flex-col overflow-y-auto p-6 space-y-6">
                {selectedNote && (
                  <>
                    <div className="space-y-3">
                      <h3 className="text-xs font-bold uppercase tracking-wider text-zinc-500 flex items-center space-x-2">
                        <FileCode size={14} />
                        <span>Code References</span>
                      </h3>
                      
                      <div className="space-y-2">
                        {(selectedNote.frontmatter?.references || []).length > 0 ? (
                          (selectedNote.frontmatter?.references || []).map(ref => {
                            const isOk = ref.status === 'OK';
                            return (
                              <button
                                key={ref.file_path}
                                onClick={() => openLocalFile(ref.file_path)}
                                className="w-full text-left p-3 bg-zinc-900/40 hover:bg-zinc-900/70 border border-zinc-900 hover:border-zinc-800 rounded-xl transition-all duration-200 flex items-center justify-between group cursor-pointer text-left"
                              >
                                <div className="min-w-0 flex-1 pr-2">
                                  <div className="text-xs font-mono truncate text-zinc-300 group-hover:text-indigo-400 transition-colors" title={ref.file_path}>
                                    {(ref.file_path || '').split('/').pop()}
                                  </div>
                                  <div className="text-[10px] text-zinc-600 group-hover:text-zinc-500 transition-colors truncate">{ref.file_path}</div>
                                </div>
                                <div className="flex-shrink-0">
                                  {isOk ? (
                                    <span className="px-2 py-0.5 rounded-full text-[10px] bg-emerald-950 border border-emerald-500/20 text-emerald-400 font-medium flex items-center space-x-1">
                                      <CheckCircle size={10} />
                                      <span>OK</span>
                                    </span>
                                  ) : (
                                    <span className="px-2 py-0.5 rounded-full text-[10px] bg-red-950 border border-red-500/20 text-red-400 font-medium flex items-center space-x-1">
                                      <AlertCircle size={10} />
                                      <span>Outdated</span>
                                    </span>
                                  )}
                                </div>
                              </button>
                            );
                          })
                        ) : (
                          <div className="text-xs text-zinc-600 italic">No code references linked to this note.</div>
                        )}
                      </div>
                    </div>

                    <div className="space-y-3">
                      <h3 className="text-xs font-bold uppercase tracking-wider text-zinc-500 flex items-center space-x-2">
                        <Link2 size={14} />
                        <span>Backlinks</span>
                      </h3>

                      <div className="space-y-2">
                        {selectedNote.backlinks && selectedNote.backlinks.length > 0 ? (
                          selectedNote.backlinks.map(bl => (
                            <button
                              key={bl.source}
                              onClick={() => setSelectedNoteName(bl.source)}
                              className="w-full text-left p-3 bg-zinc-900/40 hover:bg-zinc-900/70 border border-zinc-900 hover:border-zinc-800 rounded-xl transition-all duration-200 flex flex-col space-y-1"
                            >
                              <div className="text-xs font-semibold text-zinc-300">
                                {bl.source}
                              </div>
                              <div className="text-[10px] text-zinc-500 italic truncate">
                                "{bl.context_line}"
                              </div>
                            </button>
                          ))
                        ) : (
                          <div className="text-xs text-zinc-600 italic">No incoming links to this note.</div>
                        )}
                      </div>
                    </div>
                  </>
                )}
              </div>
            </div>
          ) : (
            <div className="w-full h-full relative overflow-hidden bg-zinc-950">
              <canvas ref={canvasRef} className="block w-full h-full cursor-grab active:cursor-grabbing" />
              
              {graphMode === 'global' && (
                <div className="absolute top-6 left-6 p-1 bg-zinc-900/80 backdrop-blur-md border border-zinc-800 rounded-xl flex items-center space-x-1 select-none z-20 text-xs text-zinc-400 px-3 py-1.5">
                  <span className="font-semibold mr-2">Folder Depth:</span>
                  {[1, 2, 3, 4].map(d => (
                    <button
                      key={d}
                      onClick={() => setMaxDepth(d)}
                      className={`w-6 h-6 rounded flex items-center justify-center font-bold font-mono transition-colors ${maxDepth === d ? 'bg-indigo-600 text-zinc-100 shadow' : 'bg-transparent hover:text-zinc-200'}`}
                    >
                      {d}
                    </button>
                  ))}
                  <button
                    onClick={() => setMaxDepth(99)}
                    className={`px-2 h-6 rounded flex items-center justify-center font-bold transition-colors ${maxDepth === 99 ? 'bg-indigo-600 text-zinc-100 shadow' : 'bg-transparent hover:text-zinc-200'}`}
                  >
                    All
                  </button>
                </div>
              )}

              {graphMode === 'local' && (
                <div className="absolute top-6 left-6 p-1 bg-zinc-900/80 backdrop-blur-md border border-zinc-800 rounded-xl flex items-center space-x-2 select-none z-20 text-xs text-zinc-400 px-3 py-1.5">
                  <span className="font-semibold text-zinc-500">Parent:</span>
                  {(() => {
                    const parentName = getParentNodeName(selectedNoteName);
                    if (parentName) {
                      return (
                        <button
                          onClick={() => {
                            setSelectedNoteName(parentName);
                            const parentMemory = memories.find(m => m && m.name && m.name.toLowerCase() === parentName.toLowerCase());
                            if (parentMemory) {
                              openLocalFile(parentMemory.file_path);
                            }
                          }}
                          className="px-2.5 py-1 rounded-lg bg-indigo-600/20 text-indigo-400 border border-indigo-500/20 hover:bg-indigo-600 hover:text-zinc-100 hover:border-indigo-500 transition-all duration-200 font-semibold flex items-center space-x-1.5 shadow-md shadow-indigo-500/5 cursor-pointer"
                          title={`Navigate up to ${parentName}`}
                        >
                          <ChevronUp size={14} />
                          <span className="truncate max-w-[150px]">{parentName}</span>
                        </button>
                      );
                    } else {
                      return <span className="text-zinc-600 italic">None (Root)</span>;
                    }
                  })()}
                </div>
              )}

              <div className="absolute top-6 right-6 p-1 bg-zinc-900/80 backdrop-blur-md border border-zinc-800 rounded-xl flex items-center space-x-1 select-none z-20">
                {graphMode === 'global' && (
                  <>
                    <button
                      onClick={() => setPhysicsEnabled(!physicsEnabled)}
                      className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all duration-200 flex items-center space-x-1 cursor-pointer ${physicsEnabled ? 'bg-emerald-600/20 text-emerald-400 border border-emerald-500/20 hover:bg-emerald-600/30' : 'bg-transparent text-zinc-500 hover:text-zinc-300'}`}
                      title="Toggle Force-Directed Layout Physics"
                    >
                      <Zap size={12} className={physicsEnabled ? 'animate-bounce' : ''} />
                      <span>{physicsEnabled ? 'Physics: ON' : 'Physics: OFF'}</span>
                    </button>
                    <button
                      onClick={() => {
                        nodePositionsRef.current = {};
                        zoomRef.current = 0.25;
                        panRef.current = { x: canvasRef.current ? canvasRef.current.width / 2 : 500, y: canvasRef.current ? canvasRef.current.height / 2 : 250 };
                        setPhysicsEnabled(false);
                      }}
                      className="px-3 py-1.5 rounded-lg text-xs font-semibold text-zinc-500 hover:text-zinc-300 bg-transparent transition-all duration-200 cursor-pointer flex items-center space-x-1"
                      title="Reset nodes to default Radial Dendrogram positions"
                    >
                      <span>Reset Layout</span>
                    </button>
                    <div className="w-px h-4 bg-zinc-800 self-center mx-1" />
                  </>
                )}
                <button
                  onClick={() => setGraphMode('local')}
                  className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all duration-200 ${graphMode === 'local' ? 'bg-indigo-600 text-zinc-100 shadow-md shadow-indigo-500/10' : 'bg-transparent text-zinc-500 hover:text-zinc-300'}`}
                >
                  Local Graph
                </button>
                <button
                  onClick={() => setGraphMode('global')}
                  className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all duration-200 ${graphMode === 'global' ? 'bg-indigo-600 text-zinc-100 shadow-md shadow-indigo-500/10' : 'bg-transparent text-zinc-500 hover:text-zinc-300'}`}
                >
                  Global Graph
                </button>
              </div>
              
              <div className="absolute bottom-6 right-6 p-1 bg-zinc-900/80 backdrop-blur-md border border-zinc-800 rounded-xl flex items-center space-x-1 select-none z-20">
                <button
                  ref={zoomInButtonRef}
                  className="w-8 h-8 flex items-center justify-center rounded-lg text-sm font-bold text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
                  title="Zoom In"
                >
                  ＋
                </button>
                <button
                  ref={zoomOutButtonRef}
                  className="w-8 h-8 flex items-center justify-center rounded-lg text-sm font-bold text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
                  title="Zoom Out"
                >
                  －
                </button>
                <button
                  ref={zoomResetButtonRef}
                  className="w-8 h-8 flex items-center justify-center rounded-lg text-sm font-bold text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
                  title="Reset View"
                >
                  ⟲
                </button>
              </div>
              
              <div className="absolute bottom-6 left-6 p-4 bg-zinc-900/80 backdrop-blur-md border border-zinc-800 rounded-xl text-xs space-y-2 select-none text-zinc-300">
                <h4 className="font-bold text-zinc-200 mb-1">Legend</h4>
                <div className="flex items-center space-x-2">
                  <span className="w-3 h-3 rounded-full bg-indigo-400" />
                  <span>Current Node</span>
                </div>
                <div className="flex items-center space-x-2">
                  <span className="w-3 h-3 rounded-full bg-indigo-500" />
                  <span>Local Memory</span>
                </div>
                <div className="flex items-center space-x-2">
                  <span className="w-3 h-3 rounded-full bg-orange-500" />
                  <span>Global Preference</span>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}