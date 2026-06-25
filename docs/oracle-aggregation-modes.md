# Oracle Aggregation Modes

This document describes the different aggregation modes available in the Property Valuation Oracle.

## Simple Median

The simple median is the default aggregation mode. It is calculated by taking the median of all the property valuations. This is a good choice for most use cases, as it is simple to understand and resistant to outliers.

## Weighted Median

The weighted median is a more advanced aggregation mode that takes into account the reputation of the oracle sources. It is calculated by taking the median of the property valuations, weighted by the reputation of the oracle sources. This is a good choice for use cases where you want to give more weight to more reputable sources.

## Trimmed Mean

The trimmed mean is a more robust aggregation mode that is resistant to outliers. It is calculated by taking the mean of the property valuations, after discarding a certain percentage of the highest and lowest values. This is a good choice for use cases where you want to mitigate the impact of outliers and produce a more robust and reliable aggregation.