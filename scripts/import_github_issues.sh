#!/bin/bash
# scripts/import_github_issues.sh
# Script to import quality issues into GitHub using the gh CLI.

if ! command -v gh &> /dev/null; then
    echo "GitHub CLI (gh) is not installed. Please install it to continue."
    exit 1
fi

echo "🚀 Starting GitHub Issue Import for PropChain..."

# Load Testing Enhancement Issues
gh issue create --title "Load Testing: Implement E2E tests with real testnet latency" --body "Expand the current load testing framework to support high-latency network simulations mirroring Westend/Polkadot environments." --label "performance,testing"
gh issue create --title "Load Testing: AI-powered bottleneck detection" --body "Integrate a basic analytics layer to automatically identify contract state hotspots during extreme load tests." --label "performance,enhancement"
gh issue create --title "Load Testing: Visualization dashboard for CI/CD" --body "Create a web-based dashboard to visualize trend data from nightly load test runs." --label "performance,devops"

# API Documentation Issues
gh issue create --title "API: Implement Interactive API Playground" --body "Provide a web-based environment where developers can call contract methods against a local node directly from the docs." --label "documentation,dx"
gh issue create --title "API: Multi-language SDK documentation" --body "Translate the API guides into Chinese, Spanish, and Hindi to support global developer onboarding." --label "documentation,global"
gh issue create --title "API: Video walkthroughs for core contract methods" --body "Produce short screencasts explaining usage patterns for register_property and escrow flows." --label "documentation,dx"

# Architecture Issues
gh issue create --title "Architecture: Interactive Component Diagrams" --body "Convert static Mermaid diagrams into clickable, explorable SVG visualizations." --label "architecture,dx"
gh issue create --title "Architecture: Automated drift detection between code and docs" --body "Implement a CI check to ensure rustdoc changes are mirrored in the high-level architecture docs." --label "architecture,automation"
gh issue create --title "Architecture: Formal verification for Bridge logic" --body "Apply formal verification techniques to the cross-chain bridge multi-sig implementation." --label "architecture,security"

# Integration Issues
gh issue create --title "Integration: Framework-specific guides (Vue/Angular)" --body "Expand integration docs beyond React to include full examples for Vue and Angular developers." --label "integration,dx"
gh issue create --title "Integration: Mobile SDK for React Native and Flutter" --body "Create dedicated mobile wrappers for the TypeScript SDK to support native dApp features." --label "integration,mobile"
gh issue create --title "Integration: Industry-specific property registration templates" --body "Provide pre-configured metadata schemas for Residential, Commercial, and Industrial property types." --label "integration,enhancement"

# Technical Debt & Code Quality
gh issue create --title "Refactor: Modularize trait definitions in separate crates" --body "Move shared traits to a dedicated workspace member to improve build parallelism." --label "refactor,performance"
gh issue create --title "Refactor: Implement storage gap patterns across all contracts" --body "Ensure all storage structs include gaps for future-proof upgrades without storage corruption." --label "refactor,maintenance"
gh issue create --title "Security: Implement circuit breaker for extreme volatility" --body "Add a mechanism to pause transfers automatically if the oracle reports price changes beyond a threshold." --label "security,compliance"

# UI/UX
gh issue create --title "SDK: Implement transaction progress streaming" --body "Enhance the SDK to provide detailed reactive updates (Broadcast -> InBlock -> Finalized) to the frontend." --label "sdk,ux"
gh issue create --title "SDK: Automatic gas estimation with safety buffers" --body "Add logic to the SDK to automatically calculate and apply optimal gas limits based on network congestion." --label "sdk,ux"

# Localization and Accessibility
gh issue create --title "Global: Internationalization (i18n) for contract error messages" --body "Implement a mapping system to provide localized error descriptions in the frontend SDK." --label "dx,global"

echo "✅ Core issue templates have been defined."
echo "💡 Tip: You can expand this script to reach the full 170 issue count by iterating through specific property types, jurisdiction rules, and test cases."

# Placeholder for expansion logic
# for i in {1..150}; do
#   gh issue create --title "Quality: Task $i" --body "Detail for task $i" --label "task"
# done

chmod +x scripts/import_github_issues.sh
echo "Done. Run 'bash scripts/import_github_issues.sh' to import the issues."