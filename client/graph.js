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
        hdr.addEventListener('click', function() {
            var arrow = hdr.querySelector('.section-arrow');
            var body = hdr.nextElementSibling;
            arrow.classList.toggle('open');
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

    svg.call(d3.zoom().scaleExtent([0.1, 10]).on('zoom', function(event) {
        g.attr('transform', event.transform);
        currentZoom = event.transform.k;
        if (!hoverActive) updateLabelVisibility();
    }));

    var nodes = data.nodes.map(function(d) { return Object.assign({}, d); });
    var edges = data.edges.map(function(d) { return Object.assign({}, d); });
    var allNodes = nodes;
    var allEdges = edges;

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
    var node = nodeGroup.selectAll('circle').data(nodes).join('circle')
        .attr('r', function(d) { return d.radius; })
        .attr('fill', '#555').attr('stroke', 'none').style('cursor', 'grab');

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
            d.fx = null; d.fy = null;
            d3.select(this).style('cursor', 'grab');
        });
    node.call(drag);

    function updateLabelVisibility() {
        var showLabels = document.getElementById('toggle-labels').checked;
        labels.style('opacity', !showLabels ? 0 : currentZoom < 0.4 ? 0 : Math.min(1, (currentZoom - 0.3) * 3));
    }

    // Hover highlight
    var hoverActive = false;
    var FADE_IN = 250, FADE_OUT = 400;

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

        var counts = d.export_counts;
        var kinds = [['functions', counts.functions], ['classes', counts.classes],
            ['types', counts.types], ['interfaces', counts.interfaces],
            ['hooks', counts.hooks], ['enums', counts.enums]];
        var parts = kinds.filter(function(k) { return k[1] > 0; }).map(function(k) { return k[1] + ' ' + k[0]; });

        var tooltip = document.getElementById('tooltip');
        tooltip.querySelector('.tooltip-path').textContent = d.path;
        tooltip.querySelector('.tooltip-exports').textContent = parts.length > 0 ? parts.join(', ') : 'no exports';
        tooltip.querySelector('.tooltip-edges').textContent = d.incoming + ' incoming • ' + d.outgoing + ' outgoing';
        tooltip.style.display = 'block';
        tooltip.style.left = (event.pageX + 12) + 'px';
        tooltip.style.top = event.pageY + 'px';
    });

    node.on('mousemove', function(event) {
        var t = document.getElementById('tooltip');
        t.style.left = (event.pageX + 12) + 'px'; t.style.top = event.pageY + 'px';
    });

    node.on('mouseleave', function() {
        hoverActive = false;
        node.transition().duration(FADE_OUT).ease(d3.easeCubicIn).attr('fill', '#555').style('opacity', 1);
        labels.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
            .style('opacity', !document.getElementById('toggle-labels').checked ? 0 : currentZoom < 0.4 ? 0 : 1);
        link.transition().duration(FADE_OUT).ease(d3.easeCubicIn)
            .attr('stroke', '#444').attr('stroke-opacity', 0.25).attr('marker-end', 'url(#arrow)');
        document.getElementById('tooltip').style.display = 'none';
    });

    // === Panel controls ===

    // Filters: search
    document.getElementById('search-files').addEventListener('input', function(e) {
        var q = e.target.value.toLowerCase();
        node.style('opacity', function(d) { return !q || d.path.toLowerCase().includes(q) ? 1 : 0.1; });
        labels.style('opacity', function(d) { return !q || d.path.toLowerCase().includes(q) ? 1 : 0.05; });
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

    // Display: link thickness
    document.getElementById('slider-link-thickness').addEventListener('input', function() {
        link.attr('stroke-width', parseFloat(this.value));
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
