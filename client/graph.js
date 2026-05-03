document.addEventListener('DOMContentLoaded', function() {
    loadAndRender();
    initPanel();
});

function initPanel() {
    var toggle = document.getElementById('panel-toggle');
    var panel = document.getElementById('panel');
    toggle.addEventListener('click', function() {
        panel.classList.toggle('collapsed');
        toggle.classList.toggle('open');
    });
    document.querySelectorAll('.section-header').forEach(function(hdr) {
        hdr.addEventListener('click', function(e) {
            if (e.target.closest('.section-actions')) return;
            var arrow = hdr.querySelector('.chevron');
            var body = hdr.nextElementSibling;
            if (arrow) arrow.classList.toggle('open');
            body.classList.toggle('open');
        });
    });
}

async function loadAndRender() {
    var data;
    try {
        var response = await fetch('/api/graph');
        if (!response.ok) throw new Error('HTTP ' + response.status);
        data = await response.json();
    } catch (err) {
        document.getElementById('error-message').textContent =
            'Failed to load graph data. Check that the cgraph server is running.';
        document.getElementById('error-state').style.display = 'block';
        return;
    }

    if (!data.nodes || data.nodes.length === 0) {
        document.getElementById('empty-message').textContent =
            'cgraph found no source files in ' + (data.project_name || 'this project') +
            '. Check that the path is correct and contains .ts, .tsx, or other supported files.';
        document.getElementById('empty-state').style.display = 'block';
        return;
    }

    document.getElementById('project-name').textContent = data.project_name || '';
    var s = data.stats;
    document.getElementById('stats').textContent =
        s.files + ' files • ' + s.symbols + ' symbols • ' + s.edges + ' edges • ' + s.elapsed_ms + 'ms';

    var width = window.innerWidth;
    var height = window.innerHeight - 40;

    var svg = d3.select('#graph').append('svg').attr('width', width).attr('height', height);
    var defs = svg.append('defs');

    defs.append('marker').attr('id', 'arrow')
        .attr('viewBox', '-0 -5 10 10').attr('refX', 0).attr('refY', 0)
        .attr('orient', 'auto').attr('markerWidth', 6).attr('markerHeight', 4)
      .append('path').attr('d', 'M 0,-5 L 10,0 L 0,5').attr('fill', '#444');

    defs.append('marker').attr('id', 'arrow-active')
        .attr('viewBox', '-0 -5 10 10').attr('refX', 0).attr('refY', 0)
        .attr('orient', 'auto').attr('markerWidth', 6).attr('markerHeight', 4)
      .append('path').attr('d', 'M 0,-5 L 10,0 L 0,5').attr('fill', '#a882ff');

    var g = svg.append('g');
    var currentZoom = 1;

    // Focus state (declared before zoomBehavior since zoom callback references focusActive)
    var focusActive = false;
    var focusedNodeId = null;

    // === Navigation History (INTR-08, D-75) ===

    var historyStack = [];
    var historyIndex = -1;
    var MAX_HISTORY = 50;
    var navigating = false; // guard to prevent push during back/forward

    function pushHistory(targetNode) {
        if (navigating) return;
        // Truncate forward history on new navigation
        historyStack = historyStack.slice(0, historyIndex + 1);
        historyStack.push({
            id: targetNode.id,
            name: targetNode.name || targetNode.filename || targetNode.id.split('/').pop()
        });
        if (historyStack.length > MAX_HISTORY) historyStack.shift();
        historyIndex = historyStack.length - 1;
        updateNavUI();
    }

    function navigateBack() {
        if (historyIndex <= 0) return;
        navigating = true;
        historyIndex--;
        var entry = historyStack[historyIndex];
        var targetNode = nodes.find(function(n) { return n.id === entry.id; });
        if (targetNode) {
            flyToNode(targetNode);
            activateFocus(targetNode);
        }
        navigating = false;
        updateNavUI();
    }

    function navigateForward() {
        if (historyIndex >= historyStack.length - 1) return;
        navigating = true;
        historyIndex++;
        var entry = historyStack[historyIndex];
        var targetNode = nodes.find(function(n) { return n.id === entry.id; });
        if (targetNode) {
            flyToNode(targetNode);
            activateFocus(targetNode);
        }
        navigating = false;
        updateNavUI();
    }

    function updateNavUI() {
        var btnBack = document.getElementById('btn-back');
        var btnForward = document.getElementById('btn-forward');

        btnBack.disabled = historyIndex <= 0;
        btnForward.disabled = historyIndex >= historyStack.length - 1;

        updateBreadcrumb();
    }

    function updateBreadcrumb() {
        var bc = document.getElementById('breadcrumb');
        if (historyStack.length === 0) {
            bc.style.display = 'none';
            return;
        }
        bc.style.display = 'flex';
        bc.innerHTML = ''; // Safe: we rebuild with textContent below

        historyStack.forEach(function(entry, idx) {
            if (idx > 0) {
                var sep = document.createElement('span');
                sep.textContent = '>';
                sep.style.color = '#666';
                sep.style.margin = '0 4px';
                bc.appendChild(sep);
            }

            var crumb = document.createElement('span');
            crumb.textContent = entry.name;
            crumb.style.cursor = 'pointer';
            crumb.style.color = idx === historyIndex ? '#7f6df2' : '#999';
            crumb.style.fontWeight = idx === historyIndex ? '500' : '400';

            crumb.addEventListener('click', (function(i) {
                return function() {
                    navigating = true;
                    historyIndex = i;
                    var target = nodes.find(function(n) { return n.id === historyStack[i].id; });
                    if (target) {
                        flyToNode(target);
                        activateFocus(target);
                    }
                    navigating = false;
                    updateNavUI();
                };
            })(idx));

            bc.appendChild(crumb);
        });
    }

    // Wire back/forward buttons
    document.getElementById('btn-back').addEventListener('click', navigateBack);
    document.getElementById('btn-forward').addEventListener('click', navigateForward);

    // Alt+Left / Alt+Right keyboard shortcuts
    document.addEventListener('keydown', function(e) {
        if (e.altKey && e.key === 'ArrowLeft') {
            e.preventDefault();
            navigateBack();
        }
        if (e.altKey && e.key === 'ArrowRight') {
            e.preventDefault();
            navigateForward();
        }
    });

    var zoomBehavior = d3.zoom().scaleExtent([0.1, 10]).on('zoom', function(event) {
        g.attr('transform', event.transform);
        currentZoom = event.transform.k;
        if (!hoverActive && !focusActive) updateLabelVisibility();
    });
    svg.call(zoomBehavior);

    var nodes = data.nodes.map(function(d) { return Object.assign({}, d); });

    // Separate file-level and symbol-level edges
    var allEdgeData = data.edges.map(function(d) { return Object.assign({}, d); });
    var edges = allEdgeData.filter(function(e) { return e.source.indexOf('::') === -1 && e.target.indexOf('::') === -1; });
    var symbolEdges = data.edges.filter(function(e) { return e.source.indexOf('::') !== -1 || e.target.indexOf('::') !== -1; }).map(function(d) { return Object.assign({}, d); });

    // Store all symbols grouped by file for expand
    var symbolsByFile = {};
    (data.symbols || []).forEach(function(s) {
        if (!symbolsByFile[s.file_path]) symbolsByFile[s.file_path] = [];
        symbolsByFile[s.file_path].push(Object.assign({}, s));
    });

    // Symbol adjacency map (precomputed from symbol edges)
    var symbolAdjacency = new Map();
    symbolEdges.forEach(function(e) {
        var src = typeof e.source === 'object' ? e.source.id : e.source;
        var tgt = typeof e.target === 'object' ? e.target.id : e.target;
        if (!symbolAdjacency.has(src)) symbolAdjacency.set(src, new Set());
        if (!symbolAdjacency.has(tgt)) symbolAdjacency.set(tgt, new Set());
        symbolAdjacency.get(src).add(tgt);
        symbolAdjacency.get(tgt).add(src);
    });

    var adjacency = new Map();
    nodes.forEach(function(n) { adjacency.set(n.id, new Set()); });

    // Node color palette (D-72)
    var NODE_COLORS = {
        'function': '#2dd4bf', 'class': '#f87171', 'type': '#fbbf24',
        'interface': '#fbbf24', 'hook': '#a78bfa', 'enum': '#4ade80',
        'module': '#555', 'file': '#555'
    };
    function nodeColor(d) {
        if (d._isSymbol) return NODE_COLORS[d.kind] || '#555';
        return '#555';
    }

    var expandMode = 'orbital';
    var expandedFiles = new Set(); // Set of file node IDs currently expanded

    var simulation = d3.forceSimulation(nodes)
        .force('link', d3.forceLink(edges).id(function(d) { return d.id; }).distance(50).strength(0.9))
        .force('charge', d3.forceManyBody().strength(-60))
        .force('center', d3.forceCenter(width / 2, height / 2).strength(0.12))
        .force('collide', d3.forceCollide().radius(function(d) { return d.radius + 8; }))
        .force('x', d3.forceX(width / 2).strength(0.06))
        .force('y', d3.forceY(height / 2).strength(0.06))
        .stop();

    simulation.tick(300);

    edges.forEach(function(e) {
        var srcId = typeof e.source === 'object' ? e.source.id : e.source;
        var tgtId = typeof e.target === 'object' ? e.target.id : e.target;
        if (adjacency.has(srcId)) adjacency.get(srcId).add(tgtId);
        if (adjacency.has(tgtId)) adjacency.get(tgtId).add(srcId);
    });

    function adjustedEndpoint(source, target, targetRadius) {
        var dx = target.x - source.x;
        var dy = target.y - source.y;
        var dist = Math.sqrt(dx * dx + dy * dy);
        if (dist === 0) return { x: target.x, y: target.y };
        return {
            x: target.x - (dx / dist) * (targetRadius + 6),
            y: target.y - (dy / dist) * (targetRadius + 6)
        };
    }

    var linkGroup = g.append('g').attr('class', 'edges');
    var link = linkGroup.selectAll('line').data(edges).join('line')
        .attr('stroke', '#444').attr('stroke-opacity', 0.25).attr('marker-end', 'url(#arrow)');

    var nodeGroup = g.append('g').attr('class', 'nodes');
    var node = nodeGroup.selectAll('circle')
        .data(nodes, function(d) { return d.id; })
        .join('circle')
        .attr('r', function(d) { return d.radius; })
        .attr('fill', function(d) { return nodeColor(d); })
        .attr('stroke', 'none').style('cursor', 'grab');

    var labelGroup = g.append('g').attr('class', 'labels');
    var labels = labelGroup.selectAll('text').data(nodes).join('text')
        .attr('text-anchor', 'middle').attr('fill', '#999')
        .attr('font-size', '11px').attr('pointer-events', 'none')
        .text(function(d) { return d.filename; });

    var nodeSizeScale = 1;

    function updatePositions() {
        node.attr('cx', function(d) { return d.x; }).attr('cy', function(d) { return d.y; });
        labels.attr('x', function(d) { return d.x; })
              .attr('y', function(d) { return d.y + d.radius * nodeSizeScale + 6 + 11; });
        link.each(function(d) {
            var src = typeof d.source === 'object' ? d.source : null;
            var tgt = typeof d.target === 'object' ? d.target : null;
            if (!src || !tgt) return;
            var ep = adjustedEndpoint(src, tgt, tgt.radius * nodeSizeScale);
            d3.select(this).attr('x1', src.x).attr('y1', src.y).attr('x2', ep.x).attr('y2', ep.y);
        });
    }

    updatePositions();
    simulation.on('tick', updatePositions);

    var drag = d3.drag()
        .on('start', function(event, d) {
            if (!event.active) simulation.alphaTarget(0.3).restart();
            d.fx = d.x; d.fy = d.y;
            d3.select(this).style('cursor', 'grabbing');
        })
        .on('drag', function(event, d) { d.fx = event.x; d.fy = event.y; })
        .on('end', function(event, d) {
            if (!event.active) simulation.alphaTarget(0);
            // Keep stacked nodes fixed
            if (!(d._isSymbol && expandMode === 'stacked')) {
                d.fx = null; d.fy = null;
            }
            d3.select(this).style('cursor', 'grab');
        });
    node.call(drag);

    function updateLabelVisibility() {
        var showLabels = document.getElementById('toggle-labels').checked;
        labels.style('opacity', !showLabels ? 0 : currentZoom < 0.4 ? 0 : Math.min(1, (currentZoom - 0.3) * 3));
    }

    // === NodeExpander (D-70, D-71, D-72) ===

    function expandFileNode(fileNode) {
        var fileId = fileNode.id;
        if (expandedFiles.has(fileId)) return;
        expandedFiles.add(fileId);

        var syms = symbolsByFile[fileId] || [];
        if (syms.length === 0) return;

        var symbolNodes = syms.map(function(s) {
            var sn = {
                id: s.id,
                name: s.name,
                kind: s.kind,
                file_path: s.file_path,
                radius: 6,
                _isSymbol: true,
                _parentId: fileId,
                is_dead_code: s.is_dead_code,
                dead_code_confidence: s.dead_code_confidence
            };
            return sn;
        });

        // Position symbols based on current expand mode
        positionSymbols(fileNode, symbolNodes);

        // Add symbol nodes to the shared nodes array
        symbolNodes.forEach(function(sn) { nodes.push(sn); });

        // Add parent-child edges (file -> symbol)
        var parentEdges = symbolNodes.map(function(sn) {
            return { source: fileId, target: sn.id, edge_type: 'parent_child', _isParentEdge: true };
        });
        parentEdges.forEach(function(e) { edges.push(e); });

        // Add symbol-level edges between expanded symbols
        symbolEdges.forEach(function(se) {
            var src = typeof se.source === 'object' ? se.source.id : se.source;
            var tgt = typeof se.target === 'object' ? se.target.id : se.target;
            var srcExpanded = nodes.some(function(n) { return n.id === src; });
            var tgtExpanded = nodes.some(function(n) { return n.id === tgt; });
            if (srcExpanded && tgtExpanded) {
                // Check if already in edges
                var exists = edges.some(function(e) {
                    var es = typeof e.source === 'object' ? e.source.id : e.source;
                    var et = typeof e.target === 'object' ? e.target.id : e.target;
                    return es === src && et === tgt;
                });
                if (!exists) {
                    edges.push({ source: src, target: tgt, edge_type: se.edge_type });
                }
            }
        });

        // Update adjacency map with symbol connections
        symbolNodes.forEach(function(sn) {
            if (!adjacency.has(sn.id)) adjacency.set(sn.id, new Set());
            adjacency.get(sn.id).add(fileId);
            if (adjacency.has(fileId)) adjacency.get(fileId).add(sn.id);
            // Merge symbol adjacency
            var symNeighbors = symbolAdjacency.get(sn.id);
            if (symNeighbors) {
                symNeighbors.forEach(function(nid) {
                    if (nodes.some(function(n) { return n.id === nid; })) {
                        adjacency.get(sn.id).add(nid);
                        if (!adjacency.has(nid)) adjacency.set(nid, new Set());
                        adjacency.get(nid).add(sn.id);
                    }
                });
            }
        });

        rebuildSimulation();
    }

    function collapseFileNode(fileId) {
        if (!expandedFiles.has(fileId)) return;
        expandedFiles.delete(fileId);

        // Remove symbol nodes
        var removedIds = new Set();
        nodes = nodes.filter(function(n) {
            if (n._parentId === fileId) { removedIds.add(n.id); return false; }
            return true;
        });

        // Remove edges referencing removed nodes
        edges = edges.filter(function(e) {
            var src = typeof e.source === 'object' ? e.source.id : e.source;
            var tgt = typeof e.target === 'object' ? e.target.id : e.target;
            return !removedIds.has(src) && !removedIds.has(tgt);
        });

        // Clean adjacency
        removedIds.forEach(function(rid) {
            adjacency.delete(rid);
            adjacency.forEach(function(neighbors) { neighbors.delete(rid); });
        });

        rebuildSimulation();
    }

    function positionSymbols(fileNode, symbolNodes) {
        var mode = expandMode;
        var count = symbolNodes.length;

        if (mode === 'orbital') {
            var orbitalRadius = 40;
            symbolNodes.forEach(function(sn, i) {
                var angle = (2 * Math.PI * i) / count - Math.PI / 2;
                sn.x = fileNode.x + orbitalRadius * Math.cos(angle);
                sn.y = fileNode.y + orbitalRadius * Math.sin(angle);
                // No fx/fy -- let simulation settle
            });
        } else if (mode === 'stacked') {
            var spacing = 18;
            var startY = fileNode.y + fileNode.radius + 12;
            symbolNodes.forEach(function(sn, i) {
                sn.x = fileNode.x;
                sn.y = startY + i * spacing;
                sn.fx = fileNode.x; // fixed position for stacked mode
                sn.fy = startY + i * spacing;
            });
        } else { // force-integrated
            // Start near parent, let simulation position them
            symbolNodes.forEach(function(sn) {
                sn.x = fileNode.x + (Math.random() - 0.5) * 30;
                sn.y = fileNode.y + (Math.random() - 0.5) * 30;
            });
        }
    }

    function rebuildSimulation() {
        simulation.nodes(nodes);
        simulation.force('link', d3.forceLink(edges).id(function(d) { return d.id; }).distance(function(d) {
            return d._isParentEdge ? 25 : 50;
        }).strength(0.9));

        // Rejoin node circles with stable keys
        node = nodeGroup.selectAll('circle')
            .data(nodes, function(d) { return d.id; })
            .join(
                function(enter) {
                    return enter.append('circle')
                        .attr('r', 0)
                        .attr('fill', function(d) { return nodeColor(d); })
                        .attr('stroke', 'none')
                        .style('cursor', 'grab')
                        .call(drag)
                        .transition().duration(300).attr('r', function(d) { return d.radius * nodeSizeScale; });
                },
                function(update) { return update; },
                function(exit) {
                    return exit.transition().duration(200).attr('r', 0).remove();
                }
            );

        // Rewire hover and click on the new selection
        wireNodeEvents(node);

        // Rejoin labels with stable keys
        labels = labelGroup.selectAll('text')
            .data(nodes, function(d) { return d.id; })
            .join(
                function(enter) {
                    return enter.append('text')
                        .attr('text-anchor', 'middle').attr('fill', '#999')
                        .attr('font-size', function(d) { return d._isSymbol ? '10px' : '11px'; })
                        .attr('pointer-events', 'none')
                        .text(function(d) { return d._isSymbol ? d.name : d.filename; })
                        .style('opacity', 0).transition().duration(300).style('opacity', 1);
                },
                function(update) { return update; },
                function(exit) { return exit.transition().duration(200).style('opacity', 0).remove(); }
            );

        // Rejoin links with stable keys
        link = linkGroup.selectAll('line')
            .data(edges, function(d) {
                var s = typeof d.source === 'object' ? d.source.id : d.source;
                var t = typeof d.target === 'object' ? d.target.id : d.target;
                return s + '->' + t;
            })
            .join(
                function(enter) {
                    return enter.append('line')
                        .attr('stroke', '#444')
                        .attr('stroke-opacity', function(d) { return d._isParentEdge ? 0.15 : 0.25; })
                        .attr('stroke-dasharray', function(d) { return d._isParentEdge ? '2 2' : null; })
                        .attr('marker-end', function(d) { return d._isParentEdge ? 'none' : 'url(#arrow)'; });
                },
                function(update) { return update; },
                function(exit) { return exit.remove(); }
            );

        simulation.alpha(0.3).restart();
    }

    // Expand mode change handler
    document.getElementById('expand-mode').addEventListener('change', function() {
        expandMode = this.value;
        // Re-expand currently expanded nodes in new mode
        var expanded = Array.from(expandedFiles);
        expanded.forEach(function(fileId) { collapseFileNode(fileId); });
        expanded.forEach(function(fileId) {
            var fileNode = nodes.find(function(n) { return n.id === fileId; });
            if (fileNode) expandFileNode(fileNode);
        });
    });

    // === FocusMode (D-74) ===

    function wireNodeEvents(sel) {
        sel.on('mouseenter', function(event, d) {
            if (focusActive) return;
            hoverActive = true;
            var connected = adjacency.get(d.id) || new Set();
            node.transition().duration(FADE_IN).ease(d3.easeCubicOut)
                .attr('fill', function(n) {
                    if (n.id === d.id) return '#7f6df2';
                    if (connected.has(n.id)) return '#a882ff';
                    return nodeColor(n);
                })
                .style('opacity', function(n) {
                    return (n.id === d.id || connected.has(n.id)) ? 1 : 0.12;
                });
            var showLabels = document.getElementById('toggle-labels').checked;
            labels.transition().duration(FADE_IN).ease(d3.easeCubicOut)
                .style('opacity', function(n) {
                    if (!showLabels || currentZoom < 0.4) return 0;
                    return (n.id === d.id || connected.has(n.id)) ? 1 : 0.06;
                });
            link.transition().duration(FADE_IN).ease(d3.easeCubicOut)
                .attr('stroke', function(e) {
                    var si = typeof e.source === 'object' ? e.source.id : e.source;
                    var ti = typeof e.target === 'object' ? e.target.id : e.target;
                    return (si === d.id || ti === d.id) ? '#a882ff' : '#444';
                })
                .attr('stroke-opacity', function(e) {
                    var si = typeof e.source === 'object' ? e.source.id : e.source;
                    var ti = typeof e.target === 'object' ? e.target.id : e.target;
                    return (si === d.id || ti === d.id) ? 0.7 : 0.04;
                })
                .attr('marker-end', function(e) {
                    var si = typeof e.source === 'object' ? e.source.id : e.source;
                    var ti = typeof e.target === 'object' ? e.target.id : e.target;
                    return (si === d.id || ti === d.id) ? 'url(#arrow-active)' : 'url(#arrow)';
                });

            // Tooltip
            var counts = d.export_counts || {};
            if (d._isSymbol) {
                var tooltip = document.getElementById('tooltip');
                tooltip.querySelector('.tooltip-path').textContent = d.file_path;
                tooltip.querySelector('.tooltip-exports').textContent = d.kind;
                tooltip.querySelector('.tooltip-edges').textContent = d.name;
                tooltip.style.display = 'block';
                tooltip.style.left = (event.pageX + 12) + 'px';
                tooltip.style.top = event.pageY + 'px';
            } else {
                var kinds = [['functions', counts.functions || 0], ['classes', counts.classes || 0],
                    ['types', counts.types || 0], ['interfaces', counts.interfaces || 0],
                    ['hooks', counts.hooks || 0], ['enums', counts.enums || 0]];
                var parts = kinds.filter(function(k) { return k[1] > 0; }).map(function(k) { return k[1] + ' ' + k[0]; });
                var tooltip = document.getElementById('tooltip');
                tooltip.querySelector('.tooltip-path').textContent = d.path || d.file_path;
                tooltip.querySelector('.tooltip-exports').textContent = parts.length > 0 ? parts.join(', ') : 'no exports';
                tooltip.querySelector('.tooltip-edges').textContent = (d.incoming || 0) + ' incoming + ' + (d.outgoing || 0) + ' outgoing';
                tooltip.style.display = 'block';
                tooltip.style.left = (event.pageX + 12) + 'px';
                tooltip.style.top = event.pageY + 'px';
            }
        });

        sel.on('mousemove', function(event) {
            var t = document.getElementById('tooltip');
            t.style.left = (event.pageX + 12) + 'px'; t.style.top = event.pageY + 'px';
        });

        sel.on('mouseleave', function() {
            if (focusActive) return;
            hoverActive = false;
            node.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
                .attr('fill', function(n) { return nodeColor(n); })
                .style('opacity', 1);
            labels.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
                .style('opacity', !document.getElementById('toggle-labels').checked ? 0 : currentZoom < 0.4 ? 0 : 1);
            link.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
                .attr('stroke', '#444').attr('stroke-opacity', 0.25).attr('marker-end', 'url(#arrow)');
            document.getElementById('tooltip').style.display = 'none';
        });

        sel.on('click', function(event, d) {
            event.stopPropagation();

            // If file node: toggle expand/collapse
            if (!d._isSymbol) {
                if (expandedFiles.has(d.id)) {
                    collapseFileNode(d.id);
                } else {
                    expandFileNode(d);
                }
            }

            // Blast radius mode takes priority over normal focus
            if (blastRadiusActive) {
                showBlastRadius(d);
                return;
            }

            // Activate focus mode
            activateFocus(d);
        });
    }

    // Wire the initial node selection
    wireNodeEvents(node);

    // Hover highlight state variables
    var hoverActive = false;
    var FADE_IN = 250, FADE_OUT = 400;

    function activateFocus(d) {
        focusActive = true;
        focusedNodeId = d.id;
        var connected = adjacency.get(d.id) || new Set();

        node.transition().duration(FADE_IN).ease(d3.easeCubicOut)
            .attr('fill', function(n) {
                if (n.id === d.id) return '#7f6df2';
                if (connected.has(n.id)) return nodeColor(n);
                return nodeColor(n);
            })
            .style('opacity', function(n) {
                return (n.id === d.id || connected.has(n.id)) ? 1 : 0.1;
            });

        var showLabels = document.getElementById('toggle-labels').checked;
        labels.transition().duration(FADE_IN).ease(d3.easeCubicOut)
            .style('opacity', function(n) {
                if (!showLabels || currentZoom < 0.4) return 0;
                return (n.id === d.id || connected.has(n.id)) ? 1 : 0.04;
            });

        link.transition().duration(FADE_IN).ease(d3.easeCubicOut)
            .attr('stroke', function(e) {
                var si = typeof e.source === 'object' ? e.source.id : e.source;
                var ti = typeof e.target === 'object' ? e.target.id : e.target;
                return (si === d.id || ti === d.id) ? '#a882ff' : '#444';
            })
            .attr('stroke-opacity', function(e) {
                var si = typeof e.source === 'object' ? e.source.id : e.source;
                var ti = typeof e.target === 'object' ? e.target.id : e.target;
                return (si === d.id || ti === d.id) ? 0.7 : 0.04;
            })
            .attr('marker-end', function(e) {
                var si = typeof e.source === 'object' ? e.source.id : e.source;
                var ti = typeof e.target === 'object' ? e.target.id : e.target;
                return (si === d.id || ti === d.id) ? 'url(#arrow-active)' : 'url(#arrow)';
            });

        document.getElementById('focus-hint').style.display = 'block';
        // Hide tooltip when focus activates (it persists from the preceding hover)
        document.getElementById('tooltip').style.display = 'none';
        hoverActive = false;
        pushHistory(d);
    }

    function clearFocus() {
        focusActive = false;
        focusedNodeId = null;
        node.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
            .attr('fill', function(n) { return nodeColor(n); })
            .style('opacity', 1);
        labels.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
            .style('opacity', !document.getElementById('toggle-labels').checked ? 0 : currentZoom < 0.4 ? 0 : 1);
        link.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
            .attr('stroke', '#444').attr('stroke-opacity', 0.25).attr('marker-end', 'url(#arrow)');
        document.getElementById('focus-hint').style.display = 'none';
    }

    // Exit focus on Escape or background click
    svg.on('click', function() { if (focusActive) clearFocus(); });
    document.addEventListener('keydown', function(e) {
        if (e.key === 'Escape' && focusActive) clearFocus();
        if (e.key === 'Escape' && blastRadiusActive && blastRadiusSourceId) clearBlastRadius();
    });

    // === Fit-to-Screen (VIZN-04) ===
    function fitToScreen() {
        var visibleNodes = nodes.filter(function(d) { return d.x !== undefined; });
        if (visibleNodes.length === 0) return;
        var minX = d3.min(visibleNodes, function(d) { return d.x - (d.radius || 6); });
        var maxX = d3.max(visibleNodes, function(d) { return d.x + (d.radius || 6); });
        var minY = d3.min(visibleNodes, function(d) { return d.y - (d.radius || 6); });
        var maxY = d3.max(visibleNodes, function(d) { return d.y + (d.radius || 6); });
        var padding = 48;
        var svgW = +svg.attr('width');
        var svgH = +svg.attr('height');
        var scale = Math.min(
            (svgW - padding * 2) / (maxX - minX),
            (svgH - padding * 2) / (maxY - minY),
            1
        );
        var tx = svgW / 2 - scale * (minX + maxX) / 2;
        var ty = svgH / 2 - scale * (minY + maxY) / 2;
        svg.transition().duration(500).ease(d3.easeCubicInOut)
            .call(zoomBehavior.transform, d3.zoomIdentity.translate(tx, ty).scale(scale));
    }

    document.getElementById('btn-fit').addEventListener('click', fitToScreen);

    // F key shortcut (when no input focused)
    document.addEventListener('keydown', function(e) {
        if (e.key === 'f' || e.key === 'F') {
            var tag = document.activeElement.tagName;
            if (tag !== 'INPUT' && tag !== 'TEXTAREA' && tag !== 'SELECT') {
                fitToScreen();
            }
        }
    });

    // === Header Search (INTR-01, D-73) ===

    // Collect all searchable items: file nodes + symbols (from API data)
    var allSearchableItems = [];
    nodes.forEach(function(n) {
        allSearchableItems.push({ id: n.id, name: n.filename || n.name, path: n.path || n.file_path, kind: 'file', _ref: n });
    });
    (data.symbols || []).forEach(function(s) {
        allSearchableItems.push({ id: s.id, name: s.name, path: s.file_path, kind: s.kind, _ref: null });
    });

    function searchNodes(query) {
        if (!query) return [];
        var q = query.toLowerCase();
        return allSearchableItems.filter(function(item) {
            return item.name.toLowerCase().includes(q) || item.path.toLowerCase().includes(q);
        });
    }

    var headerSearch = document.getElementById('header-search');
    headerSearch.addEventListener('input', function() {
        var q = this.value.trim();
        if (!q) {
            // Clear highlight
            if (!focusActive) {
                node.style('opacity', 1).attr('fill', function(d) { return nodeColor(d); });
                labels.style('opacity', !document.getElementById('toggle-labels').checked ? 0 : currentZoom < 0.4 ? 0 : 1);
            }
            return;
        }
        var matches = searchNodes(q);
        var matchIds = new Set(matches.map(function(m) { return m.id; }));

        // Also match file nodes if any of their symbols match
        matches.forEach(function(m) {
            if (m.kind !== 'file') {
                // Find the file node for this symbol's file_path
                var fileId = m.path;
                matchIds.add(fileId);
            }
        });

        node.style('opacity', function(d) { return matchIds.has(d.id) ? 1 : 0.12; })
            .attr('fill', function(d) { return matchIds.has(d.id) ? '#7f6df2' : nodeColor(d); });
        labels.style('opacity', function(d) {
            if (!document.getElementById('toggle-labels').checked || currentZoom < 0.4) return 0;
            return matchIds.has(d.id) ? 1 : 0.06;
        });
    });

    headerSearch.addEventListener('keydown', function(e) {
        if (e.key === 'Enter') {
            var q = this.value.trim();
            if (!q) return;
            var matches = searchNodes(q);
            if (matches.length > 0) {
                var target = matches[0];
                // Find the actual node object in the nodes array
                var targetNode = nodes.find(function(n) { return n.id === target.id; });
                if (!targetNode && target.kind !== 'file') {
                    // Symbol not expanded yet — expand its file first
                    var fileNode = nodes.find(function(n) { return n.id === target.path; });
                    if (fileNode && !expandedFiles.has(fileNode.id)) {
                        expandFileNode(fileNode);
                        targetNode = nodes.find(function(n) { return n.id === target.id; });
                    }
                }
                if (targetNode) {
                    flyToNode(targetNode);
                    activateFocus(targetNode);
                }
            }
            this.value = '';
            // Reset search highlight
            node.style('opacity', 1).attr('fill', function(d) { return nodeColor(d); });
        }
        if (e.key === 'Escape') {
            this.value = '';
            this.blur();
            node.style('opacity', 1).attr('fill', function(d) { return nodeColor(d); });
            labels.style('opacity', !document.getElementById('toggle-labels').checked ? 0 : currentZoom < 0.4 ? 0 : 1);
        }
    });

    function flyToNode(d) {
        var svgW = +svg.attr('width');
        var svgH = +svg.attr('height');
        var scale = 1.5;
        var tx = svgW / 2 - scale * d.x;
        var ty = svgH / 2 - scale * d.y;
        svg.transition().duration(600).ease(d3.easeCubicInOut)
            .call(zoomBehavior.transform, d3.zoomIdentity.translate(tx, ty).scale(scale));
    }

    // === Command Palette (D-73, Cmd+K) ===

    var paletteOpen = false;
    var paletteSelectedIndex = 0;
    var paletteResults = [];

    function openPalette() {
        paletteOpen = true;
        document.getElementById('palette-backdrop').style.display = 'block';
        document.getElementById('command-palette').style.display = 'block';
        var input = document.getElementById('palette-input');
        input.value = '';
        input.focus();
        renderPaletteResults('');
    }

    function closePalette() {
        paletteOpen = false;
        document.getElementById('palette-backdrop').style.display = 'none';
        document.getElementById('command-palette').style.display = 'none';
        paletteResults = [];
        paletteSelectedIndex = 0;
    }

    function renderPaletteResults(query) {
        var container = document.getElementById('palette-results');
        container.innerHTML = '';
        paletteSelectedIndex = 0;

        if (!query) {
            // Show recent or top items
            paletteResults = allSearchableItems.slice(0, 8);
        } else {
            paletteResults = searchNodes(query).slice(0, 8);
        }

        if (paletteResults.length === 0) {
            var noResults = document.createElement('div');
            noResults.className = 'palette-no-results';
            noResults.textContent = 'No matching symbols';
            container.appendChild(noResults);
            return;
        }

        paletteResults.forEach(function(item, idx) {
            var row = document.createElement('div');
            row.className = 'palette-item' + (idx === 0 ? ' selected' : '');
            row.setAttribute('data-idx', idx);

            var nameSpan = document.createElement('span');
            nameSpan.textContent = item.name;
            row.appendChild(nameSpan);

            var kindSpan = document.createElement('span');
            kindSpan.className = 'palette-kind';
            kindSpan.textContent = item.kind;
            row.appendChild(kindSpan);

            var pathSpan = document.createElement('span');
            pathSpan.className = 'palette-path';
            pathSpan.textContent = item.path;
            row.appendChild(pathSpan);

            row.addEventListener('click', function() {
                selectPaletteItem(idx);
            });
            row.addEventListener('mouseenter', function() {
                paletteSelectedIndex = idx;
                updatePaletteSelection();
            });

            container.appendChild(row);
        });
    }

    function updatePaletteSelection() {
        var items = document.querySelectorAll('.palette-item');
        items.forEach(function(el, i) {
            el.classList.toggle('selected', i === paletteSelectedIndex);
        });
    }

    function selectPaletteItem(idx) {
        var item = paletteResults[idx];
        if (!item) return;
        closePalette();

        var targetNode = nodes.find(function(n) { return n.id === item.id; });
        if (!targetNode && item.kind !== 'file') {
            var fileNode = nodes.find(function(n) { return n.id === item.path; });
            if (fileNode && !expandedFiles.has(fileNode.id)) {
                expandFileNode(fileNode);
                targetNode = nodes.find(function(n) { return n.id === item.id; });
            }
        }
        if (targetNode) {
            flyToNode(targetNode);
            activateFocus(targetNode);
        }
    }

    document.getElementById('palette-input').addEventListener('input', function() {
        renderPaletteResults(this.value.trim());
    });

    document.getElementById('palette-input').addEventListener('keydown', function(e) {
        if (e.key === 'ArrowDown') {
            e.preventDefault();
            paletteSelectedIndex = Math.min(paletteSelectedIndex + 1, paletteResults.length - 1);
            updatePaletteSelection();
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            paletteSelectedIndex = Math.max(paletteSelectedIndex - 1, 0);
            updatePaletteSelection();
        } else if (e.key === 'Enter') {
            e.preventDefault();
            selectPaletteItem(paletteSelectedIndex);
        } else if (e.key === 'Escape') {
            closePalette();
        }
    });

    document.getElementById('palette-backdrop').addEventListener('click', closePalette);

    // Global Cmd+K / Ctrl+K shortcut
    document.addEventListener('keydown', function(e) {
        if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
            e.preventDefault();
            if (paletteOpen) {
                closePalette();
            } else {
                openPalette();
            }
        }
    });

    // === Dead Code Overlay (INTR-04, D-77) ===

    // Pre-compute dead code sets from API data (symbol-level IDs)
    var deadCodeConfirmed = new Set();
    var deadCodeSuspicious = new Set();
    (data.symbols || []).forEach(function(s) {
        if (s.is_dead_code && s.dead_code_confidence === 'confirmed') {
            deadCodeConfirmed.add(s.id);
        } else if (s.is_dead_code && s.dead_code_confidence === 'suspicious') {
            deadCodeSuspicious.add(s.id);
        }
    });

    // Pre-compute dead code counts per file for file-level overlay
    // Maps file_path -> { confirmed: number, suspicious: number }
    var deadCodeByFile = {};
    (data.symbols || []).forEach(function(s) {
        if (!s.is_dead_code) return;
        if (!deadCodeByFile[s.file_path]) deadCodeByFile[s.file_path] = { confirmed: 0, suspicious: 0 };
        if (s.dead_code_confidence === 'confirmed') {
            deadCodeByFile[s.file_path].confirmed++;
        } else if (s.dead_code_confidence === 'suspicious') {
            deadCodeByFile[s.file_path].suspicious++;
        }
    });

    var deadCodeActive = false;
    var badgeGroup = null;

    function showDeadCodeOverlay() {
        deadCodeActive = true;

        // Create badge group if not exists (after nodes group for Z-order)
        if (!badgeGroup) {
            badgeGroup = g.append('g').attr('class', 'badges');
        }
        badgeGroup.selectAll('*').remove();

        // Apply border styling to nodes:
        // - Symbol nodes: match by symbol ID in deadCodeConfirmed/deadCodeSuspicious sets
        // - File nodes (not expanded): match by file path in deadCodeByFile map
        node.attr('stroke', function(d) {
            if (d._isSymbol) {
                if (deadCodeConfirmed.has(d.id) || deadCodeSuspicious.has(d.id)) return '#f87171';
                return 'none';
            }
            // File node: highlight if it has dead children AND is NOT currently expanded
            if (!expandedFiles.has(d.id) && deadCodeByFile[d.id]) return '#f87171';
            return 'none';
        })
        .attr('stroke-width', function(d) {
            if (d._isSymbol) {
                if (deadCodeConfirmed.has(d.id)) return 3;
                if (deadCodeSuspicious.has(d.id)) return 2;
                return 0;
            }
            if (!expandedFiles.has(d.id) && deadCodeByFile[d.id]) {
                return deadCodeByFile[d.id].confirmed > 0 ? 3 : 2;
            }
            return 0;
        })
        .attr('stroke-dasharray', function(d) {
            if (d._isSymbol && deadCodeSuspicious.has(d.id)) return '3 2';
            if (!d._isSymbol && !expandedFiles.has(d.id) && deadCodeByFile[d.id]) {
                // Dashed if file has ONLY suspicious dead code, solid if any confirmed
                if (deadCodeByFile[d.id].confirmed === 0 && deadCodeByFile[d.id].suspicious > 0) return '3 2';
            }
            return null;
        })
        .attr('stroke-opacity', function(d) {
            if (d._isSymbol && deadCodeSuspicious.has(d.id)) return 0.5;
            if (!d._isSymbol && !expandedFiles.has(d.id) && deadCodeByFile[d.id]) {
                if (deadCodeByFile[d.id].confirmed === 0) return 0.5;
            }
            return 1;
        });

        // Add badges to dead code nodes
        nodes.forEach(function(d) {
            if (d._isSymbol) {
                if (!deadCodeConfirmed.has(d.id) && !deadCodeSuspicious.has(d.id)) return;
                var isConfirmed = deadCodeConfirmed.has(d.id);
                var badge = badgeGroup.append('g')
                    .attr('class', 'dead-badge')
                    .attr('data-node-id', d.id);

                badge.append('circle')
                    .attr('r', 5)
                    .attr('fill', isConfirmed ? '#f87171' : 'rgba(248,113,113,0.5)')
                    .attr('stroke', 'none');

                badge.append('text')
                    .attr('text-anchor', 'middle')
                    .attr('dy', '0.35em')
                    .attr('fill', '#fff')
                    .attr('font-size', '8px')
                    .attr('font-weight', '700')
                    .attr('pointer-events', 'none')
                    .text(isConfirmed ? 'x' : '?');
            } else {
                // File node: show count badge if unexpanded and has dead children
                if (expandedFiles.has(d.id) || !deadCodeByFile[d.id]) return;
                var counts = deadCodeByFile[d.id];
                var total = counts.confirmed + counts.suspicious;
                var badge = badgeGroup.append('g')
                    .attr('class', 'dead-badge')
                    .attr('data-node-id', d.id);

                badge.append('circle')
                    .attr('r', 7)
                    .attr('fill', counts.confirmed > 0 ? '#f87171' : 'rgba(248,113,113,0.5)')
                    .attr('stroke', 'none');

                badge.append('text')
                    .attr('text-anchor', 'middle')
                    .attr('dy', '0.35em')
                    .attr('fill', '#fff')
                    .attr('font-size', '8px')
                    .attr('font-weight', '700')
                    .attr('pointer-events', 'none')
                    .text(total);
            }
        });

        // Update badge positions
        updateBadgePositions();

        // Show dead code stats
        var statsEl = document.getElementById('dead-code-stats');
        var countEl = document.getElementById('dead-code-count');
        countEl.textContent = deadCodeConfirmed.size + ' confirmed, ' + deadCodeSuspicious.size + ' suspicious';
        statsEl.style.display = 'flex';
    }

    function hideDeadCodeOverlay() {
        deadCodeActive = false;
        node.attr('stroke', 'none').attr('stroke-width', 0).attr('stroke-dasharray', null).attr('stroke-opacity', 1);
        if (badgeGroup) badgeGroup.selectAll('*').remove();
        document.getElementById('dead-code-stats').style.display = 'none';
    }

    function updateBadgePositions() {
        if (!badgeGroup) return;
        badgeGroup.selectAll('.dead-badge').each(function() {
            var nodeId = d3.select(this).attr('data-node-id');
            var d = nodes.find(function(n) { return n.id === nodeId; });
            if (d) {
                var r = (d.radius || 6) * nodeSizeScale;
                d3.select(this).attr('transform', 'translate(' + (d.x + r * 0.7) + ',' + (d.y - r * 0.7) + ')');
            }
        });
    }

    // Hook badge position updates into the simulation tick
    var origUpdatePositions = updatePositions;
    updatePositions = function() {
        origUpdatePositions();
        if (deadCodeActive) updateBadgePositions();
    };
    // Re-register tick handler
    simulation.on('tick', updatePositions);

    // Re-apply overlay after expand/collapse to update file-level vs symbol-level highlighting
    var origRebuildSimulation = rebuildSimulation;
    rebuildSimulation = function() {
        origRebuildSimulation();
        if (deadCodeActive) {
            // Re-apply overlay after DOM rejoins with new node selection
            setTimeout(function() { showDeadCodeOverlay(); }, 50);
        }
    };

    document.getElementById('toggle-dead-code').addEventListener('change', function() {
        if (this.checked) {
            showDeadCodeOverlay();
        } else {
            hideDeadCodeOverlay();
        }
    });

    // === Blast Radius Mode (INTR-03, D-78, D-82) ===

    var blastRadiusActive = false;
    var blastRadiusSourceId = null;

    function computeBlastRadius(nodeId) {
        // BFS over edges to find all transitive dependents (who depends on nodeId?)
        // An edge from A -> B means A depends on B (import/call).
        // Blast radius of B = all nodes that directly or transitively depend on B.
        // Reverse: from nodeId, traverse edges where nodeId is the TARGET, collect sources.
        var dependents = new Set();
        var queue = [nodeId];
        var allEdgesForTraversal = edges.concat(symbolEdges);

        while (queue.length > 0) {
            var current = queue.shift();
            allEdgesForTraversal.forEach(function(e) {
                var src = typeof e.source === 'object' ? e.source.id : e.source;
                var tgt = typeof e.target === 'object' ? e.target.id : e.target;
                // If edge points TO current, the source DEPENDS on current
                if (tgt === current && !dependents.has(src)) {
                    dependents.add(src);
                    queue.push(src);
                }
            });
        }
        return dependents;
    }

    function showBlastRadius(sourceNode) {
        blastRadiusSourceId = sourceNode.id;
        var dependents = computeBlastRadius(sourceNode.id);

        node.transition().duration(FADE_IN).ease(d3.easeCubicOut)
            .attr('fill', function(n) {
                if (n.id === sourceNode.id) return '#7f6df2';
                if (dependents.has(n.id)) return '#a882ff';
                return nodeColor(n);
            })
            .style('opacity', function(n) {
                if (n.id === sourceNode.id || dependents.has(n.id)) return 1;
                return 0.15;
            });

        labels.transition().duration(FADE_IN).ease(d3.easeCubicOut)
            .style('opacity', function(n) {
                if (!document.getElementById('toggle-labels').checked || currentZoom < 0.4) return 0;
                return (n.id === sourceNode.id || dependents.has(n.id)) ? 1 : 0.04;
            });

        link.transition().duration(FADE_IN).ease(d3.easeCubicOut)
            .attr('stroke', function(e) {
                var si = typeof e.source === 'object' ? e.source.id : e.source;
                var ti = typeof e.target === 'object' ? e.target.id : e.target;
                return (dependents.has(si) || si === sourceNode.id) && (dependents.has(ti) || ti === sourceNode.id)
                    ? '#a882ff' : '#444';
            })
            .attr('stroke-opacity', function(e) {
                var si = typeof e.source === 'object' ? e.source.id : e.source;
                var ti = typeof e.target === 'object' ? e.target.id : e.target;
                return (dependents.has(si) || si === sourceNode.id) && (dependents.has(ti) || ti === sourceNode.id)
                    ? 0.6 : 0.04;
            });
    }

    function clearBlastRadius() {
        blastRadiusSourceId = null;
        node.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
            .attr('fill', function(n) { return nodeColor(n); })
            .style('opacity', 1);
        labels.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
            .style('opacity', !document.getElementById('toggle-labels').checked ? 0 : currentZoom < 0.4 ? 0 : 1);
        link.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
            .attr('stroke', '#444').attr('stroke-opacity', 0.25);
    }

    document.getElementById('toggle-blast-radius').addEventListener('change', function() {
        blastRadiusActive = this.checked;
        if (blastRadiusActive) {
            document.getElementById('blast-radius-prompt').style.display = 'flex';
        } else {
            document.getElementById('blast-radius-prompt').style.display = 'none';
            clearBlastRadius();
        }
    });

    // === Panel controls ===

    // Filters: reset
    document.getElementById('btn-reset-filters').addEventListener('click', function() {
        document.getElementById('search-files').value = '';
        document.getElementById('toggle-orphans').checked = true;
        node.style('display', null).style('opacity', 1);
        labels.style('display', null).style('opacity', currentZoom < 0.4 ? 0 : 1);
    });

    // Filters: search
    document.getElementById('search-files').addEventListener('input', function(e) {
        var q = e.target.value.toLowerCase();
        node.style('opacity', function(d) { return !q || (d.path || d.file_path || '').toLowerCase().includes(q) ? 1 : 0.1; });
        labels.style('opacity', function(d) { return !q || (d.path || d.file_path || '').toLowerCase().includes(q) ? 1 : 0.05; });
    });

    // Filters: orphans
    document.getElementById('toggle-orphans').addEventListener('change', function() {
        var show = this.checked;
        node.style('display', function(d) {
            if (show) return null;
            return (adjacency.get(d.id) || new Set()).size === 0 ? 'none' : null;
        });
        labels.style('display', function(d) {
            if (show) return null;
            return (adjacency.get(d.id) || new Set()).size === 0 ? 'none' : null;
        });
    });

    // Display: arrows
    document.getElementById('toggle-arrows').addEventListener('change', function() {
        link.attr('marker-end', this.checked ? 'url(#arrow)' : 'none');
    });

    // Display: labels
    document.getElementById('toggle-labels').addEventListener('change', updateLabelVisibility);

    // Display: node size
    document.getElementById('slider-node-size').addEventListener('input', function() {
        nodeSizeScale = parseFloat(this.value);
        node.attr('r', function(d) { return d.radius * nodeSizeScale; });
        simulation.force('collide', d3.forceCollide().radius(function(d) { return d.radius * nodeSizeScale + 8; }));
        simulation.alpha(0.3).restart();
    });

    // Display: label size
    document.getElementById('slider-label-size').addEventListener('input', function() {
        labels.attr('font-size', this.value + 'px');
    });

    // Forces: center
    document.getElementById('slider-center').addEventListener('input', function() {
        var v = parseFloat(this.value);
        simulation.force('center', d3.forceCenter(width / 2, height / 2).strength(v));
        simulation.force('x', d3.forceX(width / 2).strength(v / 2));
        simulation.force('y', d3.forceY(height / 2).strength(v / 2));
        simulation.alpha(0.5).restart();
    });

    // Forces: repel
    document.getElementById('slider-repel').addEventListener('input', function() {
        simulation.force('charge', d3.forceManyBody().strength(-parseFloat(this.value)));
        simulation.alpha(0.5).restart();
    });

    // Forces: link force
    document.getElementById('slider-link-force').addEventListener('input', function() {
        simulation.force('link').strength(parseFloat(this.value));
        simulation.alpha(0.5).restart();
    });

    // Forces: link distance
    document.getElementById('slider-link-dist').addEventListener('input', function() {
        simulation.force('link').distance(parseFloat(this.value));
        simulation.alpha(0.5).restart();
    });

    // Directory halos
    var halosGroup = null;
    document.getElementById('halos-toggle').addEventListener('change', function() {
        if (halosGroup) { halosGroup.remove(); halosGroup = null; }
        if (!this.checked) return;
        var dirMap = new Map();
        nodes.forEach(function(d) {
            var i = d.path.lastIndexOf('/');
            var dir = i >= 0 ? d.path.substring(0, i) : '';
            if (!dirMap.has(dir)) dirMap.set(dir, []);
            dirMap.get(dir).push(d);
        });
        halosGroup = g.insert('g', '.edges').attr('class', 'halos');
        dirMap.forEach(function(gn) {
            if (gn.length < 2) return;
            var cx = gn.reduce(function(s, n) { return s + n.x; }, 0) / gn.length;
            var cy = gn.reduce(function(s, n) { return s + n.y; }, 0) / gn.length;
            var hull = d3.polygonHull(gn.map(function(n) { return [n.x, n.y]; }));
            if (!hull) return;
            var exp = hull.map(function(pt) {
                var dx = pt[0] - cx, dy = pt[1] - cy;
                var len = Math.sqrt(dx * dx + dy * dy);
                return len === 0 ? pt : [pt[0] + dx / len * 30, pt[1] + dy / len * 30];
            });
            halosGroup.append('path')
                .attr('d', 'M' + exp.map(function(p) { return p[0] + ',' + p[1]; }).join('L') + 'Z')
                .attr('stroke', '#7f6df2').attr('stroke-opacity', 0.15)
                .attr('stroke-dasharray', '4 2').attr('fill', '#7f6df2').attr('fill-opacity', 0.04);
        });
    });

    window.addEventListener('resize', function() {
        svg.attr('width', window.innerWidth).attr('height', window.innerHeight - 40);
    });
}
