#!/usr/bin/env python3
"""
携程网机票查询脚本
使用 Playwright 自动化浏览器查询明天从海口到北京的机票信息
"""

import asyncio
from datetime import datetime, timedelta
from playwright.async_api import (
    async_playwright,
    TimeoutError as PlaywrightTimeoutError,
)


async def search_flights():
    """主函数：查询携程网机票"""

    # 计算明天的日期
    tomorrow = datetime.now() + timedelta(days=1)
    tomorrow_str = tomorrow.strftime("%Y-%m-%d")
    print(f"📅 查询日期: {tomorrow_str}")

    async with async_playwright() as p:
        # 启动浏览器（使用 headed 模式便于观察）
        print("🚀 启动浏览器...")
        browser = await p.chromium.launch(
            headless=False,  # 设置为 True 可无头运行
            slow_mo=500,  # 放慢操作速度，便于观察
            args=["--start-maximized"],
        )

        # 创建浏览器上下文
        context = await browser.new_context(
            viewport={"width": 1920, "height": 1080},
            locale="zh-CN",
            timezone_id="Asia/Shanghai",
            user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        )

        page = await context.new_page()

        try:
            # ========== 步骤1: 导航到携程网 ==========
            print("🌐 导航到携程网...")
            await page.goto(
                "https://flights.ctrip.com/international/search/oneway-sha-bjs?depdate={}&cabin=y_s".format(
                    tomorrow_str.replace("-", "")
                ),
                wait_until="domcontentloaded",
                timeout=60000,
            )

            # 等待页面基本加载
            await page.wait_for_timeout(3000)

            # ========== 步骤2: 处理可能的弹窗 ==========
            print("🔍 检查并处理弹窗...")
            await handle_popups(page)

            # ========== 步骤3: 导航到国内机票页面 ==========
            print("✈️ 导航到国内机票搜索页面...")
            await page.goto(
                "https://flights.ctrip.com/online/list/oneway-hak-bjs?depdate={}&cabin=y_s".format(
                    tomorrow_str.replace("-", "")
                ),
                wait_until="domcontentloaded",
                timeout=60000,
            )

            await page.wait_for_timeout(3000)
            await handle_popups(page)

            # ========== 步骤4: 填写搜索信息 ==========
            print("📝 填写搜索信息...")

            # 如果页面未自动填好，手动填写
            try:
                # 点击出发城市输入框
                depart_input = page.locator(
                    'input[placeholder*="出发"], input[placeholder*="请输入"]'
                ).first
                if await depart_input.is_visible():
                    await depart_input.click()
                    await page.wait_for_timeout(500)
                    await depart_input.fill("海口")
                    await page.wait_for_timeout(1000)
                    # 选择下拉框中的海口
                    await page.keyboard.press("Enter")
            except Exception as e:
                print(f"  出发城市可能已自动填充: {e}")

            try:
                # 点击到达城市输入框
                arrive_input = page.locator(
                    'input[placeholder*="到达"], input[placeholder*="目的"]'
                ).first
                if await arrive_input.is_visible():
                    await arrive_input.click()
                    await page.wait_for_timeout(500)
                    await arrive_input.fill("北京")
                    await page.wait_for_timeout(1000)
                    await page.keyboard.press("Enter")
            except Exception as e:
                print(f"  到达城市可能已自动填充: {e}")

            # ========== 步骤5: 点击搜索按钮 ==========
            print("🔎 点击搜索按钮...")
            try:
                # 尝试多种可能的选择器
                search_selectors = [
                    'button:has-text("搜索")',
                    'button:has-text("查询")',
                    ".search-btn",
                    '[class*="search"]',
                    'button[type="submit"]',
                ]

                for selector in search_selectors:
                    try:
                        search_btn = page.locator(selector).first
                        if await search_btn.is_visible(timeout=2000):
                            await search_btn.click()
                            break
                    except:
                        continue
            except Exception as e:
                print(f"  搜索按钮点击可能已通过 URL 跳过: {e}")

            # ========== 步骤6: 等待结果加载 ==========
            print("⏳ 等待航班结果加载...")
            await page.wait_for_timeout(5000)

            # 等待航班列表出现
            try:
                await page.wait_for_selector(
                    '[class*="flight"], [class*="Flight"], [class*="list-item"]',
                    timeout=30000,
                )
            except PlaywrightTimeoutError:
                print("  警告: 未检测到航班列表，尝试继续提取...")

            # 再次处理可能的弹窗
            await handle_popups(page)

            # ========== 步骤7: 提取航班信息 ==========
            print("📊 提取航班信息...")
            flights = await extract_flight_info(page)

            # ========== 步骤8: 输出结果 ==========
            print("\n" + "=" * 80)
            print(
                f"✈️  海口(HAK) → 北京(BJS)  |  {tomorrow_str}  |  共找到 {len(flights)} 个航班"
            )
            print("=" * 80)

            if flights:
                for i, flight in enumerate(flights, 1):
                    print(f"\n【航班 {i}】")
                    print(f"  航空公司: {flight.get('airline', 'N/A')}")
                    print(f"  航班号:   {flight.get('flight_no', 'N/A')}")
                    print(f"  机型:     {flight.get('aircraft', 'N/A')}")
                    print(
                        f"  出发:     {flight.get('depart_time', 'N/A')}  {flight.get('depart_airport', '')}"
                    )
                    print(
                        f"  到达:     {flight.get('arrive_time', 'N/A')}  {flight.get('arrive_airport', '')}"
                    )
                    print(f"  价格:     ¥{flight.get('price', 'N/A')}")
                    print(f"  准点率:   {flight.get('on_time_rate', 'N/A')}")
            else:
                print("\n⚠️  未能提取到航班信息，可能原因:")
                print("  1. 页面结构已变化")
                print("  2. 需要人工验证/登录")
                print("  3. 网络问题")
                print("\n💡 建议: 保持浏览器窗口打开，手动观察页面状态")

            # 保持浏览器打开一段时间供观察
            print("\n⏸️  浏览器将在 30 秒后关闭，可手动查看...")
            await page.wait_for_timeout(30000)

        except Exception as e:
            print(f"\n❌ 发生错误: {e}")
            import traceback

            traceback.print_exc()

            # 保存截图用于调试
            screenshot_path = "error_screenshot.png"
            await page.screenshot(path=screenshot_path)
            print(f"📸 已保存错误截图: {screenshot_path}")

            # 保持浏览器打开以便调试
            print("⏸️  浏览器将保持打开 60 秒供调试...")
            await page.wait_for_timeout(60000)

        finally:
            await browser.close()
            print("\n✅ 完成!")


async def handle_popups(page):
    """处理各种弹窗"""
    popup_handlers = [
        # 关闭登录提示
        (
            'button[class*="close"], [class*="close-btn"], .close, [aria-label="关闭"]',
            "关闭按钮",
        ),
        # 关闭广告弹窗
        (
            '[class*="modal"] button[class*="close"], [class*="dialog"] [class*="close"]',
            "弹窗关闭",
        ),
        # 关闭 APP 下载提示
        ('[class*="app-download"] .close, [class*="download"] .close', "APP下载提示"),
        # 点击"稍后再说"
        (
            'button:has-text("稍后再说"), button:has-text("暂不"), button:has-text("取消")',
            "稍后提示",
        ),
        # 关闭 Cookie 提示
        (
            'button:has-text("接受"), button:has-text("同意"), button:has-text("我知道了")',
            "Cookie提示",
        ),
    ]

    for selector, desc in popup_handlers:
        try:
            elements = await page.locator(selector).all()
            for elem in elements:
                if await elem.is_visible(timeout=1000):
                    await elem.click(timeout=2000)
                    print(f"  ✓ 已处理: {desc}")
                    await page.wait_for_timeout(500)
        except:
            pass


async def extract_flight_info(page):
    """提取航班信息"""
    flights = []

    # 获取页面内容用于分析
    content = await page.content()

    # 尝试多种选择器策略
    selectors = [
        # 携程常见的航班卡片选择器
        '[class*="flight-item"]',
        '[class*="FlightItem"]',
        '[class*="list-item"]',
        '[class*="flight-card"]',
        "[data-flight]",
        ".flight-box",
    ]

    flight_elements = []
    for selector in selectors:
        try:
            elements = await page.locator(selector).all()
            if elements:
                flight_elements = elements
                print(f"  使用选择器: {selector}, 找到 {len(elements)} 个元素")
                break
        except:
            continue

    # 如果找到航班元素
    for elem in flight_elements[:20]:  # 最多提取20个航班
        try:
            flight_info = {}

            # 提取航空公司和航班号
            airline_text = await elem.locator(
                '[class*="airline"], [class*="flight-no"], [class*="Airline"]'
            ).first.text_content()
            if airline_text:
                parts = airline_text.strip().split()
                flight_info["airline"] = parts[0] if parts else ""
                flight_info["flight_no"] = parts[1] if len(parts) > 1 else ""

            # 提取时间
            time_elems = await elem.locator(
                '[class*="time"], [class*="Time"]'
            ).all_text_contents()
            if len(time_elems) >= 2:
                flight_info["depart_time"] = time_elems[0].strip()
                flight_info["arrive_time"] = time_elems[1].strip()

            # 提取机场
            airport_elems = await elem.locator(
                '[class*="airport"], [class*="Airport"]'
            ).all_text_contents()
            if len(airport_elems) >= 2:
                flight_info["depart_airport"] = airport_elems[0].strip()
                flight_info["arrive_airport"] = airport_elems[1].strip()

            # 提取价格
            price_elem = await elem.locator(
                '[class*="price"], [class*="Price"]'
            ).first.text_content()
            if price_elem:
                import re

                price_match = re.search(r"(\d+)", price_elem.replace(",", ""))
                flight_info["price"] = price_match.group(1) if price_match else ""

            # 提取机型
            craft_elem = await elem.locator(
                '[class*="craft"], [class*="plane"], [class*="机型"]'
            ).first.text_content()
            flight_info["aircraft"] = craft_elem.strip() if craft_elem else ""

            # 提取准点率
            rate_elem = await elem.locator(
                '[class*="rate"], [class*="准点"]'
            ).first.text_content()
            flight_info["on_time_rate"] = rate_elem.strip() if rate_elem else ""

            # 只有有基本信息的才添加
            if flight_info.get("flight_no") or flight_info.get("price"):
                flights.append(flight_info)

        except Exception as e:
            continue

    # 如果上述方法失败，尝试通用的文本提取
    if not flights:
        print("  尝试通用提取方法...")
        flights = await extract_generic_flight_info(page)

    return flights


async def extract_generic_flight_info(page):
    """通用的航班信息提取方法"""
    flights = []

    try:
        # 获取所有文本内容
        text_content = await page.inner_text("body")

        # 使用正则表达式提取航班号模式 (如: HU7181, CA1352)
        import re

        flight_pattern = r"\b([A-Z]{2}\d{3,4})\b"
        flight_numbers = re.findall(flight_pattern, text_content)

        # 提取时间模式 (如: 08:30, 14:45)
        time_pattern = r"\b(\d{2}:\d{2})\b"
        times = re.findall(time_pattern, text_content)

        # 提取价格模式 (如: ¥1230, 1230元)
        price_pattern = r"[¥￥]?\s*(\d{3,5})\s*(?:元|起)?"
        prices = re.findall(price_pattern, text_content)

        # 组合信息
        unique_flights = list(set(flight_numbers))
        for i, flight_no in enumerate(unique_flights[:10]):
            flight_info = {
                "flight_no": flight_no,
                "depart_time": times[i * 2] if i * 2 < len(times) else "",
                "arrive_time": times[i * 2 + 1] if i * 2 + 1 < len(times) else "",
                "price": prices[i] if i < len(prices) else "",
            }
            flights.append(flight_info)

    except Exception as e:
        print(f"  通用提取失败: {e}")

    return flights


if __name__ == "__main__":
    print("=" * 80)
    print("🐦 携程网机票查询工具 - Playwright 自动化脚本")
    print("=" * 80)
    print("\n📋 功能说明:")
    print("  • 自动打开携程网")
    print("  • 查询明天 海口→北京 的机票")
    print("  • 提取并显示航班信息")
    print("  • 自动处理弹窗")
    print("\n⚠️  注意事项:")
    print("  • 首次运行需要安装浏览器: playwright install chromium")
    print("  • 建议使用 headed 模式观察运行过程")
    print("  • 携程可能会更新页面结构，需要适时调整选择器")
    print()

    # 运行主函数
    asyncio.run(search_flights())
