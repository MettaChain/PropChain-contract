import React from 'react';
import './index.css';
import InsuranceAnalyticsDashboard from './InsuranceAnalyticsDashboard';
import LendingDashboard from './LendingDashboard';

function App() {
  return (
    <div className="App min-h-screen bg-slate-950">
      <LendingDashboard />
      <main className="px-4 pb-10 md:px-8">
        <InsuranceAnalyticsDashboard />
      </main>
    </div>
  );
}

export default App;
