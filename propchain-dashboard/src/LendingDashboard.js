import React, { useState, useEffect } from 'react';
import { Activity, Landmark, AlertCircle } from 'lucide-react';
import { fetchContractStats } from './StellarClient';

const LendingDashboard = () => {
    const [stats, setStats] = useState({ total_loaned: "0", active_loans: 0, defaults: 0 });
    const [lastUpdated, setLastUpdated] = useState(null);

    useEffect(() => {
        const fetchData = () => {
            fetchContractStats().then((result) => {
                // Simulate updating stats once the RPC call completes
                setStats({ total_loaned: "5000", active_loans: 5, defaults: 0 });
                setLastUpdated(new Date());
            });
        };

        // Initial fetch
        fetchData();

        // Poll every 30 seconds
        const interval = setInterval(fetchData, 30000);

        return () => clearInterval(interval);
    }, []);

    return (
        <div className="bg-slate-950 p-4 text-white md:p-8">
            <h1 className="text-3xl font-bold mb-8 border-b border-slate-700 pb-4 flex items-center gap-3">
                PropChain Analytics
                <span className="flex items-center gap-2 text-sm font-normal text-slate-400">
                    <span className="relative flex h-3 w-3">
                        <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                        <span className="relative inline-flex rounded-full h-3 w-3 bg-emerald-500"></span>
                    </span>
                    Live
                </span>
            </h1>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                <div className="bg-slate-800 p-6 rounded-xl border border-emerald-500/30">
                    <Landmark className="text-emerald-400 mb-2" />
                    <p className="text-slate-400 text-sm">Total Volume</p>
                    <p className="text-2xl font-mono">{Number(stats.total_loaned).toLocaleString()} XLM</p>
                </div>

                <div className="bg-slate-800 p-6 rounded-xl border border-blue-500/30">
                    <Activity className="text-blue-400 mb-2" />
                    <p className="text-slate-400 text-sm">Active Loans</p>
                    <p className="text-2xl font-mono">{stats.active_loans.toLocaleString()}</p>
                </div>

                <div className="bg-slate-800 p-6 rounded-xl border border-red-500/30">
                    <AlertCircle className="text-red-400 mb-2" />
                    <p className="text-slate-400 text-sm">Defaults</p>
                    <p className="text-2xl font-mono">{stats.defaults.toLocaleString()}</p>
                </div>
            </div>

            {lastUpdated && (
                <p className="text-slate-500 text-xs mt-6 text-center">
                    Last updated: {lastUpdated.toLocaleTimeString()}
                </p>
            )}
        </div>
    );
};

export default LendingDashboard;
