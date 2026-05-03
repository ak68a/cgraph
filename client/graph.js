// cgraph browser client — D3 force graph visualization
// Vanilla JS, no build step (D-61). Requires d3.v7.min.js loaded before this script.

document.addEventListener('DOMContentLoaded', function() {
    loadAndRender();
});

async function loadAndRender() {
    // 1. Fetch graph data from the server
    let data;
    try {
        const response = await fetch('/api/graph');
        if (!response.ok) throw new Error('HTTP ' + response.status);
        data = await response.json();
    } catch (err) {
        const errorState = document.getElementById('error-state');
        const errorMessage = document.getElementById('error-message');
        errorMessage.textContent = 'Failed to load graph data. Check that the cgraph server is running.';
        errorState.style.display = 'block';
        return;
    }

    // 2. Handle empty state
    if (!data.nodes || data.nodes.length === 0) {
        const emptyState = document.getElementById('empty-state');
        const emptyMessage = document.getElementById('empty-message');
        const projectName = data.project_name || 'this project';
        emptyMessage.textContent = 'cgraph found no source files in ' + projectName + '. Check that the path is correct and contains .ts, .tsx, or other supported files.';
        emptyState.style.display = 'block';
        return;
    }

    // 3. Update header
    document.getElementById('project-name').textContent = data.project_name || '';
    const s = data.stats;
    document.getElementById('stats').textContent =
        s.files + ' files • ' + s.symbols + ' symbols • ' + s.edges + ' edges • ' + s.elapsed_ms + 'ms';

    // 4. Create SVG
    const width = window.innerWidth;
    const height = window.innerHeight - 40; // minus header height

    const svg = d3.select('#graph')
        .append('svg')
        .attr('width', width)
        .attr('height', height);

    // 5. Define arrowhead marker in SVG defs
    svg.append('defs').append('marker')
        .attr('id', 'arrowhead')
        .attr('viewBox', '-0 -5 10 10')
        .attr('refX', 13)
        .attr('refY', 0)
        .attr('orient', 'auto')
        .attr('markerWidth', 6)
        .attr('markerHeight', 4)
      .append('path')
        .attr('d', 'M 0,-5 L 10,0 L 0,5')
        .attr('fill', '#555555');

    // 6. Create container group for zoom/pan
    const g = svg.append('g');
    svg.call(d3.zoom()
        .scaleExtent([0.1, 10])
        .on('zoom', function(event) {
            g.attr('transform', event.transform);
        }));

    // 7. Pre-settle force simulation (VIZN-07)
    // Deep-copy nodes/edges so D3 can mutate them with x/y positions
    const nodes = data.nodes.map(d => Object.assign({}, d));
    const edges = data.edges.map(d => Object.assign({}, d));

    const simulation = d3.forceSimulation(nodes)
        .force('link', d3.forceLink(edges).id(d => d.id).distance(80))
        .force('charge', d3.forceManyBody().strength(-120))
        .force('center', d3.forceCenter(width / 2, height / 2).strength(0.05))
        .force('collide', d3.forceCollide().radius(d => d.radius + 20))
        .stop();

    // Run synchronously to settled state — no animation on load
    simulation.tick(300);

    // Helper: shorten line endpoint to node circumference so arrowhead is visible (VIZN-05)
    function adjustedEndpoint(source, target, targetRadius) {
        const dx = target.x - source.x;
        const dy = target.y - source.y;
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist === 0) return { x: target.x, y: target.y };
        return {
            x: target.x - (dx / dist) * (targetRadius + 8),
            y: target.y - (dy / dist) * (targetRadius + 8)
        };
    }

    // 8. Render edges (below nodes in DOM order)
    const linkGroup = g.append('g').attr('class', 'edges');
    const link = linkGroup.selectAll('line')
        .data(edges)
        .join('line')
        .attr('stroke', '#555555')
        .attr('stroke-opacity', 0.4)
        .attr('marker-end', 'url(#arrowhead)');

    // Set static positions from pre-settled simulation
    link.each(function(d) {
        const src = typeof d.source === 'object' ? d.source : nodes.find(n => n.id === d.source);
        const tgt = typeof d.target === 'object' ? d.target : nodes.find(n => n.id === d.target);
        if (!src || !tgt) return;
        const ep = adjustedEndpoint(src, tgt, tgt.radius);
        d3.select(this)
            .attr('x1', src.x)
            .attr('y1', src.y)
            .attr('x2', ep.x)
            .attr('y2', ep.y);
    });

    // 9. Render nodes (above edges in DOM order)
    const nodeGroup = g.append('g').attr('class', 'nodes');
    const node = nodeGroup.selectAll('circle')
        .data(nodes)
        .join('circle')
        .attr('cx', d => d.x)
        .attr('cy', d => d.y)
        .attr('r', d => d.radius)
        .attr('fill', '#4a9eff')
        .attr('stroke', '#2a2a4e')
        .attr('stroke-width', 1);

    // 10. Render labels below nodes
    const labelGroup = g.append('g').attr('class', 'labels');
    labelGroup.selectAll('text')
        .data(nodes)
        .join('text')
        .attr('x', d => d.x)
        .attr('y', d => d.y + d.radius + 6 + 11)
        .attr('text-anchor', 'middle')
        .attr('fill', '#ffffff')
        .attr('font-size', '11px')
        .attr('pointer-events', 'none')
        .text(d => d.filename);

    // 11. Hover tooltip (D-55)
    const tooltip = document.getElementById('tooltip');
    const tooltipPath = tooltip.querySelector('.tooltip-path');
    const tooltipExports = tooltip.querySelector('.tooltip-exports');
    const tooltipEdges = tooltip.querySelector('.tooltip-edges');

    node.on('mouseenter', function(event, d) {
        // Build export summary string
        const counts = d.export_counts;
        const kinds = [
            ['functions', counts.functions],
            ['classes', counts.classes],
            ['types', counts.types],
            ['interfaces', counts.interfaces],
            ['hooks', counts.hooks],
            ['enums', counts.enums]
        ];
        const parts = kinds.filter(([, n]) => n > 0).map(([k, n]) => n + ' ' + k);
        const exportSummary = parts.length > 0 ? parts.join(', ') : 'no exports';

        tooltipPath.textContent = d.path;
        tooltipExports.textContent = exportSummary;
        tooltipEdges.textContent = d.incoming + ' incoming • ' + d.outgoing + ' outgoing';

        tooltip.style.display = 'block';
        tooltip.style.left = (event.pageX + 12) + 'px';
        tooltip.style.top = event.pageY + 'px';
    });

    node.on('mousemove', function(event) {
        tooltip.style.left = (event.pageX + 12) + 'px';
        tooltip.style.top = event.pageY + 'px';
    });

    node.on('mouseleave', function() {
        tooltip.style.display = 'none';
    });

    // 12. Legend panel collapse/expand
    const legendHeader = document.querySelector('.legend-header');
    const legendContent = document.querySelector('.legend-content');
    let legendExpanded = true;

    legendHeader.addEventListener('click', function() {
        legendExpanded = !legendExpanded;
        legendContent.style.display = legendExpanded ? 'block' : 'none';
        // Update arrow character
        legendHeader.innerHTML = legendExpanded ? 'Legend &#9662;' : 'Legend &#9652;';
    });

    // 13. Directory halos (D-56) — toggle-activated, default OFF
    const halosToggle = document.getElementById('halos-toggle');
    let halosGroup = null;

    function renderHalos() {
        // Remove existing halos
        if (halosGroup) {
            halosGroup.remove();
            halosGroup = null;
        }

        if (!halosToggle.checked) return;

        // Group nodes by directory
        const dirMap = new Map();
        nodes.forEach(d => {
            const lastSlash = d.path.lastIndexOf('/');
            const dir = lastSlash >= 0 ? d.path.substring(0, lastSlash) : '';
            if (!dirMap.has(dir)) dirMap.set(dir, []);
            dirMap.get(dir).push(d);
        });

        // Insert halos group BEFORE edge group so halos render behind everything
        halosGroup = g.insert('g', '.edges').attr('class', 'halos');

        dirMap.forEach(function(groupNodes, dir) {
            if (groupNodes.length < 2) return;

            // Compute centroid for expansion
            const cx = groupNodes.reduce((sum, n) => sum + n.x, 0) / groupNodes.length;
            const cy = groupNodes.reduce((sum, n) => sum + n.y, 0) / groupNodes.length;

            // Build hull points from node positions
            const rawPoints = groupNodes.map(n => [n.x, n.y]);
            const hull = d3.polygonHull(rawPoints);
            if (!hull) return;

            // Expand hull outward from centroid by 30px padding
            const expanded = hull.map(function(pt) {
                const dx = pt[0] - cx;
                const dy = pt[1] - cy;
                const len = Math.sqrt(dx * dx + dy * dy);
                if (len === 0) return pt;
                return [
                    pt[0] + (dx / len) * 30,
                    pt[1] + (dy / len) * 30
                ];
            });

            const pathData = 'M' + expanded.map(p => p[0] + ',' + p[1]).join('L') + 'Z';

            halosGroup.append('path')
                .attr('d', pathData)
                .attr('stroke', '#4a9eff')
                .attr('stroke-opacity', 0.15)
                .attr('stroke-dasharray', '4 2')
                .attr('fill', '#4a9eff')
                .attr('fill-opacity', 0.05);
        });
    }

    halosToggle.addEventListener('change', renderHalos);

    // 14. Window resize handler — just resize SVG, do not re-run simulation
    window.addEventListener('resize', function() {
        svg.attr('width', window.innerWidth)
           .attr('height', window.innerHeight - 40);
    });
}
