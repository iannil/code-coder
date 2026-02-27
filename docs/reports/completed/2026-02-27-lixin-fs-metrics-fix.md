# Fix Lixinger API fs/non_financial Invalid Metrics Error

**Date**: 2026-02-27
**Status**: Completed
**Component**: `services/zero-trading/src/data/lixin.rs`

## Problem

The `zero-trading` service was failing to fetch financial data from Lixinger API with HTTP 400 errors:

```
"(q.bs.tse.t,q.bs.caea.t,q.ps.or.t,q.ps.gp.t,q.cfs.cffaacogola.t,q.m.gpm.t,q.m.npm.t,q.m.ld_e.t,q.m.cr.t) are invalid fs metrics"
```

The root cause was that `FsMetricCodes::all_metrics()` included 9 metric codes that don't exist in the Lixinger `/cn/company/fs/non_financial` API.

## Solution

### Changes Made

1. **Added `valid_metrics()` method** (lines 2490-2505)
   - Returns only the 12 metrics that are valid for the API
   - Deprecated `all_metrics()` but kept for reference

2. **Updated `batch_fetch_financial_data()`** (line 750)
   - Changed from `FsMetricCodes::all_metrics()` to `FsMetricCodes::valid_metrics()`

3. **Updated response parsing** (lines 791-850)
   - Added derived calculations for `total_equity` and `debt_to_equity`
   - Set invalid metrics to `None`

### Valid Metrics (12)

| Constant | Code | Meaning |
|----------|------|---------|
| `TOTAL_ASSETS` | `q.bs.ta.t` | Total Assets |
| `TOTAL_LIABILITIES` | `q.bs.tl.t` | Total Liabilities |
| `TOTAL_DEBT` | `q.bs.stl.t` | Short-term Loans |
| `SHARES_OUTSTANDING` | `q.bs.tsc.t` | Total Share Capital |
| `OPERATING_INCOME` | `q.ps.op.t` | Operating Profit |
| `NET_INCOME` | `q.ps.np.t` | Net Profit |
| `INTEREST_EXPENSE` | `q.ps.ie.t` | Interest Expense |
| `OPERATING_CASH_FLOW` | `q.cfs.ncffoa.t` | Net Cash from Operating |
| `INVESTING_CASH_FLOW` | `q.cfs.ncffia.t` | Net Cash from Investing |
| `FINANCING_CASH_FLOW` | `q.cfs.ncfffa.t` | Net Cash from Financing |
| `ROE` | `q.m.roe.t` | Return on Equity |
| `ROA` | `q.m.roa.t` | Return on Assets |

### Invalid Metrics (9) - Removed from API requests

| Constant | Code | Status |
|----------|------|--------|
| `TOTAL_EQUITY` | `q.bs.tse.t` | **Derived**: `total_assets - total_liabilities` |
| `DEBT_TO_EQUITY` | `q.m.ld_e.t` | **Derived**: `total_liabilities / total_equity` |
| `CASH` | `q.bs.caea.t` | Set to `None` |
| `REVENUE` | `q.ps.or.t` | Set to `None` |
| `GROSS_PROFIT` | `q.ps.gp.t` | Set to `None` |
| `CAPEX` | `q.cfs.cffaacogola.t` | Set to `None` |
| `GROSS_MARGIN` | `q.m.gpm.t` | Set to `None` |
| `NET_MARGIN` | `q.m.npm.t` | Set to `None` |
| `CURRENT_RATIO` | `q.m.cr.t` | Set to `None` |

## Verification

```bash
# Build successful
cd services && cargo build -p zero-trading

# Verify no 400 errors after restart
./ops.sh build rust
./ops.sh start
./ops.sh logs zero-trading | grep "400 Bad Request"  # Should be empty
./ops.sh logs zero-trading | grep "financial data"   # Should show success
```

## Future Improvements

- Investigate Lixinger API documentation for alternative metric codes
- Consider using deprecated individual endpoints as fallback for missing metrics
