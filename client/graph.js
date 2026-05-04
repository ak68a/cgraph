document.addEventListener('DOMContentLoaded', function() {
    loadAndRender();
    initPanel();
});

function initPanel() {
    var toggle = document.getElementById('panel-toggle');
    var panel = document.getElementById('panel');
    toggle.addEventListener('click', function() {
        panel.classList.toggle('collapsed');
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

    var edgeColors = { import: '#a882ff', call: '#2dd4bf', type_ref: '#fbbf24', re_export: '#a882ff', parent_child: '#444' };
    function edgeColor(e) { return edgeColors[e.edge_type] || '#444'; }

    function addArrowMarker(id, color) {
        defs.append('marker').attr('id', id)
            .attr('viewBox', '-0 -5 10 10').attr('refX', 0).attr('refY', 0)
            .attr('orient', 'auto').attr('markerWidth', 6).attr('markerHeight', 4)
          .append('path').attr('d', 'M 0,-5 L 10,0 L 0,5').attr('fill', color);
    }
    addArrowMarker('arrow', '#444');
    addArrowMarker('arrow-import', edgeColors.import);
    addArrowMarker('arrow-call', edgeColors.call);
    addArrowMarker('arrow-type_ref', edgeColors.type_ref);
    addArrowMarker('arrow-active', '#a882ff');

    function edgeMarker(e) {
        if (e._isParentEdge) return 'none';
        var t = e.edge_type || 'import';
        return 'url(#arrow-' + t + ')';
    }

    var g = svg.append('g');
    var currentZoom = 1;

    // Focus state (declared before zoomBehavior since zoom callback references focusActive)
    var focusActive = false;
    var focusedNodeId = null;

    // === State Indicator (Breadcrumb Navigator) ===

    function updateStateIndicator() {
        var el = document.getElementById('state-indicator');
        el.innerHTML = '';

        if (typeof blastRadiusActive !== 'undefined' && blastRadiusActive && blastRadiusSourceId) {
            var bNode = nodes.find(function(n) { return n.id === blastRadiusSourceId; });
            var bName = bNode ? (bNode.filename || bNode.name || bNode.id.split('/').pop()) : '';
            el.innerHTML =
                '<span class="bc-seg" style="color:#e8a838;">' +
                '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#e8a838" stroke-width="2"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="6"/><circle cx="12" cy="12" r="2"/></svg>' +
                ' Blast radius</span>' +
                '<span class="bc-sep">›</span>' +
                '<span class="bc-seg" style="color:#ddd;">' + bName + '</span>' +
                '<button class="bc-close" title="Exit">×</button>';
            el.querySelector('.bc-close').addEventListener('click', function() {
                clearBlastRadius();
                updateStateIndicator();
            });
            el.style.display = 'flex';
        } else if (focusActive && focusedNodeId) {
            var fNode = nodes.find(function(n) { return n.id === focusedNodeId; });
            var fName = fNode ? (fNode.filename || fNode.name || fNode.id.split('/').pop()) : '';
            var isExpanded = fNode && !fNode._isSymbol && expandedFiles.has(fNode.id);

            var curLayer = isExpanded ? 'symbols' : 'file';

            function bcSeg(label, layerId) {
                var s = document.createElement('span');
                s.className = 'bc-seg' + (curLayer === layerId ? ' active' : '');
                s.textContent = label;
                return s;
            }
            function bcSep() {
                var s = document.createElement('span');
                s.className = 'bc-sep';
                s.textContent = '›';
                return s;
            }

            var filesBtn = bcSeg('Files', 'files');
            filesBtn.addEventListener('click', function() { clearFocus(); });
            el.appendChild(filesBtn);
            el.appendChild(bcSep());

            var fileBtn = bcSeg('Focus', 'file');
            fileBtn.addEventListener('click', function() {
                if (isExpanded) { collapseFileNode(focusedNodeId); updateStateIndicator(); }
            });
            el.appendChild(fileBtn);
            el.appendChild(bcSep());

            var symBtn = bcSeg('Symbols', 'symbols');
            symBtn.addEventListener('click', function() {
                if (!isExpanded) {
                    var fn = nodes.find(function(n) { return n.id === focusedNodeId; });
                    if (fn) expandFileNode(fn);
                    updateStateIndicator();
                }
            });
            el.appendChild(symBtn);

            var closeBtn = document.createElement('button');
            closeBtn.className = 'bc-close';
            closeBtn.title = 'Exit (Esc)';
            closeBtn.textContent = '×';
            closeBtn.addEventListener('click', function() { clearFocus(); });
            el.appendChild(closeBtn);

            el.style.display = 'flex';
        } else {
            var label = viewLevel === 'symbols' ? 'Symbols' : 'Files';
            var seg = document.createElement('span');
            seg.className = 'bc-seg active';
            seg.textContent = label;
            el.appendChild(seg);
            el.style.display = 'flex';
        }
        updateContextualControls();
    }

    // === Contextual Controls ===

    function updateContextualControls() {
        var hasExpanded = expandedFiles.size > 0;
        var hasFocus = focusActive && focusedNodeId;
        var isSymbolView = viewLevel === 'symbols';
        var showSymbolFilters = hasExpanded || isSymbolView;
        var showTypedEdgeFilters = showSymbolFilters || fileLens === 'all-edges';

        // Dark room + show neighbors: only useful when focused
        var drRow = document.getElementById('toggle-dark-room').closest('.ctrl-row');
        if (drRow) drRow.classList.toggle('disabled', !hasFocus);
        var nbRow = document.getElementById('toggle-neighbors').closest('.ctrl-row');
        if (nbRow) nbRow.classList.toggle('disabled', !hasFocus);

        // Symbol type filters: only useful when symbols are visible (expanded or symbol view)
        ['filter-fn', 'filter-class', 'filter-type', 'filter-hook', 'filter-enum'].forEach(function(id) {
            var row = document.getElementById(id).closest('.ctrl-row');
            if (row) row.classList.toggle('disabled', !showSymbolFilters);
        });
        var symHeader = document.getElementById('toggle-all-symbols').closest('div');
        if (symHeader) symHeader.classList.toggle('disabled', !showSymbolFilters);

        // Call and type_ref edge filters: enabled with symbols OR all-edges lens
        ['filter-edge-call', 'filter-edge-typeref'].forEach(function(id) {
            var row = document.getElementById(id).closest('.ctrl-row');
            if (row) row.classList.toggle('disabled', !showTypedEdgeFilters);
        });

        // Lens toggle: only visible at file level
        var lensEl = document.getElementById('lens-toggle');
        var stateEl = document.getElementById('state-indicator');
        if (lensEl) {
            var showLens = viewLevel === 'files' && expandedFiles.size === 0;
            lensEl.style.display = showLens ? 'flex' : 'none';
            if (stateEl) stateEl.style.left = showLens ? '210px' : '56px';
        }
    }

    // === Navigation History (INTR-08, D-75) ===

    var historyStack = [];
    var historyIndex = -1;
    var MAX_HISTORY = 50;
    var navigating = false; // guard to prevent push during back/forward

    function pushHistory(targetNode) {
        if (navigating) return;
        // Skip if already at the same node
        if (historyIndex >= 0 && historyStack[historyIndex].id === targetNode.id) return;
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
    var allFileEdges = allEdgeData.filter(function(e) { return e.source.indexOf('::') === -1 && e.target.indexOf('::') === -1; });
    var fileEdgesImportOnly = [];
    var seenFilePairs = {};
    allFileEdges.forEach(function(e) {
        var key = e.source + '|' + e.target;
        if (!seenFilePairs[key]) {
            seenFilePairs[key] = true;
            fileEdgesImportOnly.push(Object.assign({}, e, { edge_type: 'import' }));
        }
    });
    var fileEdgesTyped = allFileEdges.map(function(d) { return Object.assign({}, d); });
    var fileLens = 'imports';
    var edges = fileEdgesImportOnly.map(function(d) { return Object.assign({}, d); });
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

    // Preserve original file-level data for level switching
    var fileNodes = nodes.map(function(d) { return Object.assign({}, d); });
    var fileEdges = edges.map(function(d) { return Object.assign({}, d); });
    var viewLevel = 'files'; // 'files' or 'symbols'

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
    function focusColor(d) {
        if (d._isSymbol) return NODE_COLORS[d.kind] || '#7f6df2';
        return '#7f6df2';
    }

    var expandMode = 'orbital';
    var expandedFiles = new Set(); // Set of file node IDs currently expanded

    var simulation = d3.forceSimulation(nodes)
        .force('link', d3.forceLink(edges).id(function(d) { return d.id; }).distance(50).strength(0.9))
        .force('charge', d3.forceManyBody().strength(function(d) { return d._isSymbol ? -20 : -60; }))
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

    function adjustedEndpoint(source, target, targetRadius, arrowPad) {
        var dx = target.x - source.x;
        var dy = target.y - source.y;
        var dist = Math.sqrt(dx * dx + dy * dy);
        if (dist === 0) return { x: target.x, y: target.y };
        var offset = targetRadius + (arrowPad !== undefined ? arrowPad : 4);
        return {
            x: target.x - (dx / dist) * offset,
            y: target.y - (dy / dist) * offset
        };
    }

    var linkGroup = g.append('g').attr('class', 'edges');
    var link = linkGroup.selectAll('line').data(edges).join('line')
        .attr('stroke', edgeColor).style('opacity', 0.25).attr('marker-end', edgeMarker);

    var nodeGroup = g.append('g').attr('class', 'nodes');
    var node = nodeGroup.selectAll('circle')
        .data(nodes, function(d) { return d.id; })
        .join('circle')
        .attr('r', function(d) { return d.radius; })
        .attr('fill', function(d) { return nodeColor(d); })
        .attr('stroke', 'none').style('cursor', 'grab');

    var labelGroup = g.append('g').attr('class', 'labels');
    function applyLabelStyle(sel) {
        sel.attr('text-anchor', 'middle').attr('pointer-events', 'none')
            .each(function(d) {
                var el = d3.select(this);
                el.selectAll('*').remove();
                if (d._isSymbol) {
                    el.attr('fill', '#ccc').attr('font-size', '9px').attr('font-weight', '400');
                    el.text(d.name);
                } else {
                    el.attr('fill', '#888').attr('font-size', '11px').attr('font-weight', '400');
                    el.text(d.filename);
                }
            });
        return sel;
    }
    var labels = applyLabelStyle(labelGroup.selectAll('text').data(nodes).join('text'));

    var nodeSizeScale = 1;

    function updatePositions() {
        node.attr('cx', function(d) { return d.x; }).attr('cy', function(d) { return d.y; });
        labels.attr('x', function(d) { return d.x; })
              .attr('y', function(d) { return d.y + d.radius * nodeSizeScale + 6 + 11; });
        link.each(function(d) {
            var src = typeof d.source === 'object' ? d.source : null;
            var tgt = typeof d.target === 'object' ? d.target : null;
            if (!src || !tgt) return;
            var pad = d._isParentEdge ? 0 : 4;
            var ep = adjustedEndpoint(src, tgt, tgt.radius * nodeSizeScale, pad);
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

    function labelVisible(d) {
        if (d._isSymbol) return document.getElementById('toggle-symbol-labels').checked;
        return document.getElementById('toggle-labels').checked;
    }

    function defaultLabelOpacity(d) {
        if (!labelVisible(d)) return 0;
        if (currentZoom < 0.4) return 0;
        return Math.min(1, (currentZoom - 0.3) * 3);
    }

    function updateLabelVisibility() {
        labels.style('opacity', defaultLabelOpacity);
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
                radius: 8,
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
        updatePillCounts();
        updateStateIndicator();
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
        updatePillCounts();
        updateStateIndicator();
    }

    function positionSymbols(fileNode, symbolNodes) {
        var mode = expandMode;
        var count = symbolNodes.length;

        if (mode === 'orbital') {
            var orbitalRadius = Math.max(60, count * 12);
            symbolNodes.forEach(function(sn, i) {
                var angle = (2 * Math.PI * i) / count - Math.PI / 2;
                sn.x = fileNode.x + orbitalRadius * Math.cos(angle);
                sn.y = fileNode.y + orbitalRadius * Math.sin(angle);
                sn.fx = sn.x;
                sn.fy = sn.y;
            });
        } else if (mode === 'stacked') {
            var spacing = 24;
            var startY = fileNode.y + fileNode.radius + 16;
            symbolNodes.forEach(function(sn, i) {
                sn.x = fileNode.x;
                sn.y = startY + i * spacing;
                sn.fx = fileNode.x;
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
            return d._isParentEdge ? 30 : 50;
        }).strength(function(d) {
            return d._isParentEdge ? 2 : 0.9;
        }));

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
                        .transition('enter-r').duration(300).attr('r', function(d) { return d.radius * nodeSizeScale; });
                },
                function(update) { return update; },
                function(exit) {
                    return exit.transition('enter-r').duration(200).attr('r', 0).remove();
                }
            );

        // Rewire hover and click on the new selection
        wireNodeEvents(node);

        // Rejoin labels with stable keys
        labels = labelGroup.selectAll('text')
            .data(nodes, function(d) { return d.id; })
            .join(
                function(enter) {
                    var sel = enter.append('text');
                    applyLabelStyle(sel);
                    return sel.style('opacity', 0).transition().duration(300).style('opacity', 1);
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
                        .attr('stroke', function(d) { return d._isParentEdge ? '#444' : edgeColor(d); })
                        .style('opacity', function(d) { return d._isParentEdge ? 0.15 : 0.25; })
                        .attr('stroke-dasharray', function(d) { return d._isParentEdge ? '2 2' : null; })
                        .attr('marker-end', function(d) { return d._isParentEdge ? 'none' : edgeMarker(d); });
                },
                function(update) { return update; },
                function(exit) { return exit.remove(); }
            );

        simulation.alpha(0.3).restart();
    }

    // === Level Toggle (Files / Symbols) ===

    function getActiveFileEdges() {
        return fileLens === 'all-edges' ? fileEdgesTyped : fileEdgesImportOnly;
    }

    function switchFileLens(lens) {
        if (fileLens === lens) return;
        fileLens = lens;

        document.getElementById('lens-imports').classList.toggle('active', lens === 'imports');
        document.getElementById('lens-all-edges').classList.toggle('active', lens === 'all-edges');

        if (viewLevel === 'files' && !focusActive) {
            edges.length = 0;
            getActiveFileEdges().forEach(function(d) { edges.push(Object.assign({}, d)); });

            adjacency.clear();
            nodes.forEach(function(n) { adjacency.set(n.id, new Set()); });
            edges.forEach(function(e) {
                var si = typeof e.source === 'object' ? e.source.id : e.source;
                var ti = typeof e.target === 'object' ? e.target.id : e.target;
                if (adjacency.has(si)) adjacency.get(si).add(ti);
                if (adjacency.has(ti)) adjacency.get(ti).add(si);
            });

            rebuildSimulation();
            applyFilters();
        }
        updateContextualControls();
        rebuildPills();
    }

    function switchToFileLevel() {
        if (viewLevel === 'files') return;
        viewLevel = 'files';

        // Clear focus state
        focusActive = false;
        focusedNodeId = null;
        expandedFiles.clear();

        // Restore file-level data
        nodes.length = 0;
        fileNodes.forEach(function(d) { nodes.push(Object.assign({}, d)); });
        edges.length = 0;
        getActiveFileEdges().forEach(function(d) { edges.push(Object.assign({}, d)); });

        // Rebuild adjacency
        adjacency.clear();
        nodes.forEach(function(n) { adjacency.set(n.id, new Set()); });
        edges.forEach(function(e) {
            var si = typeof e.source === 'object' ? e.source.id : e.source;
            var ti = typeof e.target === 'object' ? e.target.id : e.target;
            if (adjacency.has(si)) adjacency.get(si).add(ti);
            if (adjacency.has(ti)) adjacency.get(ti).add(si);
        });

        // Pre-compute layout before rendering
        simulation.stop();
        simulation.nodes(nodes);
        simulation.force('link', d3.forceLink(edges).id(function(d) { return d.id; }).distance(50).strength(0.9));
        simulation.force('charge').strength(function(d) { return -60; });
        simulation.alpha(1);
        simulation.tick(300);
        simulation.stop();

        rebuildSimulation();
        applyFilters();
        rebuildPills();
        updateStateIndicator();

        document.getElementById('level-files').classList.add('active');
        document.getElementById('level-symbols').classList.remove('active');
    }

    function switchToSymbolLevel() {
        if (viewLevel === 'symbols') return;
        viewLevel = 'symbols';

        // Clear focus and expand state
        focusActive = false;
        focusedNodeId = null;
        expandedFiles.clear();

        // Build symbol nodes from all symbols
        var allSymbols = data.symbols || [];
        nodes.length = 0;
        allSymbols.forEach(function(s) {
            nodes.push({
                id: s.id,
                name: s.name,
                kind: s.kind,
                file_path: s.file_path,
                radius: 8,
                _isSymbol: true,
                is_dead_code: s.is_dead_code,
                dead_code_confidence: s.dead_code_confidence
            });
        });

        // Build symbol-level edges (only between symbols that exist as nodes)
        var nodeIds = new Set(nodes.map(function(n) { return n.id; }));
        edges.length = 0;
        symbolEdges.forEach(function(se) {
            var src = typeof se.source === 'object' ? se.source.id : se.source;
            var tgt = typeof se.target === 'object' ? se.target.id : se.target;
            if (nodeIds.has(src) && nodeIds.has(tgt) && src !== tgt) {
                edges.push(Object.assign({}, se));
            }
        });

        // Rebuild adjacency
        adjacency.clear();
        nodes.forEach(function(n) { adjacency.set(n.id, new Set()); });
        edges.forEach(function(e) {
            var si = typeof e.source === 'object' ? e.source.id : e.source;
            var ti = typeof e.target === 'object' ? e.target.id : e.target;
            if (adjacency.has(si)) adjacency.get(si).add(ti);
            if (adjacency.has(ti)) adjacency.get(ti).add(si);
        });

        // Pre-compute layout
        simulation.stop();
        simulation.nodes(nodes);
        simulation.force('link', d3.forceLink(edges).id(function(d) { return d.id; }).distance(40).strength(0.8));
        simulation.force('charge').strength(function(d) { return -30; });
        simulation.alpha(1);
        simulation.tick(300);
        simulation.stop();

        rebuildSimulation();
        applyFilters();
        rebuildPills();
        updateStateIndicator();

        document.getElementById('level-files').classList.remove('active');
        document.getElementById('level-symbols').classList.add('active');
    }

    document.getElementById('level-files').addEventListener('click', switchToFileLevel);
    document.getElementById('level-symbols').addEventListener('click', switchToSymbolLevel);

    document.getElementById('lens-imports').addEventListener('click', function() { switchFileLens('imports'); });
    document.getElementById('lens-all-edges').addEventListener('click', function() { switchFileLens('all-edges'); });

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
            if (focusActive && d.id === focusedNodeId) return;
            var focusConnectedSet = focusActive ? (adjacency.get(focusedNodeId) || new Set()) : null;
            var isNeighborOrChild = focusActive && (d._parentId === focusedNodeId || (focusConnectedSet && focusConnectedSet.has(d.id)));
            if (focusActive && !isNeighborOrChild) return;
            if (focusActive) {
                // Hover a neighbor/child during focus — only highlight edges to/from focused node and its children
                hoverActive = true;
                var focusFamily = new Set([focusedNodeId]);
                nodes.forEach(function(n) { if (n._parentId === focusedNodeId) focusFamily.add(n.id); });

                var relevantNodes = new Set([d.id, focusedNodeId]);
                edges.forEach(function(e) {
                    var si = typeof e.source === 'object' ? e.source.id : e.source;
                    var ti = typeof e.target === 'object' ? e.target.id : e.target;
                    if ((si === d.id && focusFamily.has(ti)) || (ti === d.id && focusFamily.has(si))) {
                        relevantNodes.add(si);
                        relevantNodes.add(ti);
                    }
                });

                node.transition('highlight').duration(FADE_IN).ease(d3.easeCubicOut)
                    .style('opacity', function(n) {
                        if (relevantNodes.has(n.id)) return 1;
                        if (focusConnectedSet.has(n.id) || n._parentId === focusedNodeId) return 0.3;
                        return 0.08;
                    });
                labels.transition('highlight').duration(FADE_IN).ease(d3.easeCubicOut)
                    .style('opacity', function(n) {
                        if (!labelVisible(n) || currentZoom < 0.4) return 0;
                        if (relevantNodes.has(n.id)) return 1;
                        if (focusConnectedSet.has(n.id) || n._parentId === focusedNodeId) return 0.2;
                        return 0.04;
                    });
                var useTypedColors = fileLens === 'all-edges' || viewLevel === 'symbols' || expandedFiles.size > 0;
                link.transition('highlight').duration(FADE_IN).ease(d3.easeCubicOut)
                    .attr('stroke', function(e) {
                        if (e._isParentEdge) return '#666';
                        var si = typeof e.source === 'object' ? e.source.id : e.source;
                        var ti = typeof e.target === 'object' ? e.target.id : e.target;
                        var isRelevantEdge = (si === d.id && focusFamily.has(ti)) || (ti === d.id && focusFamily.has(si));
                        if (isRelevantEdge) return useTypedColors ? edgeColor(e) : '#a882ff';
                        return edgeColor(e);
                    })
                    .attr('marker-end', function(e) {
                        if (e._isParentEdge) return 'none';
                        var si = typeof e.source === 'object' ? e.source.id : e.source;
                        var ti = typeof e.target === 'object' ? e.target.id : e.target;
                        var isRelevantEdge = (si === d.id && focusFamily.has(ti)) || (ti === d.id && focusFamily.has(si));
                        if (isRelevantEdge) return useTypedColors ? edgeMarker(e) : 'url(#arrow-active)';
                        return 'none';
                    })
                    .style('opacity', function(e) {
                        var si = typeof e.source === 'object' ? e.source.id : e.source;
                        var ti = typeof e.target === 'object' ? e.target.id : e.target;
                        var isRelevantEdge = (si === d.id && focusFamily.has(ti)) || (ti === d.id && focusFamily.has(si));
                        if (isRelevantEdge) return 0.7;
                        if (e._isParentEdge && focusFamily.has(si) && focusFamily.has(ti)) return 0.15;
                        return 0.04;
                    });

                var tooltip = document.getElementById('tooltip');
                if (d._isSymbol) {
                    tooltip.querySelector('.tooltip-path').textContent = d.file_path;
                    tooltip.querySelector('.tooltip-exports').textContent = d.kind;
                    tooltip.querySelector('.tooltip-edges').textContent = d.name;
                } else {
                    var counts = d.export_counts || {};
                    var kinds = [['functions', counts.functions || 0], ['classes', counts.classes || 0],
                        ['types', counts.types || 0], ['interfaces', counts.interfaces || 0],
                        ['hooks', counts.hooks || 0], ['enums', counts.enums || 0]];
                    var parts = kinds.filter(function(k) { return k[1] > 0; }).map(function(k) { return k[1] + ' ' + k[0]; });
                    tooltip.querySelector('.tooltip-path').textContent = d.path || d.file_path;
                    tooltip.querySelector('.tooltip-exports').textContent = parts.length > 0 ? parts.join(', ') : 'no exports';
                    tooltip.querySelector('.tooltip-edges').textContent = (d.incoming || 0) + ' incoming + ' + (d.outgoing || 0) + ' outgoing';
                }
                tooltip.style.display = 'block';
                tooltip.style.left = (event.pageX + 12) + 'px';
                tooltip.style.top = event.pageY + 'px';
                return;
            }
            hoverActive = true;
            var connected = adjacency.get(d.id) || new Set();
            node.transition('highlight').duration(FADE_IN).ease(d3.easeCubicOut)
                .attr('fill', function(n) {
                    if (n.id === d.id) return '#7f6df2';
                    if (connected.has(n.id)) return '#a882ff';
                    return nodeColor(n);
                })
                .style('opacity', function(n) {
                    return (n.id === d.id || connected.has(n.id)) ? 1 : 0.12;
                });
            labels.transition('highlight').duration(FADE_IN).ease(d3.easeCubicOut)
                .style('opacity', function(n) {
                    if (!labelVisible(n) || currentZoom < 0.4) return 0;
                    return (n.id === d.id || connected.has(n.id)) ? 1 : 0.06;
                });
            var useTypedColors = fileLens === 'all-edges' || viewLevel === 'symbols' || expandedFiles.size > 0;
            link.transition('highlight').duration(FADE_IN).ease(d3.easeCubicOut)
                .attr('stroke', function(e) {
                    if (e._isParentEdge) return '#666';
                    var si = typeof e.source === 'object' ? e.source.id : e.source;
                    var ti = typeof e.target === 'object' ? e.target.id : e.target;
                    if (si === d.id || ti === d.id) return useTypedColors ? edgeColor(e) : '#a882ff';
                    return edgeColor(e);
                })
                .attr('marker-end', function(e) {
                    if (e._isParentEdge) return 'none';
                    var si = typeof e.source === 'object' ? e.source.id : e.source;
                    var ti = typeof e.target === 'object' ? e.target.id : e.target;
                    if (si === d.id || ti === d.id) return useTypedColors ? edgeMarker(e) : 'url(#arrow-active)';
                    return 'none';
                })
                .style('opacity', function(e) {
                    var si = typeof e.source === 'object' ? e.source.id : e.source;
                    var ti = typeof e.target === 'object' ? e.target.id : e.target;
                    return (si === d.id || ti === d.id) ? 0.7 : 0.04;
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
            if (focusActive) {
                hoverActive = false;
                document.getElementById('tooltip').style.display = 'none';
                applyFilters();
                return;
            }
            hoverActive = false;
            node.transition('highlight').duration(FADE_OUT).ease(d3.easeCubicIn)
                .attr('fill', function(n) { return nodeColor(n); })
                .style('opacity', 1);
            labels.transition('highlight').duration(FADE_OUT).ease(d3.easeCubicIn)
                .style('opacity', defaultLabelOpacity);
            link.transition('highlight').duration(FADE_OUT).ease(d3.easeCubicIn)
                .attr('stroke', edgeColor).attr('marker-end', edgeMarker).style('opacity', 0.25);
            document.getElementById('tooltip').style.display = 'none';
        });

        sel.on('click', function(event, d) {
            event.stopPropagation();

            // If file node: toggle expand/collapse
            // Only collapse if this file is already the focused node (deliberate toggle)
            if (!d._isSymbol && viewLevel === 'files') {
                if (expandedFiles.has(d.id)) {
                    if (focusedNodeId === d.id) {
                        collapseFileNode(d.id);
                    }
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

        node.attr('fill', function(n) {
            if (n.id === d.id) return '#7f6df2';
            return nodeColor(n);
        });

        if (centeredMode) flyToNode(d, true);

        applyFilters();
        updateStateIndicator();
        updateFitButton();
        showDetailPanel(d);
        document.getElementById('tooltip').style.display = 'none';
        hoverActive = false;
        pushHistory(d);
    }

    function clearFocus() {
        Array.from(expandedFiles).forEach(function(fid) { collapseFileNode(fid); });
        focusActive = false;
        focusedNodeId = null;
        centeredMode = false;
        node.attr('fill', function(n) { return nodeColor(n); });
        link.attr('stroke-opacity', null);
        applyFilters();
        updateStateIndicator();
        updateFitButton();
        hideDetailPanel();
    }

    // === Detail Panel ===
    var detailPanel = document.getElementById('detail-panel');
    var EDGE_TYPE_LABELS = { import: 'import', call: 'call', type_ref: 'type ref', re_export: 're-export' };

    function clearPanelHovers() {
        node.transition('panel-hover').duration(0)
            .attr('r', function(n) { return (n.radius || 8) * nodeSizeScale; })
            .attr('stroke', 'none').attr('stroke-width', 0);
    }

    function showDetailPanel(d) {
        clearPanelHovers();
        var panel = detailPanel;
        var kindEl = document.getElementById('detail-kind');
        var nameEl = document.getElementById('detail-name');
        var pathEl = document.getElementById('detail-path');
        var bodyEl = document.getElementById('detail-body');

        var kind = d._isSymbol ? (d.kind || 'symbol') : 'file';
        var name = d._isSymbol ? d.name : (d.filename || d.id.split('/').pop());
        var path = d._isSymbol ? d.file_path : (d.path || d.id);

        kindEl.textContent = kind;
        if (kind === 'file') {
            kindEl.style.background = '#666';
            kindEl.style.color = '#ddd';
        } else {
            kindEl.style.background = (NODE_COLORS[kind] || '#555') + '66';
            kindEl.style.color = NODE_COLORS[kind] || '#999';
        }
        nameEl.textContent = name;
        pathEl.textContent = path;

        bodyEl.innerHTML = '';

        // Dead code section with actual symbol names
        var filePath = d._isSymbol ? d.file_path : d.id;
        if (d._isSymbol) {
            if (deadCodeConfirmed.has(d.id)) {
                addDeadCodeBadge(bodyEl, 'Confirmed dead code', '#f87171',
                    'No file in the scanned codebase imports or calls this symbol. It may still be used via dynamic imports, string-based routing, or config-driven registration.');
            } else if (deadCodeSuspicious.has(d.id)) {
                addDeadCodeBadge(bodyEl, 'Suspicious dead code', '#fbbf24',
                    'This symbol has very few incoming references and may be unused.');
            }
        } else {
            var deadSymbols = (symbolsByFile[filePath] || []).filter(function(s) { return s.is_dead_code; });
            if (deadSymbols.length > 0) {
                var dcSection = document.createElement('div');
                dcSection.style.cssText = 'border-bottom:1px solid #333;padding:6px 0;';

                var dcHeader = document.createElement('div');
                dcHeader.className = 'detail-section-title';
                dcHeader.style.color = '#f87171';
                dcHeader.textContent = 'Dead code (' + deadSymbols.length + ')';
                dcSection.appendChild(dcHeader);

                var dcExpl = document.createElement('div');
                dcExpl.style.cssText = 'padding:0 12px 4px;font-size:10px;color:#666;line-height:1.4;';
                dcExpl.textContent = 'Symbols with no incoming imports or calls in the scanned codebase.';
                dcSection.appendChild(dcExpl);

                deadSymbols.forEach(function(s) {
                    var row = document.createElement('div');
                    row.className = 'detail-item';
                    var dot = document.createElement('span');
                    dot.className = 'detail-dot';
                    dot.style.background = s.dead_code_confidence === 'confirmed' ? '#f87171' : '#fbbf24';
                    row.appendChild(dot);
                    var nameSpan = document.createElement('span');
                    nameSpan.textContent = s.name;
                    row.appendChild(nameSpan);
                    var confSpan = document.createElement('span');
                    confSpan.className = 'detail-edge-type';
                    confSpan.textContent = s.dead_code_confidence;
                    confSpan.style.color = s.dead_code_confidence === 'confirmed' ? '#f87171' : '#fbbf24';
                    row.appendChild(confSpan);
                    dcSection.appendChild(row);
                });

                bodyEl.appendChild(dcSection);
            }
        }

        // Collect edges
        var outgoing = [];
        var incoming = [];
        var seen = new Set();

        edges.forEach(function(e) {
            if (e._isParentEdge) return;
            var si = typeof e.source === 'object' ? e.source.id : e.source;
            var ti = typeof e.target === 'object' ? e.target.id : e.target;
            if (si === d.id) { outgoing.push({ nodeId: ti, edgeType: e.edge_type }); seen.add('o:' + ti + ':' + e.edge_type); }
            if (ti === d.id) { incoming.push({ nodeId: si, edgeType: e.edge_type }); seen.add('i:' + si + ':' + e.edge_type); }
        });

        if (!d._isSymbol) {
            symbolEdges.forEach(function(se) {
                var src = typeof se.source === 'object' ? se.source.id : se.source;
                var tgt = typeof se.target === 'object' ? se.target.id : se.target;
                var srcFile = src.indexOf('::') !== -1 ? src.substring(0, src.indexOf('::')) : src;
                var tgtFile = tgt.indexOf('::') !== -1 ? tgt.substring(0, tgt.indexOf('::')) : tgt;
                if (srcFile === d.id && tgtFile !== d.id) {
                    var displayId = nodes.some(function(n) { return n.id === tgt; }) ? tgt : tgtFile;
                    var key = 'o:' + displayId + ':' + se.edge_type;
                    if (!seen.has(key)) { outgoing.push({ nodeId: displayId, edgeType: se.edge_type }); seen.add(key); }
                }
                if (tgtFile === d.id && srcFile !== d.id) {
                    var displayId2 = nodes.some(function(n) { return n.id === src; }) ? src : srcFile;
                    var key2 = 'i:' + displayId2 + ':' + se.edge_type;
                    if (!seen.has(key2)) { incoming.push({ nodeId: displayId2, edgeType: se.edge_type }); seen.add(key2); }
                }
            });
        }

        function findNodeName(id) {
            var n = nodes.find(function(nd) { return nd.id === id; });
            if (n) return n._isSymbol ? n.name : (n.filename || n.id.split('/').pop());
            var parts = id.split('::');
            if (parts.length > 1) return parts[parts.length - 1];
            return id.split('/').pop();
        }

        function findNodeKind(id) {
            var n = nodes.find(function(nd) { return nd.id === id; });
            if (n && n._isSymbol) return n.kind || 'symbol';
            if (n) return 'file';
            if (id.indexOf('::') !== -1) return 'symbol';
            return 'file';
        }

        var outItems = outgoing.map(function(o) {
            return { nodeId: o.nodeId, name: findNodeName(o.nodeId), kind: findNodeKind(o.nodeId), edgeType: o.edgeType };
        });
        var inItems = incoming.map(function(i) {
            return { nodeId: i.nodeId, name: findNodeName(i.nodeId), kind: findNodeKind(i.nodeId), edgeType: i.edgeType };
        });

        // Summary line
        var summaryParts = [];
        if (outItems.length > 0) summaryParts.push('depends on ' + outItems.length);
        if (inItems.length > 0) summaryParts.push(inItems.length + ' dependents');
        if (summaryParts.length > 0) {
            var summary = document.createElement('div');
            summary.style.cssText = 'padding:6px 12px;font-size:11px;color:#888;border-bottom:1px solid #333;';
            summary.textContent = summaryParts.join(' · ');
            bodyEl.appendChild(summary);
        }

        addGroupedSection('Depends on', '→', outItems, bodyEl);
        addGroupedSection('Depended on by', '←', inItems, bodyEl);

        if (outItems.length === 0 && inItems.length === 0) {
            var empty = document.createElement('div');
            empty.className = 'detail-empty';
            if (!d._isSymbol) {
                var crossFileCount = 0;
                symbolEdges.forEach(function(se) {
                    var src = typeof se.source === 'object' ? se.source.id : se.source;
                    var tgt = typeof se.target === 'object' ? se.target.id : se.target;
                    var srcFile = src.indexOf('::') !== -1 ? src.substring(0, src.indexOf('::')) : src;
                    var tgtFile = tgt.indexOf('::') !== -1 ? tgt.substring(0, tgt.indexOf('::')) : tgt;
                    if ((srcFile === d.id || tgtFile === d.id) && srcFile !== tgtFile) crossFileCount++;
                });
                if (crossFileCount > 0) {
                    empty.textContent = 'Isolated file — no direct file imports, but symbols have ' + crossFileCount + ' cross-file relationship' + (crossFileCount > 1 ? 's' : '') + '. Expand to see.';
                } else {
                    empty.textContent = 'Isolated file — no connections found in scanned codebase';
                }
            } else {
                empty.textContent = 'No connections';
            }
            bodyEl.appendChild(empty);
        }

        // Edge legend
        var legend = document.createElement('div');
        legend.style.cssText = 'padding:8px 12px;font-size:10px;color:#555;';
        var lines = [
            ['#a882ff', null, 'import'],
            ['#2dd4bf', null, 'call'],
            ['#fbbf24', null, 'type ref'],
            ['#888', '4 3', 'contains (file → symbol)']
        ];
        lines.forEach(function(l) {
            var row = document.createElement('div');
            row.style.cssText = 'display:flex;align-items:center;gap:8px;height:20px;';
            var svgNS = 'http://www.w3.org/2000/svg';
            var svg = document.createElementNS(svgNS, 'svg');
            svg.setAttribute('width', '24');
            svg.setAttribute('height', '4');
            svg.style.flexShrink = '0';
            var line = document.createElementNS(svgNS, 'line');
            line.setAttribute('x1', '0'); line.setAttribute('y1', '2');
            line.setAttribute('x2', '24'); line.setAttribute('y2', '2');
            line.setAttribute('stroke', l[0]);
            line.setAttribute('stroke-width', '2');
            if (l[1]) line.setAttribute('stroke-dasharray', l[1]);
            svg.appendChild(line);
            row.appendChild(svg);
            row.appendChild(document.createTextNode(l[2]));
            legend.appendChild(row);
        });
        bodyEl.appendChild(legend);

        panel.classList.add('open');
    }

    function addDeadCodeBadge(container, text, color, explanation) {
        var wrapper = document.createElement('div');
        wrapper.style.cssText = 'border-bottom:1px solid #333;padding:6px 12px;';
        var badge = document.createElement('div');
        badge.style.cssText = 'font-size:11px;color:' + color + ';display:flex;align-items:center;gap:6px;';
        var dot = document.createElement('span');
        dot.style.cssText = 'width:6px;height:6px;border-radius:50%;background:' + color + ';flex-shrink:0;';
        badge.appendChild(dot);
        badge.appendChild(document.createTextNode(text));
        wrapper.appendChild(badge);
        if (explanation) {
            var explEl = document.createElement('div');
            explEl.style.cssText = 'font-size:10px;color:#666;margin-top:4px;line-height:1.4;';
            explEl.textContent = explanation;
            wrapper.appendChild(explEl);
        }
        container.appendChild(wrapper);
    }

    function addGroupedSection(title, arrow, items, container) {
        if (items.length === 0) return;

        var groups = {};
        items.forEach(function(item) {
            var key = item.edgeType || 'import';
            if (!groups[key]) groups[key] = [];
            groups[key].push(item);
        });

        var sectionEl = document.createElement('div');
        sectionEl.style.cssText = 'border-bottom:1px solid #333;padding:4px 0;';

        var headerEl = document.createElement('div');
        headerEl.className = 'detail-section-title';
        headerEl.style.display = 'flex';
        headerEl.style.justifyContent = 'space-between';
        headerEl.style.alignItems = 'center';
        var headerText = document.createElement('span');
        headerText.textContent = title + ' (' + items.length + ')';
        headerEl.appendChild(headerText);
        var arrowEl = document.createElement('span');
        arrowEl.textContent = arrow;
        arrowEl.style.cssText = 'font-size:13px;color:#666;';
        arrowEl.title = arrow === '→' ? 'This node depends on these (outgoing edges)' : 'These depend on this node (incoming edges)';
        headerEl.appendChild(arrowEl);
        sectionEl.appendChild(headerEl);

        var groupOrder = ['import', 'call', 'type_ref', 're_export'];
        groupOrder.forEach(function(edgeType) {
            var groupItems = groups[edgeType];
            if (!groupItems) return;

            var subHeader = document.createElement('div');
            subHeader.style.cssText = 'padding:3px 12px 2px;font-size:9px;color:' + (edgeColors[edgeType] || '#666') + ';text-transform:uppercase;letter-spacing:0.5px;';
            subHeader.textContent = (EDGE_TYPE_LABELS[edgeType] || edgeType) + ' (' + groupItems.length + ')';
            sectionEl.appendChild(subHeader);

            groupItems.sort(function(a, b) { return a.name.localeCompare(b.name); });

            groupItems.forEach(function(item) {
                var row = document.createElement('div');
                row.className = 'detail-item';
                row.setAttribute('data-node-id', item.nodeId);

                var dot = document.createElement('span');
                dot.className = 'detail-dot';
                dot.style.background = NODE_COLORS[item.kind] || '#555';
                row.appendChild(dot);

                var nameSpan = document.createElement('span');
                nameSpan.textContent = item.name;
                nameSpan.title = item.name;
                row.appendChild(nameSpan);

                row.addEventListener('mouseenter', function() {
                    var target = nodes.find(function(n) { return n.id === item.nodeId; });
                    if (target) {
                        node.filter(function(n) { return n.id === item.nodeId; })
                            .transition('panel-hover').duration(150)
                            .attr('r', function(n) { return (n.radius || 8) * nodeSizeScale * 1.6; })
                            .attr('stroke', '#fff').attr('stroke-width', 2);
                    }
                });

                row.addEventListener('mouseleave', function() {
                    node.filter(function(n) { return n.id === item.nodeId; })
                        .transition('panel-hover').duration(200)
                        .attr('r', function(n) { return (n.radius || 8) * nodeSizeScale; })
                        .attr('stroke', 'none').attr('stroke-width', 0);
                });

                row.addEventListener('click', function() {
                    var targetNode = nodes.find(function(n) { return n.id === item.nodeId; });
                    if (targetNode) {
                        if (!targetNode._isSymbol && !expandedFiles.has(targetNode.id)) {
                            expandFileNode(targetNode);
                        }
                        activateFocus(targetNode);
                    }
                });

                sectionEl.appendChild(row);
            });
        });

        container.appendChild(sectionEl);
    }

    function hideDetailPanel() {
        detailPanel.classList.remove('open');
    }

    document.getElementById('detail-close').addEventListener('click', function() {
        clearFocus();
    });


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

    var fitBtn = document.getElementById('btn-fit');
    var centeredMode = false;
    var fitIconExpand = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 3 21 3 21 9"/><polyline points="9 21 3 21 3 15"/><line x1="21" y1="3" x2="14" y2="10"/><line x1="3" y1="21" x2="10" y2="14"/></svg>';
    var fitIconShrink = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 14 10 14 10 20"/><polyline points="20 10 14 10 14 4"/><line x1="14" y1="10" x2="21" y2="3"/><line x1="3" y1="21" x2="10" y2="14"/></svg>';

    function updateFitButton() {
        if (centeredMode) {
            fitBtn.innerHTML = fitIconShrink;
            fitBtn.title = 'Switch to overview (F)';
            fitBtn.style.borderColor = '#7f6df2';
            fitBtn.style.color = '#7f6df2';
        } else {
            fitBtn.innerHTML = fitIconExpand;
            fitBtn.title = 'Fit to screen (F)';
            fitBtn.style.borderColor = '#444';
            fitBtn.style.color = '#999';
        }
    }

    function toggleFitMode() {
        if (centeredMode) {
            centeredMode = false;
            fitToScreen();
        } else {
            if (focusActive && focusedNodeId) {
                centeredMode = true;
                var d = nodes.find(function(n) { return n.id === focusedNodeId; });
                if (d) flyToNode(d);
            } else {
                fitToScreen();
            }
        }
        updateFitButton();
    }

    fitBtn.addEventListener('click', toggleFitMode);

    // F key shortcut (when no input focused)
    document.addEventListener('keydown', function(e) {
        if (e.key === 'f' || e.key === 'F') {
            var tag = document.activeElement.tagName;
            if (tag !== 'INPUT' && tag !== 'TEXTAREA' && tag !== 'SELECT') {
                toggleFitMode();
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
                labels.style('opacity', defaultLabelOpacity);
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
            if (!labelVisible(d) || currentZoom < 0.4) return 0;
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
            labels.style('opacity', defaultLabelOpacity);
        }
    });

    function flyToNode(d, useCurrentZoom) {
        var svgW = +svg.attr('width');
        var svgH = +svg.attr('height');
        var scale = useCurrentZoom ? currentZoom : 1.5;
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
        // BFS returning Map of nodeId -> depth (hop distance from source)
        var depthMap = new Map();
        var queue = [{id: nodeId, depth: 0}];
        depthMap.set(nodeId, 0);
        var allEdgesForTraversal = edges.concat(symbolEdges);

        while (queue.length > 0) {
            var item = queue.shift();
            allEdgesForTraversal.forEach(function(e) {
                var src = typeof e.source === 'object' ? e.source.id : e.source;
                var tgt = typeof e.target === 'object' ? e.target.id : e.target;
                if (tgt === item.id && !depthMap.has(src)) {
                    depthMap.set(src, item.depth + 1);
                    queue.push({id: src, depth: item.depth + 1});
                }
            });
        }
        depthMap.delete(nodeId);
        return depthMap;
    }

    function blastColorForDepth(depth) {
        if (depth === 1) return '#c084fc';
        if (depth === 2) return '#a855f7';
        if (depth === 3) return '#7c3aed';
        return '#5b21b6';
    }

    function blastOpacityForDepth(depth) {
        if (depth === 1) return 1.0;
        if (depth === 2) return 0.75;
        if (depth === 3) return 0.55;
        return 0.4;
    }

    function showBlastRadius(sourceNode) {
        blastRadiusSourceId = sourceNode.id;
        var depthMap = computeBlastRadius(sourceNode.id);
        var maxDepth = 0;
        depthMap.forEach(function(d) { if (d > maxDepth) maxDepth = d; });

        node.transition().duration(FADE_IN).ease(d3.easeCubicOut)
            .attr('fill', function(n) {
                if (n.id === sourceNode.id) return '#7f6df2';
                var depth = depthMap.get(n.id);
                if (depth !== undefined) return blastColorForDepth(depth);
                return nodeColor(n);
            })
            .style('opacity', function(n) {
                if (n.id === sourceNode.id) return 1;
                var depth = depthMap.get(n.id);
                if (depth !== undefined) return blastOpacityForDepth(depth);
                return 0.08;
            });

        labels.transition().duration(FADE_IN).ease(d3.easeCubicOut)
            .style('opacity', function(n) {
                if (!labelVisible(n) || currentZoom < 0.4) return 0;
                if (n.id === sourceNode.id) return 1;
                var depth = depthMap.get(n.id);
                if (depth !== undefined) return depth <= 2 ? 1 : 0.5;
                return 0.04;
            });

        var useTypedBlast = fileLens === 'all-edges' || viewLevel === 'symbols';
        link.transition().duration(FADE_IN).ease(d3.easeCubicOut)
            .attr('stroke', function(e) {
                var si = typeof e.source === 'object' ? e.source.id : e.source;
                var ti = typeof e.target === 'object' ? e.target.id : e.target;
                var sd = depthMap.get(si), td = depthMap.get(ti);
                var srcIn = si === sourceNode.id || sd !== undefined;
                var tgtIn = ti === sourceNode.id || td !== undefined;
                if (srcIn && tgtIn) return useTypedBlast ? edgeColor(e) : '#a882ff';
                return edgeColor(e);
            })
            .attr('stroke-opacity', function(e) {
                var si = typeof e.source === 'object' ? e.source.id : e.source;
                var ti = typeof e.target === 'object' ? e.target.id : e.target;
                var sd = depthMap.get(si), td = depthMap.get(ti);
                var srcIn = si === sourceNode.id || sd !== undefined;
                var tgtIn = ti === sourceNode.id || td !== undefined;
                return srcIn && tgtIn ? 0.5 : 0.04;
            })
            .attr('marker-end', function(e) {
                if (e._isParentEdge) return 'none';
                var si = typeof e.source === 'object' ? e.source.id : e.source;
                var ti = typeof e.target === 'object' ? e.target.id : e.target;
                var sd = depthMap.get(si), td = depthMap.get(ti);
                var srcIn = si === sourceNode.id || sd !== undefined;
                var tgtIn = ti === sourceNode.id || td !== undefined;
                if (srcIn && tgtIn) return useTypedBlast ? edgeMarker(e) : 'url(#arrow-active)';
                return 'none';
            });

        var promptEl = document.getElementById('blast-radius-prompt');
        promptEl.querySelector('.ctrl-label').textContent =
            depthMap.size + '/' + nodes.length + ' files affected (' + maxDepth + ' hops max)';
        updateStateIndicator();
    }

    function clearBlastRadius() {
        blastRadiusSourceId = null;
        node.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
            .attr('fill', function(n) { return nodeColor(n); })
            .style('opacity', 1);
        labels.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
            .style('opacity', defaultLabelOpacity);
        link.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
            .attr('stroke', edgeColor).style('opacity', 0.25).attr('marker-end', edgeMarker);
        updateStateIndicator();
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

    // === Filter System (INTR-05, INTR-06, INTR-07, D-79, D-80) ===

    var filterState = {
        dirQuery: '',
        darkRoom: false,
        showNeighbors: true,
        symbolTypes: {
            'function': true, 'class': true, 'type': true,
            'interface': true, 'hook': true, 'enum': true
        },
        edgeTypes: {
            'import': true, 'call': true, 'type_ref': true
        }
    };

    function symbolKindMatch(kind) {
        var k = kind === 'interface' ? 'type' : kind;
        return filterState.symbolTypes[k] !== false;
    }

    function hasSymbolTypeFilter() {
        return Object.keys(filterState.symbolTypes).some(function(k) { return !filterState.symbolTypes[k]; });
    }

    function isNodeVisible(d, skipFocusExempt) {
        // Focused node is always visible
        if (!skipFocusExempt && focusActive && focusedNodeId && d.id === focusedNodeId) return true;
        // Expanded child symbols of focused node: visible if they match symbol type filter
        if (!skipFocusExempt && focusActive && focusedNodeId && d._isSymbol && d._parentId === focusedNodeId) {
            return symbolKindMatch(d.kind);
        }

        var dirMatch = !filterState.dirQuery ||
            (d.path || d.file_path || '').toLowerCase().includes(filterState.dirQuery);

        if (d._isSymbol) {
            return dirMatch && symbolKindMatch(d.kind);
        }

        if (!dirMatch) return false;

        if (filterState.darkRoom && hasSymbolTypeFilter()) {
            var syms = symbolsByFile[d.id] || [];
            return syms.some(function(s) { return symbolKindMatch(s.kind); });
        }

        return true;
    }

    function isEdgeVisible(e) {
        var et = e.edge_type || 'import';
        if (et === 'parent_child') return true;
        if (et === 're_export') return true;
        return filterState.edgeTypes[et] !== false;
    }

    function isSoloActive() {
        var symKeys = Object.keys(filterState.symbolTypes);
        var edgeKeys = Object.keys(filterState.edgeTypes);
        var symOn = symKeys.filter(function(k) { return filterState.symbolTypes[k]; }).length;
        var edgeOn = edgeKeys.filter(function(k) { return filterState.edgeTypes[k]; }).length;
        // Solo = exactly one symbol type on (interface mirrors type, so count as 2) or one edge type on
        return (symOn <= 2 && symOn < symKeys.length) || (edgeOn === 1 && edgeOn < edgeKeys.length);
    }

    function applyFilters() {
        var focusNode = focusActive && focusedNodeId ? nodes.find(function(n) { return n.id === focusedNodeId; }) : null;
        var focusConnected = focusNode ? (adjacency.get(focusNode.id) || new Set()) : null;
        var dr = filterState.darkRoom;

        function isFocusRelevant(d) {
            if (!focusNode) return true;
            if (d.id === focusNode.id) return true;
            if (d._parentId === focusNode.id) return true;
            if (!filterState.showNeighbors) return false;
            if (focusConnected) return focusConnected.has(d.id);
            return true;
        }

        // Re-apply focus fill colors
        if (focusNode) {
            node.attr('fill', function(n) {
                if (n.id === focusNode.id) return '#7f6df2';
                if (isFocusRelevant(n)) return nodeColor(n);
                return nodeColor(n);
            });
        }

        node.style('opacity', function(d) {
            if (!isNodeVisible(d)) return dr ? 0 : 0.08;
            if (focusNode) return isFocusRelevant(d) ? 1 : (dr ? 0 : 0.1);
            return 1;
        }).style('pointer-events', function(d) {
            if (!isNodeVisible(d)) return 'none';
            if (focusNode && dr && !isFocusRelevant(d)) return 'none';
            return null;
        });

        labels.style('opacity', function(d) {
            if (!labelVisible(d) || currentZoom < 0.4) return 0;
            if (!isNodeVisible(d)) return dr ? 0 : 0.04;
            if (focusNode) return isFocusRelevant(d) ? 1 : (dr ? 0 : 0.04);
            return 1;
        });

        var useTypedFocus = fileLens === 'all-edges' || viewLevel === 'symbols' || expandedFiles.size > 0;
        link.attr('stroke', function(e) {
            if (e._isParentEdge) return focusNode ? '#666' : '#444';
            if (!focusNode) return edgeColor(e);
            var si = typeof e.source === 'object' ? e.source.id : e.source;
            var ti = typeof e.target === 'object' ? e.target.id : e.target;
            if (si === focusNode.id || ti === focusNode.id) return useTypedFocus ? edgeColor(e) : '#a882ff';
            return edgeColor(e);
        })
        .attr('stroke-opacity', null)
        .attr('marker-end', function(e) {
            if (e._isParentEdge) return 'none';
            if (!focusNode) return edgeMarker(e);
            var si = typeof e.source === 'object' ? e.source.id : e.source;
            var ti = typeof e.target === 'object' ? e.target.id : e.target;
            var srcN = nodes.find(function(n) { return n.id === si; });
            var tgtN = nodes.find(function(n) { return n.id === ti; });
            var isFocusEdge = si === focusNode.id || ti === focusNode.id ||
                (srcN && srcN._parentId === focusNode.id) || (tgtN && tgtN._parentId === focusNode.id);
            if (isFocusEdge) return useTypedFocus ? edgeMarker(e) : 'url(#arrow-active)';
            return 'none';
        })
        .style('opacity', function(e) {
            if (!isEdgeVisible(e)) return 0;
            var si = typeof e.source === 'object' ? e.source.id : e.source;
            var ti = typeof e.target === 'object' ? e.target.id : e.target;
            var srcN = nodes.find(function(n) { return n.id === si; });
            var tgtN = nodes.find(function(n) { return n.id === ti; });
            if (srcN && !isNodeVisible(srcN)) return 0;
            if (tgtN && !isNodeVisible(tgtN)) return 0;
            if (focusNode && !filterState.showNeighbors) {
                var srcInternal = si === focusNode.id || (srcN && srcN._parentId === focusNode.id);
                var tgtInternal = ti === focusNode.id || (tgtN && tgtN._parentId === focusNode.id);
                if (!srcInternal || !tgtInternal) return 0;
            }
            if (focusNode) return (si === focusNode.id || ti === focusNode.id || (srcN && srcN._parentId === focusNode.id) || (tgtN && tgtN._parentId === focusNode.id)) ? 0.7 : (dr ? 0 : 0.04);
            return 0.25;
        })
        .style('pointer-events', function(e) {
            return isEdgeVisible(e) ? null : 'none';
        });
        updatePillCounts();
    }

    // Info icon tooltips — position fixed to avoid panel overflow clipping
    document.querySelectorAll('.info-icon').forEach(function(icon) {
        var tip = icon.querySelector('.info-tip');
        if (!tip) return;
        icon.addEventListener('mouseenter', function() {
            var r = icon.getBoundingClientRect();
            tip.style.left = (r.right + 8) + 'px';
            tip.style.top = (r.top + r.height / 2) + 'px';
            tip.style.transform = 'translateY(-50%)';
            tip.style.display = 'block';
        });
        icon.addEventListener('mouseleave', function() { tip.style.display = 'none'; });
    });

    // Dark room mode — hide filtered-out nodes entirely instead of fading
    document.getElementById('toggle-dark-room').addEventListener('change', function() {
        filterState.darkRoom = this.checked;
        applyFilters();
    });

    // Directory filter (INTR-05)
    document.getElementById('filter-dir').addEventListener('input', function() {
        filterState.dirQuery = this.value.trim().toLowerCase();
        applyFilters();
    });

    // Symbol type filters (INTR-06)
    var symbolFilterMap = {
        'filter-fn': 'function',
        'filter-class': 'class',
        'filter-type': 'type',  // Also covers 'interface'
        'filter-hook': 'hook',
        'filter-enum': 'enum'
    };

    Object.keys(symbolFilterMap).forEach(function(checkboxId) {
        var kindKey = symbolFilterMap[checkboxId];
        document.getElementById(checkboxId).addEventListener('change', function() {
            filterState.symbolTypes[kindKey] = this.checked;
            if (kindKey === 'type') {
                filterState.symbolTypes['interface'] = this.checked;
            }
            syncMasterToggles();
            syncPillsFromState();
            updateSoloButtons();
            applyFilters();
        });
    });

    // Edge type filters (INTR-07)
    var edgeFilterMap = {
        'filter-edge-import': 'import',
        'filter-edge-call': 'call',
        'filter-edge-typeref': 'type_ref'
    };

    Object.keys(edgeFilterMap).forEach(function(checkboxId) {
        var edgeKey = edgeFilterMap[checkboxId];
        document.getElementById(checkboxId).addEventListener('change', function() {
            filterState.edgeTypes[edgeKey] = this.checked;
            syncMasterToggles();
            syncPillsFromState();
            updateSoloButtons();
            applyFilters();
        });
    });

    // Master toggle: all symbol types
    document.getElementById('toggle-all-symbols').addEventListener('change', function() {
        var on = this.checked;
        Object.keys(filterState.symbolTypes).forEach(function(k) { filterState.symbolTypes[k] = on; });
        Object.keys(symbolFilterMap).forEach(function(id) { document.getElementById(id).checked = on; });
        syncPillsFromState();
        updateSoloButtons();
        applyFilters();
    });

    // Master toggle: all edge types
    document.getElementById('toggle-all-edges').addEventListener('change', function() {
        var on = this.checked;
        Object.keys(filterState.edgeTypes).forEach(function(k) { filterState.edgeTypes[k] = on; });
        Object.keys(edgeFilterMap).forEach(function(id) { document.getElementById(id).checked = on; });
        syncPillsFromState();
        updateSoloButtons();
        applyFilters();
    });

    function syncMasterToggles() {
        var allSymOn = Object.keys(filterState.symbolTypes).every(function(k) { return filterState.symbolTypes[k]; });
        var anySymOn = Object.keys(filterState.symbolTypes).some(function(k) { return filterState.symbolTypes[k]; });
        var symToggle = document.getElementById('toggle-all-symbols');
        symToggle.checked = anySymOn;
        symToggle.indeterminate = anySymOn && !allSymOn;

        var allEdgeOn = Object.keys(filterState.edgeTypes).every(function(k) { return filterState.edgeTypes[k]; });
        var anyEdgeOn = Object.keys(filterState.edgeTypes).some(function(k) { return filterState.edgeTypes[k]; });
        var edgeToggle = document.getElementById('toggle-all-edges');
        edgeToggle.checked = anyEdgeOn;
        edgeToggle.indeterminate = anyEdgeOn && !allEdgeOn;
    }

    // Solo buttons in filter panel
    function updateSoloButtons() {
        document.querySelectorAll('.solo-btn').forEach(function(btn) {
            var parts = btn.getAttribute('data-solo').split(':');
            var category = parts[0];
            var key = parts[1];
            var isSolo = false;
            if (category === 'symbol') {
                isSolo = filterState.symbolTypes[key] &&
                    Object.keys(filterState.symbolTypes).every(function(k) {
                        return k === key || (key === 'type' && k === 'interface') || !filterState.symbolTypes[k];
                    });
            } else {
                isSolo = filterState.edgeTypes[key] &&
                    Object.keys(filterState.edgeTypes).every(function(k) {
                        return k === key || !filterState.edgeTypes[k];
                    });
            }
            btn.classList.toggle('active', isSolo);
        });
    }

    document.querySelectorAll('.solo-btn').forEach(function(btn) {
        btn.addEventListener('click', function() {
            var parts = this.getAttribute('data-solo').split(':');
            var category = parts[0];
            var key = parts[1];
            var wasSolo = this.classList.contains('active');

            if (category === 'symbol') {
                Object.keys(filterState.symbolTypes).forEach(function(k) { filterState.symbolTypes[k] = wasSolo; });
                if (!wasSolo) {
                    filterState.symbolTypes[key] = true;
                    if (key === 'type') filterState.symbolTypes['interface'] = true;
                }
                Object.keys(symbolFilterMap).forEach(function(id) {
                    document.getElementById(id).checked = filterState.symbolTypes[symbolFilterMap[id]];
                });
            } else {
                Object.keys(filterState.edgeTypes).forEach(function(k) { filterState.edgeTypes[k] = wasSolo; });
                if (!wasSolo) filterState.edgeTypes[key] = true;
                Object.keys(edgeFilterMap).forEach(function(id) {
                    document.getElementById(id).checked = filterState.edgeTypes[edgeFilterMap[id]];
                });
            }
            syncMasterToggles();
            updateSoloButtons();
            syncPillsFromState();
            applyFilters();
        });
    });

    // Quick-Filter Pills (D-80) — dynamic, bidirectional sync with panel checkboxes
    var allPillDefs = [
        { stateKey: 'function', label: 'Functions', checkboxId: 'filter-fn', type: 'symbol', color: '#2dd4bf' },
        { stateKey: 'class', label: 'Classes', checkboxId: 'filter-class', type: 'symbol', color: '#f87171' },
        { stateKey: 'type', label: 'Types', checkboxId: 'filter-type', type: 'symbol', color: '#fbbf24' },
        { stateKey: 'hook', label: 'Hooks', checkboxId: 'filter-hook', type: 'symbol', color: '#a78bfa' },
        { stateKey: 'enum', label: 'Enums', checkboxId: 'filter-enum', type: 'symbol', color: '#4ade80' },
        { stateKey: 'import', label: 'Imports', checkboxId: 'filter-edge-import', type: 'edge', color: '#a882ff' },
        { stateKey: 'call', label: 'Calls', checkboxId: 'filter-edge-call', type: 'edge', color: '#2dd4bf' },
        { stateKey: 'type_ref', label: 'Type refs', checkboxId: 'filter-edge-typeref', type: 'edge', color: '#fbbf24' }
    ];

    var quickFiltersEl = document.getElementById('quick-filters');

    function countForDef(def) {
        if (def.type === 'symbol') {
            return (data.symbols || []).filter(function(s) {
                var k = s.kind === 'interface' ? 'type' : s.kind;
                return k === def.stateKey;
            }).length;
        } else {
            var edgeSource = viewLevel === 'symbols' ? symbolEdges :
                             fileLens === 'all-edges' ? fileEdgesTyped : fileEdgesImportOnly;
            return edgeSource.filter(function(e) {
                return (e.edge_type || 'import') === def.stateKey;
            }).length;
        }
    }

    function rebuildPills() {
        quickFiltersEl.innerHTML = '';
        allPillDefs.forEach(function(def) {
            var count = countForDef(def);
            if (count === 0) return;

            var pill = document.createElement('button');
            pill.className = 'pill';
            pill.setAttribute('data-filter', def.stateKey);
            pill.setAttribute('data-type', def.type);
            pill.title = 'Click to toggle · ⌥ click to solo';
            pill.style.cssText = 'padding:4px 10px;font-size:11px;border-radius:12px;border:1px solid #444;background:#333;color:#999;cursor:pointer;';
            pill.textContent = def.label + ' · ' + count;

            var isActive = def.type === 'symbol' ? filterState.symbolTypes[def.stateKey] : filterState.edgeTypes[def.stateKey];
            if (isActive) {
                pill.classList.add('active');
                pill.style.borderColor = def.color;
                pill.style.background = def.color + '22';
                pill.style.color = def.color;
            }

            pill.addEventListener('click', function(e) {
                if (e.altKey) {
                    if (def.type === 'symbol') {
                        var allOff = Object.keys(filterState.symbolTypes).every(function(k) { return !filterState.symbolTypes[k] || k === def.stateKey || (def.stateKey === 'type' && k === 'interface'); });
                        var soloActive = filterState.symbolTypes[def.stateKey] && allOff;
                        Object.keys(filterState.symbolTypes).forEach(function(k) { filterState.symbolTypes[k] = soloActive; });
                        if (!soloActive) {
                            filterState.symbolTypes[def.stateKey] = true;
                            if (def.stateKey === 'type') filterState.symbolTypes['interface'] = true;
                        }
                        Object.keys(symbolFilterMap).forEach(function(id) { document.getElementById(id).checked = filterState.symbolTypes[symbolFilterMap[id]]; });
                    } else {
                        var allEdgeOff = Object.keys(filterState.edgeTypes).every(function(k) { return !filterState.edgeTypes[k] || k === def.stateKey; });
                        var edgeSoloActive = filterState.edgeTypes[def.stateKey] && allEdgeOff;
                        Object.keys(filterState.edgeTypes).forEach(function(k) { filterState.edgeTypes[k] = edgeSoloActive; });
                        if (!edgeSoloActive) filterState.edgeTypes[def.stateKey] = true;
                        Object.keys(edgeFilterMap).forEach(function(id) { document.getElementById(id).checked = filterState.edgeTypes[edgeFilterMap[id]]; });
                    }
                } else {
                    if (def.type === 'symbol') {
                        var cur = filterState.symbolTypes[def.stateKey];
                        filterState.symbolTypes[def.stateKey] = !cur;
                        if (def.stateKey === 'type') filterState.symbolTypes['interface'] = !cur;
                        document.getElementById(def.checkboxId).checked = !cur;
                    } else {
                        var curEdge = filterState.edgeTypes[def.stateKey];
                        filterState.edgeTypes[def.stateKey] = !curEdge;
                        document.getElementById(def.checkboxId).checked = !curEdge;
                    }
                }
                syncMasterToggles();
                rebuildPills();
                applyFilters();
            });

            quickFiltersEl.appendChild(pill);
        });
    }

    function syncPillsFromState() { rebuildPills(); }
    function updatePillCounts() { rebuildPills(); }

    rebuildPills();
    updateContextualControls();

    // === Panel controls ===

    // Filters: reset (replaces prior reset handler — now also resets filter panel controls)
    document.getElementById('btn-reset-filters').addEventListener('click', function() {
        document.getElementById('search-files').value = '';
        document.getElementById('filter-dir').value = '';
        document.getElementById('toggle-orphans').checked = true;
        document.getElementById('toggle-dark-room').checked = false;
        filterState.darkRoom = false;
        document.getElementById('toggle-neighbors').checked = true;
        filterState.showNeighbors = true;

        // Reset symbol type filters
        Object.keys(symbolFilterMap).forEach(function(id) {
            document.getElementById(id).checked = true;
        });
        filterState.symbolTypes = { 'function': true, 'class': true, 'type': true, 'interface': true, 'hook': true, 'enum': true };

        // Reset edge type filters
        Object.keys(edgeFilterMap).forEach(function(id) {
            document.getElementById(id).checked = true;
        });
        filterState.edgeTypes = { 'import': true, 'call': true, 'type_ref': true };
        filterState.dirQuery = '';

        syncMasterToggles();
        syncPillsFromState();

        node.style('display', null).style('opacity', 1).style('pointer-events', null);
        labels.style('display', null).style('opacity', currentZoom < 0.4 ? 0 : 1);
        link.attr('stroke', edgeColor).style('opacity', 0.25).style('pointer-events', null);
        updatePillCounts();
    });

    // Filters: search
    document.getElementById('search-files').addEventListener('input', function(e) {
        var q = e.target.value.toLowerCase();
        node.style('opacity', function(d) { return !q || (d.path || d.file_path || '').toLowerCase().includes(q) ? 1 : 0.1; });
        labels.style('opacity', function(d) { return !q || (d.path || d.file_path || '').toLowerCase().includes(q) ? 1 : 0.05; });
    });

    // Filters: show neighbors
    document.getElementById('toggle-neighbors').addEventListener('change', function() {
        filterState.showNeighbors = this.checked;
        applyFilters();
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
        var checked = this.checked;
        link.attr('marker-end', function(e) { return checked ? edgeMarker(e) : 'none'; });
    });

    // Display: labels
    document.getElementById('toggle-labels').addEventListener('change', updateLabelVisibility);
    document.getElementById('toggle-symbol-labels').addEventListener('change', updateLabelVisibility);

    // Display: node size
    document.getElementById('slider-node-size').addEventListener('input', function() {
        nodeSizeScale = parseFloat(this.value);
        node.attr('r', function(d) { return d.radius * nodeSizeScale; });
        simulation.force('collide', d3.forceCollide().radius(function(d) { return d.radius * nodeSizeScale + 8; }));
        simulation.alpha(0.3).restart();
    });

    // Display: label size
    document.getElementById('slider-label-size').addEventListener('input', function() {
        var baseSize = parseFloat(this.value);
        labels.attr('font-size', function(d) { return (d._isSymbol ? baseSize - 2 : baseSize) + 'px'; });
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
