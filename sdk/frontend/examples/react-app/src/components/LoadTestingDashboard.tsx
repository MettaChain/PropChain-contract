import React, { useState, useEffect } from 'react';

interface LoadTestMetrics {
  timestamp: string;
  testName: string;
  concurrentUsers: number;
  durationSecs: number;
  totalOperations: number;
  successfulOperations: number;
  failedOperations: number;
  avgLatencyMs: number;
  p95LatencyMs: number;
  p99LatencyMs: number;
  opsPerSecond: number;
  errorRate: number;
}

interface TrendData {
  dates: string[];
  opsPerSecond: number[];
  avgLatency: number[];
  errorRate: number[];
  successRate: number[];
}

export function LoadTestingDashboard() {
  const [selectedTimeRange, setSelectedTimeRange] = useState<'7d' | '30d' | '90d'>('30d');
  const [selectedTest, setSelectedTest] = useState<string>('all');
  const [trendData, setTrendData] = useState<TrendData | null>(null);
  const [recentTests, setRecentTests] = useState<LoadTestMetrics[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const mockTrendData: TrendData = {
      dates: generateDates(selectedTimeRange),
      opsPerSecond: [45, 52, 48, 61, 58, 72, 68, 75, 82, 79, 85, 88, 91, 89, 94, 96, 92, 98, 101, 99, 103, 105, 102, 108, 110, 107, 112, 115, 113, 118],
      avgLatency: [120, 115, 125, 110, 108, 95, 100, 92, 88, 90, 85, 82, 78, 80, 75, 73, 77, 70, 68, 72, 65, 63, 67, 60, 58, 62, 55, 53, 57, 50],
      errorRate: [2.5, 2.1, 2.8, 1.9, 1.7, 1.5, 1.8, 1.3, 1.1, 1.4, 1.0, 0.9, 0.7, 0.8, 0.6, 0.5, 0.7, 0.4, 0.3, 0.5, 0.3, 0.2, 0.4, 0.2, 0.1, 0.3, 0.1, 0.1, 0.2, 0.0],
      successRate: [97.5, 97.9, 97.2, 98.1, 98.3, 98.5, 98.2, 98.7, 98.9, 98.6, 99.0, 99.1, 99.3, 99.2, 99.4, 99.5, 99.3, 99.6, 99.7, 99.5, 99.7, 99.8, 99.6, 99.8, 99.9, 99.7, 99.9, 99.9, 99.8, 100.0],
    };

    const mockRecentTests: LoadTestMetrics[] = [
      {
        timestamp: '2026-04-22T02:00:00Z',
        testName: 'Concurrent Registration (Heavy)',
        concurrentUsers: 50,
        durationSecs: 300,
        totalOperations: 15420,
        successfulOperations: 15418,
        failedOperations: 2,
        avgLatencyMs: 50,
        p95LatencyMs: 120,
        p99LatencyMs: 180,
        opsPerSecond: 118,
        errorRate: 0.01,
      },
      {
        timestamp: '2026-04-21T02:00:00Z',
        testName: 'Property Transfer Load',
        concurrentUsers: 30,
        durationSecs: 180,
        totalOperations: 8920,
        successfulOperations: 8915,
        failedOperations: 5,
        avgLatencyMs: 55,
        p95LatencyMs: 125,
        p99LatencyMs: 190,
        opsPerSecond: 113,
        errorRate: 0.06,
      },
      {
        timestamp: '2026-04-20T02:00:00Z',
        testName: 'Endurance Test (4 hours)',
        concurrentUsers: 20,
        durationSecs: 14400,
        totalOperations: 458920,
        successfulOperations: 458850,
        failedOperations: 70,
        avgLatencyMs: 48,
        p95LatencyMs: 110,
        p99LatencyMs: 165,
        opsPerSecond: 107,
        errorRate: 0.02,
      },
      {
        timestamp: '2026-04-19T02:00:00Z',
        testName: 'Stress Test (100 users)',
        concurrentUsers: 100,
        durationSecs: 600,
        totalOperations: 61200,
        successfulOperations: 61080,
        failedOperations: 120,
        avgLatencyMs: 95,
        p95LatencyMs: 220,
        p99LatencyMs: 380,
        opsPerSecond: 102,
        errorRate: 0.2,
      },
    ];

    setTimeout(() => {
      setTrendData(mockTrendData);
      setRecentTests(mockRecentTests);
      setLoading(false);
    }, 500);
  }, [selectedTimeRange, selectedTest]);

  function generateDates(range: '7d' | '30d' | '90d'): string[] {
    const count = range === '7d' ? 7 : range === '30d' ? 30 : 90;
    const dates: string[] = [];
    for (let i = count - 1; i >= 0; i--) {
      const date = new Date();
      date.setDate(date.getDate() - i);
      dates.push(date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' }));
    }
    return dates;
  }

  const formatNumber = (num: number, decimals = 0) => num.toFixed(decimals);

  const getStatusColor = (errorRate: number) => {
    if (errorRate < 0.1) return '#10b981';
    if (errorRate < 1.0) return '#f59e0b';
    return '#ef4444';
  };

  if (loading) {
    return (
      <div className="dashboard-container">
        <div className="loading-state">
          <div className="spinner"></div>
          <p>Loading load test data...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="dashboard-container">
      <div className="dashboard-header">
        <h2>Load Testing Dashboard</h2>
        <p>CI/CD Performance Metrics & Trends</p>
      </div>

      <div className="dashboard-controls">
        <div className="control-group">
          <label>Time Range</label>
          <select
            value={selectedTimeRange}
            onChange={(e) => setSelectedTimeRange(e.target.value as '7d' | '30d' | '90d')}
            className="select-input"
          >
            <option value="7d">Last 7 Days</option>
            <option value="30d">Last 30 Days</option>
            <option value="90d">Last 90 Days</option>
          </select>
        </div>
        <div className="control-group">
          <label>Test Type</label>
          <select
            value={selectedTest}
            onChange={(e) => setSelectedTest(e.target.value)}
            className="select-input"
          >
            <option value="all">All Tests</option>
            <option value="registration">Registration</option>
            <option value="transfer">Transfer</option>
            <option value="endurance">Endurance</option>
            <option value="stress">Stress</option>
          </select>
        </div>
        <button className="refresh-btn" onClick={() => setLoading(true)}>
          Refresh Data
        </button>
      </div>

      <div className="metrics-grid">
        <div className="metric-card">
          <div className="metric-header">
            <span className="metric-icon">⚡</span>
            <span className="metric-label">Avg Ops/Sec</span>
          </div>
          <div className="metric-value">
            {formatNumber(trendData!.opsPerSecond[trendData!.opsPerSecond.length - 1])}
          </div>
          <div className="metric-change positive">
            +{formatNumber((trendData!.opsPerSecond[trendData!.opsPerSecond.length - 1] - trendData!.opsPerSecond[0]) / trendData!.opsPerSecond[0] * 100, 1)}%
          </div>
        </div>

        <div className="metric-card">
          <div className="metric-header">
            <span className="metric-icon">⏱️</span>
            <span className="metric-label">Avg Latency</span>
          </div>
          <div className="metric-value">
            {formatNumber(trendData!.avgLatency[trendData!.avgLatency.length - 1])}ms
          </div>
          <div className="metric-change positive">
            -{formatNumber((trendData!.avgLatency[0] - trendData!.avgLatency[trendData!.avgLatency.length - 1]) / trendData!.avgLatency[0] * 100, 1)}%
          </div>
        </div>

        <div className="metric-card">
          <div className="metric-header">
            <span className="metric-icon">✅</span>
            <span className="metric-label">Success Rate</span>
          </div>
          <div className="metric-value">
            {formatNumber(trendData!.successRate[trendData!.successRate.length - 1], 2)}%
          </div>
          <div className="metric-change positive">
            +{formatNumber(trendData!.successRate[trendData!.successRate.length - 1] - trendData!.successRate[0], 2)}%
          </div>
        </div>

        <div className="metric-card">
          <div className="metric-header">
            <span className="metric-icon">❌</span>
            <span className="metric-label">Error Rate</span>
          </div>
          <div className="metric-value" style={{ color: getStatusColor(trendData!.errorRate[trendData!.errorRate.length - 1]) }}>
            {formatNumber(trendData!.errorRate[trendData!.errorRate.length - 1], 2)}%
          </div>
          <div className="metric-change positive">
            -{formatNumber((trendData!.errorRate[0] - trendData!.errorRate[trendData!.errorRate.length - 1]) / (trendData!.errorRate[0] || 1) * 100, 1)}%
          </div>
        </div>
      </div>

      <div className="charts-section">
        <div className="chart-container">
          <h3>Throughput Trend (Ops/Sec)</h3>
          <SimpleLineChart data={trendData!.opsPerSecond} labels={trendData!.dates} color="#3b82f6" />
        </div>

        <div className="chart-container">
          <h3>Latency Trend (ms)</h3>
          <SimpleLineChart data={trendData!.avgLatency} labels={trendData!.dates} color="#8b5cf6" />
        </div>

        <div className="chart-container">
          <h3>Error Rate Trend (%)</h3>
          <SimpleLineChart data={trendData!.errorRate} labels={trendData!.dates} color="#ef4444" />
        </div>

        <div className="chart-container">
          <h3>Success Rate Trend (%)</h3>
          <SimpleLineChart data={trendData!.successRate} labels={trendData!.dates} color="#10b981" />
        </div>
      </div>

      <div className="table-section">
        <h3>Recent Test Runs</h3>
        <div className="table-container">
          <table className="test-table">
            <thead>
              <tr>
                <th>Date</th>
                <th>Test Name</th>
                <th>Users</th>
                <th>Duration</th>
                <th>Ops/Sec</th>
                <th>Avg Latency</th>
                <th>P95</th>
                <th>P99</th>
                <th>Error Rate</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              {recentTests.map((test, index) => (
                <tr key={index}>
                  <td>{new Date(test.timestamp).toLocaleDateString()}</td>
                  <td>{test.testName}</td>
                  <td>{test.concurrentUsers}</td>
                  <td>{Math.round(test.durationSecs / 60)}m</td>
                  <td>{test.opsPerSecond}</td>
                  <td>{test.avgLatencyMs}ms</td>
                  <td>{test.p95LatencyMs}ms</td>
                  <td>{test.p99LatencyMs}ms</td>
                  <td style={{ color: getStatusColor(test.errorRate) }}>
                    {test.errorRate.toFixed(2)}%
                  </td>
                  <td>
                    <span
                      className="status-badge"
                      style={{ backgroundColor: getStatusColor(test.errorRate) }}
                    >
                      {test.errorRate < 0.1 ? 'PASS' : test.errorRate < 1.0 ? 'WARN' : 'FAIL'}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      <div className="export-section">
        <h3>Export Data</h3>
        <div className="export-buttons">
          <button className="export-btn">Download CSV</button>
          <button className="export-btn">Download JSON</button>
          <button className="export-btn">Generate Report</button>
        </div>
      </div>
    </div>
  );
}

function SimpleLineChart({ data, labels, color }: { data: number[]; labels: string[]; color: string }) {
  const max = Math.max(...data);
  const min = Math.min(...data);
  const range = max - min || 1;

  const points = data.map((value, index) => {
    const x = (index / (data.length - 1)) * 100;
    const y = 100 - ((value - min) / range) * 80 - 10;
    return `${x},${y}`;
  }).join(' ');

  return (
    <div className="chart-wrapper">
      <svg viewBox="0 0 100 100" preserveAspectRatio="none" className="line-chart">
        {[0, 25, 50, 75, 100].map((y) => (
          <line key={y} x1="0" y1={y} x2="100" y2={y} stroke="#e5e7eb" strokeWidth="0.5" />
        ))}
        <polygon
          points={`0,100 ${points} 100,100`}
          fill={color}
          fillOpacity="0.1"
        />
        <polyline
          points={points}
          fill="none"
          stroke={color}
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
        {data.map((value, index) => {
          const x = (index / (data.length - 1)) * 100;
          const y = 100 - ((value - min) / range) * 80 - 10;
          return (
            <circle
              key={index}
              cx={x}
              cy={y}
              r="1.5"
              fill={color}
              stroke="white"
              strokeWidth="0.5"
            />
          );
        })}
      </svg>
      <div className="chart-labels">
        {labels.filter((_, i) => i % Math.ceil(labels.length / 6) === 0 || i === labels.length - 1).map((label, index) => (
          <span key={index} className="chart-label">{label}</span>
        ))}
      </div>
    </div>
  );
}