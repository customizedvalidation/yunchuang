(function () {
  var style = getComputedStyle(document.documentElement);
  var accent = style.getPropertyValue('--accent').trim();
  var accent2 = style.getPropertyValue('--accent2').trim();
  var ink = style.getPropertyValue('--ink').trim();
  var muted = style.getPropertyValue('--muted').trim();
  var rule = style.getPropertyValue('--rule').trim();
  var bg2 = style.getPropertyValue('--bg2').trim();

  // ---------- Chart 1: 力-位移曲线 (代表性, 需依据 SP5 socket datasheet 校准) ----------
  var el1 = document.getElementById('chart-force-profile');
  if (el1) {
    var c1 = echarts.init(el1, null, { renderer: 'svg' });

    var d = [];
    for (var i = 0; i <= 100; i += 2) d.push(i);

    // CPU ILM 闭合 (凸轮机构): 缓升→峰值→微回落
    var ilm = d.map(function (x) {
      if (x < 55) return 8 + x * 1.0;            // 缓升
      if (x < 82) return 63 + (x - 55) * 1.2;    // 陡升至峰值
      if (x < 92) return 95.4 - (x - 82) * 1.1;   // 峰后微回落
      return 84 - (x - 92) * 0.8;                // 锁紧到位
    });
    // DIMM 插入: 缓升→连接器咬合峰值→回落就位
    var ins = d.map(function (x) {
      if (x < 30) return 3 + x * 0.55;
      if (x < 72) return 19.5 + (x - 30) * 0.85;
      if (x < 84) return 55.2 - (x - 72) * 1.6;
      return 36 - (x - 84) * 0.5;
    });
    // DIMM 拔出: 起始松脱→破断峰值→缓降
    var ext = d.map(function (x) {
      if (x < 14) return 18 + x * 3.6;
      if (x < 24) return 68.4 + (x - 14) * 0.7;  // 破断峰值 ~75N
      if (x < 60) return 75.4 - (x - 24) * 1.25;
      return 30.4 - (x - 60) * 0.18;
    });

    c1.setOption({
      animation: false,
      color: [accent, accent, accent2],
      tooltip: { trigger: 'axis', appendToBody: true, valueFormatter: function (v) { return v.toFixed(1) + ' N'; } },
      legend: { top: 4, textStyle: { color: muted, fontSize: 12 }, icon: 'roundRect', itemWidth: 16, itemHeight: 8 },
      grid: { left: 52, right: 24, top: 44, bottom: 46 },
      xAxis: {
        type: 'value', name: '位移/行程 (%标么)', nameLocation: 'middle', nameGap: 30,
        nameTextStyle: { color: muted, fontSize: 12 },
        axisLine: { lineStyle: { color: rule } },
        axisLabel: { color: muted, fontSize: 11, formatter: '{value}' },
        splitLine: { lineStyle: { color: rule, type: 'dashed' } }
      },
      yAxis: {
        type: 'value', name: '力 (N)', nameLocation: 'middle', nameGap: 38,
        nameTextStyle: { color: muted, fontSize: 12 },
        axisLine: { lineStyle: { color: rule } },
        axisLabel: { color: muted, fontSize: 11 },
        splitLine: { lineStyle: { color: rule } },
        max: 100
      },
      series: [
        { name: 'CPU ILM 闭合', type: 'line', smooth: true, symbol: 'none', data: d.map(function (x, i) { return [x, ilm[i]]; }), lineStyle: { width: 2.6 }, areaStyle: { color: accent, opacity: 0.06 } },
        { name: 'DIMM 插入', type: 'line', smooth: true, symbol: 'none', data: d.map(function (x, i) { return [x, ins[i]]; }), lineStyle: { width: 2.6, type: 'solid' } },
        { name: 'DIMM 拔出', type: 'line', smooth: true, symbol: 'none', data: d.map(function (x, i) { return [x, ext[i]]; }), lineStyle: { width: 2.6, type: 'dashed' } }
      ]
    });
    window.addEventListener('resize', function () { c1.resize(); });
  }

  // ---------- Chart 2: 测试覆盖矩阵 (heatmap) ----------
  var el2 = document.getElementById('chart-coverage');
  if (el2) {
    var c2 = echarts.init(el2, null, { renderer: 'svg' });
    var groups = ['CPU 取料/对位', 'CPU ILM 闭合动作', 'CPU 拆卸/取回', 'DIMM 取料/对位', 'DIMM 插入', 'DIMM 拔出', '电气与功能自检', '耐久/应力监测', '安全/边界与故障'];
    var layers = ['功能Function', '力学Force', '视觉Vision', '电气Electrical', '耐久Endurance', '安全Safety'];
    // 0=不适用, 1=辅助, 2=覆盖, 3=核心
    var raw = [
      [2, 3, 3, 0, 1, 3],
      [2, 3, 1, 0, 2, 3],
      [2, 3, 1, 0, 2, 3],
      [2, 2, 3, 0, 1, 3],
      [2, 3, 2, 1, 2, 3],
      [2, 3, 1, 1, 2, 3],
      [1, 0, 0, 3, 1, 2],
      [1, 3, 1, 2, 3, 2],
      [2, 2, 2, 1, 1, 3]
    ];
    var data = [];
    groups.forEach(function (_, gi) {
      layers.forEach(function (_, li) {
        data.push([li, gi, raw[gi][li]]);
      });
    });
    c2.setOption({
      animation: false,
      tooltip: { appendToBody: true, formatter: function (p) { return '<b>' + groups[p.value[1]] + '</b> × ' + layers[p.value[0]] + '<br/>覆盖度: ' + ['N/A', '辅助', '覆盖', '核心'][p.value[2]]; } },
      grid: { left: 132, right: 30, top: 30, bottom: 56 },
      xAxis: {
        type: 'category', data: layers, splitArea: { show: false },
        axisLine: { lineStyle: { color: rule } }, axisLabel: { color: muted, fontSize: 11 }
      },
      yAxis: {
        type: 'category', data: groups, splitArea: { show: false },
        axisLine: { lineStyle: { color: rule } }, axisLabel: { color: muted, fontSize: 11 }
      },
      visualMap: {
        min: 0, max: 3, show: true, orient: 'horizontal', left: 'center', bottom: 4,
        itemWidth: 14, itemHeight: 110,
        textStyle: { color: muted, fontSize: 11 },
        pieces: [
          { value: 0, label: 'N/A', color: bg2 },
          { value: 1, label: '辅助', color: accent + '33' },
          { value: 2, label: '覆盖', color: accent + '99' },
          { value: 3, label: '核心', color: accent }
        ]
      },
      series: [{
        type: 'heatmap', data: data, label: { show: true, color: ink, fontSize: 11, formatter: function (p) { return p.value[2] === 0 ? '·' : ['·', '○', '●', '★'][p.value[2]]; } },
        itemStyle: { borderColor: bg2, borderWidth: 2 }
      }]
    });
    window.addEventListener('resize', function () { c2.resize(); });
  }

  // ---------- Chart 3: 各类测试用例分布 (横向 bar) ----------
  var el3 = document.getElementById('chart-cases');
  if (el3) {
    var c3 = echarts.init(el3, null, { renderer: 'svg' });
    var cats = ['CPU 正常拆装', 'CPU 异常/边界', 'DIMM 正常插拔', 'DIMM 异常/边界', '电气/功能验证', '耐久/应力', '安全/互锁'];
    var counts = [14, 9, 12, 8, 10, 6, 7];
    c3.setOption({
      animation: false,
      tooltip: { trigger: 'axis', appendToBody: true, axisPointer: { type: 'shadow' } },
      grid: { left: 120, right: 40, top: 20, bottom: 30 },
      xAxis: {
        type: 'value', name: '用例数',
        nameTextStyle: { color: muted, fontSize: 12 }, nameLocation: 'middle', nameGap: 28,
        axisLine: { lineStyle: { color: rule } }, axisLabel: { color: muted, fontSize: 11 },
        splitLine: { lineStyle: { color: rule } }
      },
      yAxis: {
        type: 'category', data: cats,
        axisLine: { lineStyle: { color: rule } }, axisLabel: { color: ink, fontSize: 11 }
      },
      series: [{
        type: 'bar', data: counts, barWidth: 16,
        itemStyle: { color: accent, borderRadius: [0, 3, 3, 0] },
        label: { show: true, position: 'right', color: ink, fontSize: 11 }
      }]
    });
    window.addEventListener('resize', function () { c3.resize(); });
  }

  // ---------- Chart 4: 耐久循环峰值力漂移监测 (line) ----------
  var el4 = document.getElementById('chart-drift');
  if (el4) {
    var c4 = echarts.init(el4, null, { renderer: 'svg' });
    var cycles = [];
    for (var k = 0; k <= 1000; k += 50) cycles.push(k);
    // ILM 峰值力随循环微升 (磨损/接触变化) + 阈值上下界
    var ilmPeak = cycles.map(function (x) { return 95.0 + (x / 1000) * 3.2 + Math.sin(x / 90) * 0.6; });
    var dimmPeak = cycles.map(function (x) { return 55.0 + (x / 1000) * 4.5 + Math.sin(x / 70) * 0.7; });
    c4.setOption({
      animation: false,
      color: [accent, accent2],
      tooltip: { trigger: 'axis', appendToBody: true, valueFormatter: function (v) { return v.toFixed(2) + ' N'; } },
      legend: { top: 4, textStyle: { color: muted, fontSize: 12 }, icon: 'roundRect', itemWidth: 16, itemHeight: 8 },
      grid: { left: 52, right: 24, top: 44, bottom: 46 },
      xAxis: {
        type: 'category', data: cycles, name: '循环次数 (次)', nameLocation: 'middle', nameGap: 30,
        nameTextStyle: { color: muted, fontSize: 12 }, boundaryGap: false,
        axisLine: { lineStyle: { color: rule } }, axisLabel: { color: muted, fontSize: 11 },
        splitLine: { show: false }
      },
      yAxis: {
        type: 'value', name: '峰值力 (N)', nameLocation: 'middle', nameGap: 38,
        nameTextStyle: { color: muted, fontSize: 12 },
        axisLine: { lineStyle: { color: rule } }, axisLabel: { color: muted, fontSize: 11 },
        splitLine: { lineStyle: { color: rule } }, min: 40, max: 110
      },
      series: [
        {
          name: 'CPU ILM 峰值力', type: 'line', smooth: true, symbol: 'none',
          data: ilmPeak, lineStyle: { width: 2.4 }, markLine: {
            symbol: 'none', silent: true, label: { color: accent2, fontSize: 10, position: 'insideEndTop' },
            lineStyle: { color: accent2, type: 'dashed', width: 1.2 },
            data: [{ yAxis: 108, name: '上限 108 N' }]
          }
        },
        {
          name: 'DIMM 插入峰值力', type: 'line', smooth: true, symbol: 'none',
          data: dimmPeak, lineStyle: { width: 2.4 }, markLine: {
            symbol: 'none', silent: true, label: { color: accent2, fontSize: 10, position: 'insideEndBottom' },
            lineStyle: { color: accent2, type: 'dashed', width: 1.2 },
            data: [{ yAxis: 62, name: '上限 62 N' }]
          }
        }
      ]
    });
    window.addEventListener('resize', function () { c4.resize(); });
  }
})();
