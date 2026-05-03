document.addEventListener('DOMContentLoaded', function() {
    loadAndRender();
});

async function loadAndRender() {
    var data;
    try {
        var response = await fetch('/api/graph');
        if (!response.ok) throw new Error('HTTP ' + response.status);
        data = await response.json();
    } catch (err) {
        var errorState = document.getElementById('error-state');
        document.getElementById('error-message').textContent =
            'Failed to load graph data. Check that the cgraph server is running.';
        errorState.style.display = 'block';
        return;
    }

    if (!data.nodes || data.nodes.length === 0) {
        var emptyState = document.getElementById('empty-state');
        var projectName = data.project_name || 'this project';
        document.getElementById('empty-message').textContent =
            'cgraph found no source files in ' + projectName +
            '. Check that the path is correct and contains .ts, .tsx, or other supported files.';
        emptyState.style.display = 'block';
        return;
    }

    document.getElementById('project-name').textContent = data.project_name || '';
    var s = data.stats;
    document.getElementById('stats').textContent =
        s.files + ' files • ' + s.symbols + ' symbols • ' + s.edges + ' edges • ' + s.elapsed_ms + 'ms';

    var width = window.innerWidth;
    var height = window.innerHeight - 40;

    var svg = d3.select('#graph')
        .append('svg')
        .attr('width', width)
        .attr('height', height);

    var defs = svg.append('defs');

    defs.append('marker')
        .attr('id', 'arrow')
        .attr('viewBox', '-0 -5 10 10')
        .attr('refX', 10).attr('refY', 0)
        .attr('orient', 'auto')
        .attr('markerWidth', 6).attr('markerHeight', 4)
      .append('path')
        .attr('d', 'M 0,-5 L 10,0 L 0,5')
        .attr('fill', '#444');

    defs.append('marker')
        .attr('id', 'arrow-active')
        .attr('viewBox', '-0 -5 10 10')
        .attr('refX', 10).attr('refY', 0)
        .attr('orient', 'auto')
        .attr('markerWidth', 6).attr('markerHeight', 4)
      .append('path')
        .attr('d', 'M 0,-5 L 10,0 L 0,5')
        .attr('fill', '#a882ff');

    var g = svg.append('g');

    var currentZoom = 1;
    svg.call(d3.zoom()
        .scaleExtent([0.1, 10])
        .on('zoom', function(event) {
            g.attr('transform', event.transform);
            currentZoom = event.transform.k;
            updateLabelVisibility();
        }));

    var nodes = data.nodes.map(function(d) { return Object.assign({}, d); });
    var edges = data.edges.map(function(d) { return Object.assign({}, d); });

    var adjacency = new Map();
    nodes.forEach(function(n) { adjacency.set(n.id, new Set()); });

    var simulation = d3.forceSimulation(nodes)
        .force('link', d3.forceLink(edges).id(function(d) { return d.id; }).distance(50).strength(0.9))
        .force('charge', d3.forceManyBody().strength(-60))
        .force('center', d3.forceCenter(width / 2, height / 2).strength(0.12))
        .force('collide', d3.forceCollide().radius(function(d) { return d.radius + 8; }))
        .force('x', d3.forceX(width / 2).strength(0.06))
        .force('y', d3.forceY(height / 2).strength(0.06))
        .stop();

    // Pre-settle so initial render has no jitter (VIZN-07)
    simulation.tick(300);

    // Build adjacency after forceLink has resolved source/target to objects
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
            x: target.x - (dx / dist) * (targetRadius + 4),
            y: target.y - (dy / dist) * (targetRadius + 4)
        };
    }

    // Edges
    var linkGroup = g.append('g').attr('class', 'edges');
    var link = linkGroup.selectAll('line')
        .data(edges)
        .join('line')
        .attr('stroke', '#444')
        .attr('stroke-opacity', 0.25)
        .attr('marker-end', 'url(#arrow)');

    // Nodes
    var nodeGroup = g.append('g').attr('class', 'nodes');
    var node = nodeGroup.selectAll('circle')
        .data(nodes)
        .join('circle')
        .attr('r', function(d) { return d.radius; })
        .attr('fill', '#555')
        .attr('stroke', 'none')
        .style('cursor', 'grab');

    // Labels
    var labelGroup = g.append('g').attr('class', 'labels');
    var labels = labelGroup.selectAll('text')
        .data(nodes)
        .join('text')
        .attr('text-anchor', 'middle')
        .attr('fill', '#999')
        .attr('font-size', '11px')
        .attr('pointer-events', 'none')
        .text(function(d) { return d.filename; });

    // Position update — used by tick handler and initial render
    function updatePositions() {
        node.attr('cx', function(d) { return d.x; })
            .attr('cy', function(d) { return d.y; });

        labels.attr('x', function(d) { return d.x; })
             .attr('y', function(d) { return d.y + d.radius + 6 + 11; });

        link.each(function(d) {
            var src = typeof d.source === 'object' ? d.source : nodes.find(function(n) { return n.id === d.source; });
            var tgt = typeof d.target === 'object' ? d.target : nodes.find(function(n) { return n.id === d.target; });
            if (!src || !tgt) return;
            var ep = adjustedEndpoint(src, tgt, tgt.radius);
            d3.select(this)
                .attr('x1', src.x).attr('y1', src.y)
                .attr('x2', ep.x).attr('y2', ep.y);
        });
    }

    // Initial static positions from pre-settled simulation
    updatePositions();

    // Re-enable simulation tick handler for drag interactions
    simulation.on('tick', updatePositions);

    // Drag behavior — Obsidian style: drag repositions, simulation re-settles
    var drag = d3.drag()
        .on('start', function(event, d) {
            if (!event.active) simulation.alphaTarget(0.3).restart();
            d.fx = d.x;
            d.fy = d.y;
            d3.select(this).style('cursor', 'grabbing');
        })
        .on('drag', function(event, d) {
            d.fx = event.x;
            d.fy = event.y;
        })
        .on('end', function(event, d) {
            if (!event.active) simulation.alphaTarget(0);
            d.fx = null;
            d.fy = null;
            d3.select(this).style('cursor', 'grab');
        });

    node.call(drag);

    // Semantic zoom: hide labels when zoomed out far
    function updateLabelVisibility() {
        if (hoverActive) return;
        labels.style('opacity', currentZoom < 0.4 ? 0 : Math.min(1, (currentZoom - 0.3) * 3));
    }

    // Hover highlight with smooth D3 transitions
    var hoverActive = false;
    var FADE_IN = 250;
    var FADE_OUT = 400;

    node.on('mouseenter', function(event, d) {
        hoverActive = true;
        var connected = adjacency.get(d.id) || new Set();

        node.transition().duration(FADE_IN).ease(d3.easeCubicOut)
            .attr('fill', function(n) {
                if (n.id === d.id) return '#7f6df2';
                if (connected.has(n.id)) return '#a882ff';
                return '#555';
            })
            .style('opacity', function(n) {
                if (n.id === d.id || connected.has(n.id)) return 1;
                return 0.12;
            });

        labels.transition().duration(FADE_IN).ease(d3.easeCubicOut)
            .style('opacity', function(n) {
                if (currentZoom < 0.4) return 0;
                if (n.id === d.id || connected.has(n.id)) return 1;
                return 0.06;
            });

        link.transition().duration(FADE_IN).ease(d3.easeCubicOut)
            .attr('stroke', function(e) {
                var srcId = typeof e.source === 'object' ? e.source.id : e.source;
                var tgtId = typeof e.target === 'object' ? e.target.id : e.target;
                if (srcId === d.id || tgtId === d.id) return '#a882ff';
                return '#444';
            })
            .attr('stroke-opacity', function(e) {
                var srcId = typeof e.source === 'object' ? e.source.id : e.source;
                var tgtId = typeof e.target === 'object' ? e.target.id : e.target;
                if (srcId === d.id || tgtId === d.id) return 0.7;
                return 0.04;
            })
            .attr('marker-end', function(e) {
                var srcId = typeof e.source === 'object' ? e.source.id : e.source;
                var tgtId = typeof e.target === 'object' ? e.target.id : e.target;
                if (srcId === d.id || tgtId === d.id) return 'url(#arrow-active)';
                return 'url(#arrow)';
            });

        // Tooltip
        var counts = d.export_counts;
        var kinds = [
            ['functions', counts.functions],
            ['classes', counts.classes],
            ['types', counts.types],
            ['interfaces', counts.interfaces],
            ['hooks', counts.hooks],
            ['enums', counts.enums]
        ];
        var parts = kinds.filter(function(k) { return k[1] > 0; }).map(function(k) { return k[1] + ' ' + k[0]; });
        var exportSummary = parts.length > 0 ? parts.join(', ') : 'no exports';

        var tooltip = document.getElementById('tooltip');
        tooltip.querySelector('.tooltip-path').textContent = d.path;
        tooltip.querySelector('.tooltip-exports').textContent = exportSummary;
        tooltip.querySelector('.tooltip-edges').textContent = d.incoming + ' incoming • ' + d.outgoing + ' outgoing';
        tooltip.style.display = 'block';
        tooltip.style.left = (event.pageX + 12) + 'px';
        tooltip.style.top = event.pageY + 'px';
    });

    node.on('mousemove', function(event) {
        var tooltip = document.getElementById('tooltip');
        tooltip.style.left = (event.pageX + 12) + 'px';
        tooltip.style.top = event.pageY + 'px';
    });

    node.on('mouseleave', function() {
        hoverActive = false;

        node.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
            .attr('fill', '#555')
            .style('opacity', 1);

        labels.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
            .style('opacity', currentZoom < 0.4 ? 0 : 1);

        link.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
            .attr('stroke', '#444')
            .attr('stroke-opacity', 0.25)
            .attr('marker-end', 'url(#arrow)');

        document.getElementById('tooltip').style.display = 'none';
    });

    // Legend collapse/expand
    var legendHeader = document.querySelector('.legend-header');
    var legendContent = document.querySelector('.legend-content');
    var legendExpanded = true;

    legendHeader.addEventListener('click', function() {
        legendExpanded = !legendExpanded;
        legendContent.style.display = legendExpanded ? 'block' : 'none';
        legendHeader.innerHTML = legendExpanded ? 'Legend &#9662;' : 'Legend &#9652;';
    });

    // Directory halos
    var halosToggle = document.getElementById('halos-toggle');
    var halosGroup = null;

    function renderHalos() {
        if (halosGroup) { halosGroup.remove(); halosGroup = null; }
        if (!halosToggle.checked) return;

        var dirMap = new Map();
        nodes.forEach(function(d) {
            var lastSlash = d.path.lastIndexOf('/');
            var dir = lastSlash >= 0 ? d.path.substring(0, lastSlash) : '';
            if (!dirMap.has(dir)) dirMap.set(dir, []);
            dirMap.get(dir).push(d);
        });

        halosGroup = g.insert('g', '.edges').attr('class', 'halos');

        dirMap.forEach(function(groupNodes, dir) {
            if (groupNodes.length < 2) return;

            var cx = groupNodes.reduce(function(sum, n) { return sum + n.x; }, 0) / groupNodes.length;
            var cy = groupNodes.reduce(function(sum, n) { return sum + n.y; }, 0) / groupNodes.length;

            var rawPoints = groupNodes.map(function(n) { return [n.x, n.y]; });
            var hull = d3.polygonHull(rawPoints);
            if (!hull) return;

            var expanded = hull.map(function(pt) {
                var dx = pt[0] - cx;
                var dy = pt[1] - cy;
                var len = Math.sqrt(dx * dx + dy * dy);
                if (len === 0) return pt;
                return [pt[0] + (dx / len) * 30, pt[1] + (dy / len) * 30];
            });

            halosGroup.append('path')
                .attr('d', 'M' + expanded.map(function(p) { return p[0] + ',' + p[1]; }).join('L') + 'Z')
                .attr('stroke', '#7f6df2')
                .attr('stroke-opacity', 0.15)
                .attr('stroke-dasharray', '4 2')
                .attr('fill', '#7f6df2')
                .attr('fill-opacity', 0.04);
        });
    }

    halosToggle.addEventListener('change', renderHalos);

    window.addEventListener('resize', function() {
        svg.attr('width', window.innerWidth).attr('height', window.innerHeight - 40);
    });
}
