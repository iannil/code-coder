# 携程网机票查询 - Playwright 自动化脚本

这是一个使用 Playwright 自动化浏览器查询携程网机票信息的 Python 脚本。

## 功能特性

- ✅ 自动启动浏览器并导航到携程网
- ✅ 自动填写出发城市（海口）和到达城市（北京）
- ✅ 自动选择明天的日期
- ✅ 自动处理各种弹窗（登录提示、广告、Cookie等）
- ✅ 提取航班信息（航空公司、航班号、时间、价格等）
- ✅ 支持 headed 和 headless 模式

## 安装依赖

### 1. 安装 Python 依赖

```bash
pip install -r requirements.txt
```

或直接安装：

```bash
pip install playwright
```

### 2. 安装浏览器（首次使用必须执行）

```bash
playwright install chromium
```

如果需要安装所有浏览器：

```bash
playwright install
```

## 使用方法

### 基本使用

```bash
python flight_search_ctrip.py
```

### 自定义参数

你可以在脚本中修改以下参数：

```python
# 修改出发城市和到达城市
# 在 URL 中将 'hak' 和 'bjs' 替换为其他城市代码
# HAK = 海口, BJS = 北京, SHA = 上海, CAN = 广州, SZX = 深圳

# 修改日期（当前默认为明天）
tomorrow = datetime.now() + timedelta(days=1)
# 可改为指定天数
future_date = datetime.now() + timedelta(days=7)  # 7天后

# 切换 headless 模式
browser = await p.chromium.launch(headless=True)  # 无头模式
```

## 脚本流程

```
1. 启动 Chromium 浏览器
   ↓
2. 导航到携程网机票页面
   ↓
3. 处理可能的弹窗（登录提示、广告等）
   ↓
4. 填写搜索信息
   - 出发城市：海口
   - 到达城市：北京
   - 出发日期：明天
   ↓
5. 点击搜索按钮
   ↓
6. 等待航班列表加载
   ↓
7. 提取航班信息
   - 航空公司
   - 航班号
   - 起飞/到达时间
   - 出发/到达机场
   - 价格
   - 准点率
   ↓
8. 输出结果到控制台
```

## 输出示例

```
================================================================================
✈️  海口(HAK) → 北京(BJS)  |  2024-01-15  |  共找到 15 个航班
================================================================================

【航班 1】
  航空公司: 海南航空
  航班号:   HU7181
  机型:     737
  出发:     08:30  海口美兰国际机场
  到达:     12:15  北京首都国际机场
  价格:     ¥1230
  准点率:   92%

【航班 2】
  航空公司: 中国国航
  航班号:   CA1352
  ...
```

## 常见问题

### 1. 浏览器启动失败

确保已安装浏览器：

```bash
playwright install chromium
```

### 2. 无法提取航班信息

可能原因：

- 页面结构已更新，需要修改选择器
- 网络问题导致页面未完全加载
- 触发了反爬机制

解决方法：

- 使用 headed 模式观察页面状态
- 检查页面截图（错误时会自动保存）
- 增加等待时间

### 3. 遇到登录验证

携程可能会要求登录或验证。解决方法：

- 保持 headed 模式，手动完成验证
- 使用 cookies 保持登录状态
- 添加随机延迟模拟人类操作

### 4. 选择器失效

携程可能会更新页面结构，需要：

- 查看错误截图
- 使用浏览器开发者工具检查元素
- 更新脚本中的选择器

## 高级用法

### 添加 Cookies 保持登录

```python
# 保存 cookies
await context.storage_state(path="state.json")

# 下次运行时加载
context = await browser.new_context(storage_state="state.json")
```

### 添加随机延迟（反爬）

```python
import random

# 在操作之间添加随机延迟
await page.wait_for_timeout(random.randint(1000, 3000))
```

### 截图保存

```python
# 保存当前页面截图
await page.screenshot(path="flights.png", full_page=True)
```

### 导出为 JSON

```python
import json

# 在脚本末尾添加
with open('flights.json', 'w', encoding='utf-8') as f:
    json.dump(flights, f, ensure_ascii=False, indent=2)
```

## 注意事项

⚠️ **免责声明**：

- 本脚本仅供学习研究使用
- 请遵守携程网的使用条款和 robots.txt
- 不要频繁请求，以免对服务器造成压力
- 商业用途请使用携程官方 API

⚠️ **反爬机制**：

- 携程有较强的反爬措施
- 建议添加合理的延迟
- 避免频繁大量请求
- 可能需要处理验证码

## 替代方案

如果需要稳定的航班查询，建议使用：

- 携程官方 API（需申请）
- 航空公司官方 API
- 第三方航班数据服务（如 VariFlight、FlightAware）

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！
