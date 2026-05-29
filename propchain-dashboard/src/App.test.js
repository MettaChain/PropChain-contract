import { render, screen } from '@testing-library/react';

jest.mock('recharts', () => ({
  Area: () => null,
  AreaChart: ({ children }) => <div>{children}</div>,
  Bar: () => null,
  BarChart: ({ children }) => <div>{children}</div>,
  CartesianGrid: () => null,
  Cell: () => null,
  Pie: ({ children }) => <div>{children}</div>,
  PieChart: ({ children }) => <div>{children}</div>,
  ResponsiveContainer: ({ children }) => <div>{children}</div>,
  Tooltip: () => null,
  XAxis: () => null,
  YAxis: () => null,
}));

jest.mock('./StellarClient', () => ({
  fetchContractStats: () => Promise.resolve(null),
  fetchInsuranceAnalytics: () =>
    Promise.resolve({
      totalPolicies: 10,
      activePolicies: 8,
      totalPremiumsCollected: 1000,
      totalClaimsPaid: 250,
      totalClaims: 4,
      approvedClaims: 3,
      openClaims: 1,
      coverageExposure: 5000,
      availableCapital: 3000,
      averageClaimSeverity: 83,
      monthlyTrend: [{ month: 'Jan', premiums: 1000, claims: 250 }],
      claimStatusBreakdown: [
        { status: 'Approved', count: 3 },
        { status: 'Under Review', count: 1 },
        { status: 'Pending', count: 0 },
        { status: 'Rejected', count: 0 },
      ],
      poolUtilization: [{ pool: 'Fire', utilization: 42 }],
    }),
}));

const App = require('./App').default;

test('renders insurance analytics dashboard', async () => {
  render(<App />);
  expect(screen.getByText(/PropChain Analytics/i)).toBeInTheDocument();
  expect(await screen.findByText(/Insurance Analytics Dashboard/i)).toBeInTheDocument();
  expect(await screen.findByText(/Claim Ratio/i)).toBeInTheDocument();
  expect(screen.getByText(/Premiums vs Claims/i)).toBeInTheDocument();
});
