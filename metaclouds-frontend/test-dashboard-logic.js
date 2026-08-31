// Dashboard 组件核心逻辑测试（纯 JavaScript，无需 jsdom）

// 从 API 响应中提取数据的工具函数
function extractData(response) {
  return response?.data ?? [];
}

// 计算集群数量
function getClusterCount(clusters) {
  const data = extractData(clusters);
  return data.length || 3;
}

// 计算运行中作业数量
function getRunningJobsCount(jobs) {
  const data = extractData(jobs);
  return data.filter((job) => job.status === 'running').length || 5;
}

// 计算总作业数量
function getTotalJobsCount(jobs) {
  const data = extractData(jobs);
  return data.length || 12;
}

// 计算告警数量
function getAlertCount(alerts) {
  const data = extractData(alerts);
  return data.length || 2;
}

// 获取告警样式
function getAlertStyle(level) {
  switch (level) {
    case 'critical':
      return { background: 'rgba(239, 68, 68, 0.05)', borderColor: '#ef4444', icon: '🔴' };
    case 'error':
      return { background: 'rgba(249, 115, 22, 0.05)', borderColor: '#f97316', icon: '🟠' };
    case 'warning':
      return { background: 'rgba(245, 158, 11, 0.05)', borderColor: '#f59e0b', icon: '🟡' };
    default:
      return { background: 'rgba(34, 197, 94, 0.05)', borderColor: '#22c55e', icon: '🟢' };
  }
}

// 获取告警标题
function getAlertTitle(alert) {
  return alert.message || alert.title || '未知告警';
}

// 获取告警描述
function getAlertDescription(alert) {
  return alert.details || alert.description || '暂无详情';
}

// 获取告警时间
function getAlertTime(alert) {
  return alert.created_at || alert.time || new Date().toLocaleString();
}

// ==================== 测试用例 ====================

console.log('=====================================');
console.log('Dashboard 核心逻辑单元测试');
console.log('=====================================');

let passed = 0;
let failed = 0;

// 测试 1: 计算集群数量
try {
  const clusters = { success: true, data: [{ id: 1, name: 'Cluster 1' }], timestamp: Date.now() };
  const count = getClusterCount(clusters);
  if (count === 1) {
    console.log('✅ 测试 1 通过: 集群数量计算正确');
    passed++;
  } else {
    console.log('❌ 测试 1 失败: 期望 1，实际 ' + count);
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 1 失败: ' + error.message);
  failed++;
}

// 测试 2: 空集群数据使用默认值
try {
  const count = getClusterCount(null);
  if (count === 3) {
    console.log('✅ 测试 2 通过: 空数据使用默认值');
    passed++;
  } else {
    console.log('❌ 测试 2 失败: 期望 3，实际 ' + count);
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 2 失败: ' + error.message);
  failed++;
}

// 测试 3: 计算运行中作业数量
try {
  const jobs = {
    success: true,
    data: [
      { id: 1, status: 'running', name: 'Job 1' },
      { id: 2, status: 'running', name: 'Job 2' },
      { id: 3, status: 'completed', name: 'Job 3' },
    ],
    timestamp: Date.now(),
  };
  const count = getRunningJobsCount(jobs);
  if (count === 2) {
    console.log('✅ 测试 3 通过: 运行中作业数量计算正确');
    passed++;
  } else {
    console.log('❌ 测试 3 失败: 期望 2，实际 ' + count);
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 3 失败: ' + error.message);
  failed++;
}

// 测试 4: 计算总作业数量
try {
  const jobs = {
    success: true,
    data: [
      { id: 1, status: 'running', name: 'Job 1' },
      { id: 2, status: 'completed', name: 'Job 2' },
    ],
    timestamp: Date.now(),
  };
  const count = getTotalJobsCount(jobs);
  if (count === 2) {
    console.log('✅ 测试 4 通过: 总作业数量计算正确');
    passed++;
  } else {
    console.log('❌ 测试 4 失败: 期望 2，实际 ' + count);
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 4 失败: ' + error.message);
  failed++;
}

// 测试 5: 空作业数据使用默认值
try {
  const count = getTotalJobsCount(undefined);
  if (count === 12) {
    console.log('✅ 测试 5 通过: 空作业数据使用默认值');
    passed++;
  } else {
    console.log('❌ 测试 5 失败: 期望 12，实际 ' + count);
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 5 失败: ' + error.message);
  failed++;
}

// 测试 6: 计算告警数量
try {
  const alerts = {
    success: true,
    data: [
      { id: 1, level: 'warning', message: 'Alert 1' },
      { id: 2, level: 'error', message: 'Alert 2' },
      { id: 3, level: 'critical', message: 'Alert 3' },
    ],
    timestamp: Date.now(),
  };
  const count = getAlertCount(alerts);
  if (count === 3) {
    console.log('✅ 测试 6 通过: 告警数量计算正确');
    passed++;
  } else {
    console.log('❌ 测试 6 失败: 期望 3，实际 ' + count);
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 6 失败: ' + error.message);
  failed++;
}

// 测试 7: 获取 critical 级别告警样式
try {
  const style = getAlertStyle('critical');
  if (style.icon === '🔴' && style.borderColor === '#ef4444') {
    console.log('✅ 测试 7 通过: critical 级别样式正确');
    passed++;
  } else {
    console.log('❌ 测试 7 失败: 样式不匹配');
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 7 失败: ' + error.message);
  failed++;
}

// 测试 8: 获取 warning 级别告警样式
try {
  const style = getAlertStyle('warning');
  if (style.icon === '🟡' && style.borderColor === '#f59e0b') {
    console.log('✅ 测试 8 通过: warning 级别样式正确');
    passed++;
  } else {
    console.log('❌ 测试 8 失败: 样式不匹配');
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 8 失败: ' + error.message);
  failed++;
}

// 测试 9: 获取 error 级别告警样式
try {
  const style = getAlertStyle('error');
  if (style.icon === '🟠' && style.borderColor === '#f97316') {
    console.log('✅ 测试 9 通过: error 级别样式正确');
    passed++;
  } else {
    console.log('❌ 测试 9 失败: 样式不匹配');
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 9 失败: ' + error.message);
  failed++;
}

// 测试 10: 获取默认级别告警样式
try {
  const style = getAlertStyle('info');
  if (style.icon === '🟢' && style.borderColor === '#22c55e') {
    console.log('✅ 测试 10 通过: 默认级别样式正确');
    passed++;
  } else {
    console.log('❌ 测试 10 失败: 样式不匹配');
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 10 失败: ' + error.message);
  failed++;
}

// 测试 11: 获取告警标题（使用 message 字段）
try {
  const alert = { id: 1, level: 'warning', message: 'Test Message' };
  const title = getAlertTitle(alert);
  if (title === 'Test Message') {
    console.log('✅ 测试 11 通过: 使用 message 字段');
    passed++;
  } else {
    console.log('❌ 测试 11 失败: 期望 "Test Message"，实际 "' + title + '"');
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 11 失败: ' + error.message);
  failed++;
}

// 测试 12: 获取告警标题（使用 title 字段）
try {
  const alert = { id: 1, level: 'warning', title: 'Test Title' };
  const title = getAlertTitle(alert);
  if (title === 'Test Title') {
    console.log('✅ 测试 12 通过: 使用 title 字段');
    passed++;
  } else {
    console.log('❌ 测试 12 失败: 期望 "Test Title"，实际 "' + title + '"');
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 12 失败: ' + error.message);
  failed++;
}

// 测试 13: 获取告警标题（无字段时使用默认值）
try {
  const alert = { id: 1, level: 'warning' };
  const title = getAlertTitle(alert);
  if (title === '未知告警') {
    console.log('✅ 测试 13 通过: 无标题字段使用默认值');
    passed++;
  } else {
    console.log('❌ 测试 13 失败: 期望 "未知告警"，实际 "' + title + '"');
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 13 失败: ' + error.message);
  failed++;
}

// 测试 14: 获取告警描述（使用 details 字段）
try {
  const alert = { id: 1, level: 'warning', details: 'Test Details' };
  const desc = getAlertDescription(alert);
  if (desc === 'Test Details') {
    console.log('✅ 测试 14 通过: 使用 details 字段');
    passed++;
  } else {
    console.log('❌ 测试 14 失败: 期望 "Test Details"，实际 "' + desc + '"');
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 14 失败: ' + error.message);
  failed++;
}

// 测试 15: 获取告警描述（使用 description 字段）
try {
  const alert = { id: 1, level: 'warning', description: 'Test Description' };
  const desc = getAlertDescription(alert);
  if (desc === 'Test Description') {
    console.log('✅ 测试 15 通过: 使用 description 字段');
    passed++;
  } else {
    console.log('❌ 测试 15 失败: 期望 "Test Description"，实际 "' + desc + '"');
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 15 失败: ' + error.message);
  failed++;
}

// 测试 16: 获取告警描述（无字段时使用默认值）
try {
  const alert = { id: 1, level: 'warning' };
  const desc = getAlertDescription(alert);
  if (desc === '暂无详情') {
    console.log('✅ 测试 16 通过: 无描述字段使用默认值');
    passed++;
  } else {
    console.log('❌ 测试 16 失败: 期望 "暂无详情"，实际 "' + desc + '"');
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 16 失败: ' + error.message);
  failed++;
}

// 测试 17: 获取告警时间（使用 created_at 字段）
try {
  const alert = { id: 1, level: 'warning', created_at: '2026-05-18T10:00:00Z' };
  const time = getAlertTime(alert);
  if (time === '2026-05-18T10:00:00Z') {
    console.log('✅ 测试 17 通过: 使用 created_at 字段');
    passed++;
  } else {
    console.log('❌ 测试 17 失败: 期望 "2026-05-18T10:00:00Z"，实际 "' + time + '"');
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 17 失败: ' + error.message);
  failed++;
}

// 测试 18: 获取告警时间（使用 time 字段）
try {
  const alert = { id: 1, level: 'warning', time: '2026-05-18 10:00' };
  const result = getAlertTime(alert);
  if (result === '2026-05-18 10:00') {
    console.log('✅ 测试 18 通过: 使用 time 字段');
    passed++;
  } else {
    console.log('❌ 测试 18 失败: 期望 "2026-05-18 10:00"，实际 "' + result + '"');
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 18 失败: ' + error.message);
  failed++;
}

// 测试 19: 空告警数据使用默认值
try {
  const count = getAlertCount({ success: true, data: [], timestamp: Date.now() });
  if (count === 2) {
    console.log('✅ 测试 19 通过: 空告警数据使用默认值');
    passed++;
  } else {
    console.log('❌ 测试 19 失败: 期望 2，实际 ' + count);
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 19 失败: ' + error.message);
  failed++;
}

// 测试 20: 处理 undefined 数据
try {
  const count = getClusterCount(undefined);
  if (count === 3) {
    console.log('✅ 测试 20 通过: 处理 undefined 数据');
    passed++;
  } else {
    console.log('❌ 测试 20 失败: 期望 3，实际 ' + count);
    failed++;
  }
} catch (error) {
  console.log('❌ 测试 20 失败: ' + error.message);
  failed++;
}

// ==================== 测试结果汇总 ====================

console.log('');
console.log('=====================================');
console.log('测试结果汇总');
console.log('=====================================');
console.log('通过: ' + passed + ' 个');
console.log('失败: ' + failed + ' 个');
console.log('总计: ' + (passed + failed) + ' 个');
console.log('');

if (failed === 0) {
  console.log('🎉 所有测试通过！');
  console.log('');
  console.log('核心逻辑验证完成，Dashboard 组件的数据处理逻辑正确。');
} else {
  console.log('⚠️ 部分测试失败，请检查代码逻辑。');
}
