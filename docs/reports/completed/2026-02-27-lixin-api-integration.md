# 理杏仁 API 接口对接完成报告

**日期**: 2026-02-27
**状态**: 已完成

## 概述

完成了理杏仁 API 的扩展接口对接，包括公司详情、基金数据和估值统计计算功能。

## 完成内容

### Phase 1: 公司详情 API (`/cn/company/profile`)

**文件**: `services/zero-trading/src/data/lixin.rs`

新增功能：
- 添加端点常量 `COMPANY_PROFILE_ENDPOINT`
- 添加请求结构体 `LixinProfileRequest`
- 添加响应结构体 `CompanyProfile`（公开导出）
- 实现 `get_company_profile()` 方法，支持批量查询

字段覆盖：
- 基本信息：股票代码、公司名称、公司简介
- 业务信息：主营业务描述
- 注册信息：注册资本、省份、城市、成立日期
- 人员信息：董事长、总经理、法定代表人、董秘
- 联系信息：网站、地址、电话、传真、邮箱
- 其他：员工人数

### Phase 2: 基金数据 API (`/cn/fund`, `/cn/fund/nav`)

新增功能：
- 添加端点常量 `FUND_LIST_ENDPOINT`, `FUND_NAV_ENDPOINT`
- 添加请求结构体 `LixinFundListRequest`, `LixinFundNavRequest`
- 添加响应结构体 `FundInfo`, `FundNav`（公开导出）
- 实现 `get_fund_list()` 方法，支持按类型筛选
- 实现 `get_fund_nav()` 方法，支持批量查询和日期范围

字段覆盖：
- 基金列表：代码、名称、全称、类型、公司、基金经理、成立日期、管理费率、托管费率、资产规模
- 基金净值：代码、日期、单位净值、累计净值、日收益率

### Phase 3: 估值统计计算

改进功能：
- 重构 `fetch_metric_statistics()` 方法
- 实现本地统计计算，不依赖付费 API
- 添加辅助方法：
  - `granularity_to_start_date()`: 将时间粒度转换为起始日期
  - `fetch_historical_valuation_data()`: 获取历史估值数据
  - `calculate_statistics()`: 计算统计指标
  - `percentile_value()`: 使用线性插值计算百分位数

统计指标：
- 当前值百分位（历史排名）
- 四分位数：Q25, Q50 (中位数), Q80
- 极值：最小值、最大值、平均值

### Phase 4: 测试覆盖

新增单元测试：
- `test_statistics_calculation`: 验证统计计算逻辑
- `test_percentile_calculation`: 验证百分位数计算
- `test_granularity_to_start_date`: 验证日期转换

新增集成测试：
- `test_get_company_profile`: 单个公司详情
- `test_get_company_profile_batch`: 批量公司详情
- `test_get_fund_list`: 基金列表
- `test_get_fund_list_by_type`: 按类型筛选基金
- `test_get_fund_nav`: 基金净值数据
- `test_valuation_statistics_with_local_calculation`: 估值统计
- `test_endpoint_verification_report_extended`: 扩展端点验证

## 测试结果

```bash
$ cargo test --lib lixin -- --nocapture

running 25 tests
test data::lixin::tests::test_is_index_symbol ... ok
test data::lixin::tests::test_to_lixin_code ... ok
test data::lixin::tests::test_percentile_calculation ... ok
test data::lixin::tests::test_provider_info ... ok
test data::lixin::tests::test_capabilities ... ok
test data::lixin::tests::test_granularity_to_start_date ... ok
test data::lixin::tests::test_statistics_calculation ... ok

test result: ok. 7 passed; 0 failed; 18 ignored
```

## API 端点状态汇总

| 端点 | 状态 | 说明 |
|------|------|------|
| `/cn/company` | ✅ 已实现 | 股票列表 |
| `/cn/company/candlestick` | ✅ 已实现 | 股票日K线 |
| `/cn/index/candlestick` | ✅ 已实现 | 指数日K线 |
| `/cn/company/fundamental/non_financial` | ✅ 已实现 | 估值指标 |
| `/cn/company/fs/non_financial` | ✅ 已实现 | 财务报表 |
| `/cn/company/profile` | ✅ **新增** | 公司详情 |
| `/cn/fund` | ✅ **新增** | 基金列表 |
| `/cn/fund/nav` | ✅ **新增** | 基金净值 |

## 技术要点

### 1. API 参数格式差异
- K线端点使用 `stockCode` (单数)
- 其他端点使用 `stockCodes` (复数数组)

### 2. 本地统计计算
由于统计 API 可能需要升级权限，实现了本地计算方案：
- 获取指定时间段内的历史估值数据
- 使用线性插值计算百分位数
- 支持 1年/3年/5年/10年/20年/上市以来 等粒度

### 3. 错误处理
- 历史数据不足（<10条）时返回基本统计（仅min/max/avg）
- 过滤无效值（0、负数等）

## 后续建议

1. **权限升级**: 联系理杏仁确认 token 权限范围，可能可以获取更多 API 访问
2. **缓存优化**: 考虑缓存历史估值数据以减少 API 调用
3. **并行优化**: 批量统计计算可考虑并行执行
