import React, { useEffect, useMemo, useState } from 'react';
import {
    Activity,
    CircleDollarSign,
    FileWarning,
    Percent,
    ShieldCheck,
    TrendingUp,
} from 'lucide-react';
import {
    Area,
    AreaChart,
    Bar,
    BarChart,
    CartesianGrid,
    Cell,
    Pie,
    PieChart,
    ResponsiveContainer,
    Tooltip,
    XAxis,
    YAxis,
} from 'recharts';
import { fetchInsuranceAnalytics } from './StellarClient';

const currencyFormatter = new Intl.NumberFormat('en-US', {
    maximumFractionDigits: 0,
});

const percentFormatter = new Intl.NumberFormat('en-US', {
    maximumFractionDigits: 1,
});

const statusColors = {
    Approved: '#22c55e',
    'Under Review': '#38bdf8',
    Pending: '#f59e0b',
    Rejected: '#ef4444',
};

const getClaimRatio = (claimsPaid, premiumsCollected) => {
    if (!premiumsCollected) {
        return 0;
    }

    return (claimsPaid / premiumsCollected) * 100;
};

const MetricCard = ({ icon: Icon, label, value, detail, tone }) => (
    <div className={`rounded-lg border bg-slate-900/80 p-5 ${tone}`}>
        <div className="mb-4 flex items-center justify-between gap-3">
            <Icon className="h-5 w-5" aria-hidden="true" />
            <span className="text-xs uppercase tracking-wide text-slate-400">{detail}</span>
        </div>
        <p className="text-sm text-slate-400">{label}</p>
        <p className="mt-2 text-2xl font-semibold text-white">{value}</p>
    </div>
);

const InsuranceAnalyticsDashboard = () => {
    const [analytics, setAnalytics] = useState(null);
    const [lastUpdated, setLastUpdated] = useState(null);

    useEffect(() => {
        const loadAnalytics = () => {
            fetchInsuranceAnalytics().then((result) => {
                setAnalytics(result);
                setLastUpdated(new Date());
            });
        };

        loadAnalytics();
        const interval = setInterval(loadAnalytics, 30000);

        return () => clearInterval(interval);
    }, []);

    const summary = useMemo(() => {
        if (!analytics) {
            return null;
        }

        const claimRatio = getClaimRatio(
            analytics.totalClaimsPaid,
            analytics.totalPremiumsCollected
        );
        const approvalRate = analytics.totalClaims
            ? (analytics.approvedClaims / analytics.totalClaims) * 100
            : 0;
        const reserveRatio = analytics.coverageExposure
            ? (analytics.availableCapital / analytics.coverageExposure) * 100
            : 0;

        return {
            claimRatio,
            approvalRate,
            reserveRatio,
        };
    }, [analytics]);

    if (!analytics || !summary) {
        return (
            <section className="rounded-lg border border-slate-700 bg-slate-900 p-6">
                <p className="text-sm text-slate-400">Loading insurance analytics...</p>
            </section>
        );
    }

    return (
        <section className="space-y-6" aria-labelledby="insurance-analytics-title">
            <div className="flex flex-col gap-2 border-b border-slate-700 pb-4 md:flex-row md:items-end md:justify-between">
                <div>
                    <p className="text-sm font-medium text-sky-300">Insurance</p>
                    <h2 id="insurance-analytics-title" className="text-2xl font-bold text-white">
                        Insurance Analytics Dashboard
                    </h2>
                </div>
                {lastUpdated && (
                    <p className="text-xs text-slate-500">
                        Last updated: {lastUpdated.toLocaleTimeString()}
                    </p>
                )}
            </div>

            <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4">
                <MetricCard
                    icon={ShieldCheck}
                    label="Active Policies"
                    value={analytics.activePolicies.toLocaleString()}
                    detail={`${analytics.totalPolicies.toLocaleString()} total`}
                    tone="border-emerald-500/30 text-emerald-300"
                />
                <MetricCard
                    icon={CircleDollarSign}
                    label="Premiums Collected"
                    value={`${currencyFormatter.format(analytics.totalPremiumsCollected)} XLM`}
                    detail="12 month"
                    tone="border-cyan-500/30 text-cyan-300"
                />
                <MetricCard
                    icon={Percent}
                    label="Claim Ratio"
                    value={`${percentFormatter.format(summary.claimRatio)}%`}
                    detail="paid / premium"
                    tone="border-amber-500/30 text-amber-300"
                />
                <MetricCard
                    icon={FileWarning}
                    label="Open Claims"
                    value={analytics.openClaims.toLocaleString()}
                    detail={`${percentFormatter.format(summary.approvalRate)}% approved`}
                    tone="border-rose-500/30 text-rose-300"
                />
            </div>

            <div className="grid grid-cols-1 gap-6 xl:grid-cols-3">
                <div className="rounded-lg border border-slate-700 bg-slate-900/80 p-5 xl:col-span-2">
                    <div className="mb-5 flex items-center justify-between gap-3">
                        <div>
                            <h3 className="text-base font-semibold text-white">Premiums vs Claims</h3>
                            <p className="text-sm text-slate-400">Monthly loss-ratio trend</p>
                        </div>
                        <TrendingUp className="h-5 w-5 text-emerald-300" aria-hidden="true" />
                    </div>
                    <div className="h-72">
                        <ResponsiveContainer width="100%" height="100%">
                            <AreaChart data={analytics.monthlyTrend}>
                                <CartesianGrid stroke="#334155" strokeDasharray="3 3" />
                                <XAxis dataKey="month" stroke="#94a3b8" />
                                <YAxis stroke="#94a3b8" />
                                <Tooltip
                                    contentStyle={{ background: '#0f172a', border: '1px solid #334155' }}
                                    formatter={(value, name) => [
                                        `${currencyFormatter.format(value)} XLM`,
                                        name,
                                    ]}
                                />
                                <Area
                                    type="monotone"
                                    dataKey="premiums"
                                    name="Premiums"
                                    stroke="#22c55e"
                                    fill="#22c55e"
                                    fillOpacity={0.18}
                                />
                                <Area
                                    type="monotone"
                                    dataKey="claims"
                                    name="Claims"
                                    stroke="#f59e0b"
                                    fill="#f59e0b"
                                    fillOpacity={0.18}
                                />
                            </AreaChart>
                        </ResponsiveContainer>
                    </div>
                </div>

                <div className="rounded-lg border border-slate-700 bg-slate-900/80 p-5">
                    <div className="mb-5 flex items-center justify-between gap-3">
                        <div>
                            <h3 className="text-base font-semibold text-white">Claim Status</h3>
                            <p className="text-sm text-slate-400">Current claim queue</p>
                        </div>
                        <Activity className="h-5 w-5 text-sky-300" aria-hidden="true" />
                    </div>
                    <div className="h-72">
                        <ResponsiveContainer width="100%" height="100%">
                            <PieChart>
                                <Pie
                                    data={analytics.claimStatusBreakdown}
                                    dataKey="count"
                                    nameKey="status"
                                    innerRadius={54}
                                    outerRadius={88}
                                    paddingAngle={3}
                                >
                                    {analytics.claimStatusBreakdown.map((entry) => (
                                        <Cell key={entry.status} fill={statusColors[entry.status]} />
                                    ))}
                                </Pie>
                                <Tooltip
                                    contentStyle={{ background: '#0f172a', border: '1px solid #334155' }}
                                />
                            </PieChart>
                        </ResponsiveContainer>
                    </div>
                    <div className="grid grid-cols-2 gap-3">
                        {analytics.claimStatusBreakdown.map((entry) => (
                            <div key={entry.status} className="flex items-center gap-2 text-sm text-slate-300">
                                <span
                                    className="h-2.5 w-2.5 rounded-full"
                                    style={{ backgroundColor: statusColors[entry.status] }}
                                />
                                {entry.status}: {entry.count}
                            </div>
                        ))}
                    </div>
                </div>
            </div>

            <div className="grid grid-cols-1 gap-6 xl:grid-cols-3">
                <div className="rounded-lg border border-slate-700 bg-slate-900/80 p-5 xl:col-span-2">
                    <div className="mb-5">
                        <h3 className="text-base font-semibold text-white">Pool Utilization</h3>
                        <p className="text-sm text-slate-400">Capital committed by coverage pool</p>
                    </div>
                    <div className="h-72">
                        <ResponsiveContainer width="100%" height="100%">
                            <BarChart data={analytics.poolUtilization}>
                                <CartesianGrid stroke="#334155" strokeDasharray="3 3" />
                                <XAxis dataKey="pool" stroke="#94a3b8" />
                                <YAxis stroke="#94a3b8" />
                                <Tooltip
                                    contentStyle={{ background: '#0f172a', border: '1px solid #334155' }}
                                    formatter={(value) => [`${value}%`, 'Utilization']}
                                />
                                <Bar dataKey="utilization" fill="#38bdf8" radius={[4, 4, 0, 0]} />
                            </BarChart>
                        </ResponsiveContainer>
                    </div>
                </div>

                <div className="rounded-lg border border-slate-700 bg-slate-900/80 p-5">
                    <h3 className="text-base font-semibold text-white">Risk Snapshot</h3>
                    <div className="mt-5 space-y-4">
                        <div>
                            <div className="mb-2 flex items-center justify-between text-sm">
                                <span className="text-slate-400">Reserve Ratio</span>
                                <span className="font-medium text-white">
                                    {percentFormatter.format(summary.reserveRatio)}%
                                </span>
                            </div>
                            <div className="h-2 rounded-full bg-slate-800">
                                <div
                                    className="h-2 rounded-full bg-emerald-400"
                                    style={{ width: `${Math.min(summary.reserveRatio, 100)}%` }}
                                />
                            </div>
                        </div>
                        <div>
                            <p className="text-sm text-slate-400">Coverage Exposure</p>
                            <p className="mt-1 text-xl font-semibold text-white">
                                {currencyFormatter.format(analytics.coverageExposure)} XLM
                            </p>
                        </div>
                        <div>
                            <p className="text-sm text-slate-400">Available Capital</p>
                            <p className="mt-1 text-xl font-semibold text-white">
                                {currencyFormatter.format(analytics.availableCapital)} XLM
                            </p>
                        </div>
                        <div>
                            <p className="text-sm text-slate-400">Average Claim Severity</p>
                            <p className="mt-1 text-xl font-semibold text-white">
                                {currencyFormatter.format(analytics.averageClaimSeverity)} XLM
                            </p>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    );
};

export default InsuranceAnalyticsDashboard;
