# 理杏仁 OpenAPI 完整对接完成报告

## 概述

**完成时间**: 2026-02-27
**状态**: ✅ 已完成

本次工作扩展了 `LixinAdapter`，成功对接了理杏仁 OpenAPI 的 13 个新接口中的 8 个可用接口。

## 实现详情

### 新增常量 (Endpoints)

```rust
// 金融专用接口
const BANK_FUNDAMENTAL_ENDPOINT: &str = "/cn/company/fundamental/bank";
const SECURITY_FUNDAMENTAL_ENDPOINT: &str = "/cn/company/fundamental/security";
const INSURANCE_FUNDAMENTAL_ENDPOINT: &str = "/cn/company/fundamental/insurance";
const BANK_FS_ENDPOINT: &str = "/cn/company/fs/bank";  // 待实现

// 公司信息接口
const COMPANY_INDUSTRIES_ENDPOINT: &str = "/cn/company/industries";
const COMPANY_INDICES_ENDPOINT: &str = "/cn/company/indices";
const COMPANY_CUSTOMERS_ENDPOINT: &str = "/cn/company/customers";  // 返回空数据

// 市场数据接口
const ANNOUNCEMENT_ENDPOINT: &str = "/cn/company/announcement";
const BLOCK_DEAL_ENDPOINT: &str = "/cn/company/block-deal";
const PLEDGE_ENDPOINT: &str = "/cn/company/pledge";
const OPERATING_DATA_ENDPOINT: &str = "/cn/company/operating-data";  // 返回空数据

// 指数数据
const INDEX_FUNDAMENTAL_ENDPOINT: &str = "/cn/index/fundamental";
```

### 新增数据结构

| 结构体 | 用途 |
|-------|------|
| `IndustryClassification` | 公司行业分类（申万/中信/国证） |
| `IndexConstituent` | 公司所属指数成分 |
| `Announcement` | 公司公告信息 |
| `BlockDeal` | 大宗交易数据 |
| `PledgeInfo` | 股权质押信息 |
| `IndexFundamental` | 指数基本面数据 |
| `IndexFundamentalMetrics` | 指数基本面指标常量 |

### 新增方法

| 方法 | 功能 | 返回类型 |
|-----|------|---------|
| `get_company_industries()` | 获取公司行业分类 | `Vec<IndustryClassification>` |
| `get_company_indices()` | 获取公司所属指数 | `Vec<IndexConstituent>` |
| `get_announcements()` | 获取公司公告 | `Vec<Announcement>` |
| `get_block_deals()` | 获取大宗交易 | `Vec<BlockDeal>` |
| `get_pledge_info()` | 获取股权质押 | `Vec<PledgeInfo>` |
| `get_index_fundamental()` | 获取指数基本面 | `Vec<IndexFundamental>` |
| `get_bank_valuation()` | 获取银行估值 | `Vec<ValuationMetrics>` |
| `get_security_valuation()` | 获取证券估值 | `Vec<ValuationMetrics>` |
| `get_insurance_valuation()` | 获取保险估值 | `Vec<ValuationMetrics>` |

### 测试验证结果

```
═══════════════════════════════════════════════════════════════
          Lixin API New Endpoints Verification Report
═══════════════════════════════════════════════════════════════

[1/8] Company Industries (/cn/company/industries)... ✅ PASS (9 classifications)
[2/8] Company Indices (/cn/company/indices)... ✅ PASS (48 indices)
[3/8] Announcements (/cn/company/announcement)... ✅ PASS (4 announcements)
[4/8] Block Deals (/cn/company/block-deal)... ✅ PASS (4 deals)
[5/8] Index Fundamental (/cn/index/fundamental)... ✅ PASS (1 indices)
[6/8] Bank Valuation (/cn/company/fundamental/bank)... ✅ PASS (PE: 6.72, PB: 0.65)
[7/8] Security Valuation (/cn/company/fundamental/security)... ✅ PASS (PE: 12.40, PB: 1.13)
[8/8] Insurance Valuation (/cn/company/fundamental/insurance)... ✅ PASS (PE: 8.14, PB: 1.18)

Summary:
  Passed: 8/8
═══════════════════════════════════════════════════════════════
```

## 官方文档验证

**验证时间**: 2026-02-27
**验证来源**: https://www.lixinger.com/open/api/doc

### 端点验证结果

| 接口 | 官方端点 | 实现状态 |
|-----|---------|---------|
| 大宗交易 | `https://open.lixinger.com/api/cn/company/block-deal` | ✅ 匹配 |
| 股权质押 | `https://open.lixinger.com/api/cn/company/pledge` | ✅ 匹配 |
| 股票所属指数 | `https://open.lixinger.com/api/cn/company/indices` | ✅ 匹配 |
| 股票所属行业 | `https://open.lixinger.com/api/cn/company/industries` | ✅ 匹配 |
| 公告 | `https://open.lixinger.com/api/cn/company/announcement` | ✅ 匹配 |
| 指数基本面 | `https://open.lixinger.com/api/cn/index/fundamental` | ✅ 匹配 |
| 银行基本面 | `https://open.lixinger.com/api/cn/company/fundamental/bank` | ✅ 匹配 |
| 证券基本面 | `https://open.lixinger.com/api/cn/company/fundamental/security` | ✅ 匹配 |
| 保险基本面 | `https://open.lixinger.com/api/cn/company/fundamental/insurance` | ✅ 匹配 |

### 参数规范确认

| 接口 | 参数名 | 必填 | 备注 |
|-----|-------|-----|------|
| 公司信息类 | `stockCode` | Yes | 单数形式，非 `stockCodes` |
| 公告 | `stockCode`, `startDate` | Yes | 两个必填参数 |
| 股权质押 | `stockCode`, `startDate` | Yes | 两个必填参数 |
| 大宗交易 | `stockCode` | No | 仅在 date range 模式下生效 |
| 指数基本面 | `stockCodes` | Yes | 复数形式，数组 |

### 指数基本面 metricsList 格式

官方文档明确了三种格式：

1. **完整格式**: `[metricsName].[granularity].[metricsType].[statisticsDataType]`
   - 示例: `pe_ttm.y10.mcw.cvpos` (10年市值加权PE分位点)

2. **简化格式**: `[metricsName].[metricsType]`
   - 示例: `pe_ttm.mcw` (市值加权PE)

3. **基础格式**: `[metricsName]`
   - 示例: `mc` (市值), `tv` (成交量), `cp` (收盘点位)

**metricsType 选项**:
- `mcw` - 市值加权
- `ew` - 等权
- `ewpvo` - 正数等权
- `avg` - 平均值
- `median` - 中位数

**statisticsDataType 选项**:
- `cv` - 当前值
- `cvpos` - 分位点%
- `minv`/`maxv` - 最小/最大值
- `q2v`/`q5v`/`q8v` - 20%/50%/80%分位点值

## 技术细节

### API 响应处理

1. **空对象处理**: `/cn/company/indices` 接口返回的数组中可能包含空对象 `{}`，通过将 `IndexConstituent` 的字段改为 `Option` 类型并添加 `is_valid()` 方法过滤无效条目。

2. **金融行业估值复用**: 银行/证券/保险的 `fundamental` 接口使用与非金融公司相同的 `metricsList` 格式（pe_ttm, pb, dyr, mc），因此复用了现有的 `LixinNonFinancialRequest` 结构。

3. **指数基本面已完善**: `/cn/index/fundamental` 接口现已支持完整的估值指标：
   - 基础指标: `mc` (市值)
   - 估值指标: `pe_ttm.mcw`, `pb.mcw`, `dyr.mcw` (市值加权)
   - 分位点: `pe_ttm.y10.mcw.cvpos`, `pb.y10.mcw.cvpos` (10年历史分位)

   **API 验证结果** (2026-02-26 数据):
   | 指数 | PE-TTM | PB | PE分位 | PB分位 |
   |-----|--------|-----|--------|--------|
   | 沪深300 | 14.17 | 1.49 | 83.9% | 59.1% |
   | 中证500 | 38.72 | 2.65 | 88.0% | 83.0% |
   | 上证50 | 11.50 | 1.27 | 79.0% | 62.0% |

### 未实现接口

| 接口 | 原因 |
|-----|------|
| `cn/company/fs/bank` | 需要特殊的 `metricsList` 格式，待查阅文档 |
| `cn/company/customers` | 测试时返回空数组，可能数据覆盖不全 |
| `cn/company/operating-data` | 测试时返回空数组 |

## 代码变更

- **文件**: `services/zero-trading/src/data/lixin.rs`
  - 新增行数: ~760 行 (含指数估值扩展)
  - 总行数: 4050+ 行

- **文件**: `services/zero-trading/src/data/mod.rs`
  - 新增导出: 9 个类型

### 指数基本面增强 (2026-02-27 更新)

新增 `IndexFundamentalMetrics` 常量和方法：

```rust
// 估值指标常量
pub const PE_TTM_MCW: &str = "pe_ttm.mcw";
pub const PB_MCW: &str = "pb.mcw";
pub const DYR_MCW: &str = "dyr.mcw";
pub const PE_TTM_Y10_MCW_CVPOS: &str = "pe_ttm.y10.mcw.cvpos";
pub const PB_Y10_MCW_CVPOS: &str = "pb.y10.mcw.cvpos";

// 方法集合
fn valuation_metrics() -> Vec<&'static str>;
fn valuation_with_percentiles() -> Vec<&'static str>;
fn common_metrics() -> Vec<&'static str>;  // 默认返回 valuation_with_percentiles()
```

新增 `IndexFundamental` 字段：
- `pe_ttm_mcw`, `pb_mcw`, `dyr_mcw` - 市值加权估值
- `pe_ttm_y10_percentile`, `pb_y10_percentile`, `dyr_y10_percentile` - 10年分位点

新增方法：
- `get_index_fundamental_with_metrics()` - 支持自定义指标列表

## 后续优化建议

1. **文件拆分**: lixin.rs 已超过 4000 行，建议按功能模块拆分（如 `lixin/fundamental.rs`, `lixin/market.rs`）

2. ~~**指数估值**: 研究 `/cn/index/fundamental` 的完整 metricsList 格式以支持 PE/PB/股息率等指标~~ ✅ 已完成

3. **银行财报**: 查阅理杏仁文档获取银行财报的正确 `metricsList` 格式

4. **批量查询**: 为 `get_company_industries` 和 `get_company_indices` 添加批量查询支持

5. **等权估值**: 添加 `pe_ttm.ew`, `pb.ew` 等等权估值指标支持

---
*生成时间: 2026-02-27*
*更新时间: 2026-02-27 (指数估值增强)*
