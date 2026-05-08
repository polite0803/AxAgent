from playwright.sync_api import sync_playwright
import time

with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    page = browser.new_page(viewport={"width": 1280, "height": 800})
    
    console_errors = []
    page.on("console", lambda msg: console_errors.append(f"[{msg.type}] {msg.text}") if msg.type in ["error", "warning"] else None)
    page.on("pageerror", lambda err: console_errors.append(f"[PAGE_ERROR] {err}"))
    
    page.goto("http://localhost:1420/settings")
    page.wait_for_load_state("networkidle")
    time.sleep(3)
    
    page.screenshot(path="/tmp/05_settings_initial.png", full_page=True)
    
    sidebar = page.locator('[data-testid="settings-sidebar"]')
    if sidebar.is_visible(timeout=5000):
        sidebar_html = sidebar.inner_html()
        with open("/tmp/sidebar_content.html", "w", encoding="utf-8") as f:
            f.write(sidebar_html[:10000])
        print("Sidebar HTML saved")
    else:
        print("Sidebar not visible")
    
    all_menu_items = page.locator('[data-testid="settings-sidebar"] .ant-menu-item, [data-testid="settings-sidebar"] .ant-menu-submenu-title')
    count = all_menu_items.count()
    print(f"Found {count} menu items")
    for i in range(min(count, 30)):
        try:
            text = all_menu_items.nth(i).inner_text()
            print(f"  Item {i}: {text}")
        except:
            pass
    
    tabs = page.locator('[data-testid="settings-sidebar"] .ant-tabs-tab')
    tab_count = tabs.count()
    print(f"Found {tab_count} tabs")
    for i in range(tab_count):
        try:
            text = tabs.nth(i).inner_text()
            print(f"  Tab {i}: {text}")
        except:
            pass
    
    data_tab = page.locator('[data-testid="settings-sidebar"]').get_by_text("数据", exact=False).first
    if data_tab.is_visible(timeout=3000):
        data_tab.click()
        time.sleep(1)
        print("Clicked data tab")
        page.screenshot(path="/tmp/06_data_tab.png", full_page=True)
        
        all_menu_items2 = page.locator('[data-testid="settings-sidebar"] .ant-menu-item')
        count2 = all_menu_items2.count()
        print(f"After data tab: Found {count2} menu items")
        for i in range(min(count2, 20)):
            try:
                text = all_menu_items2.nth(i).inner_text()
                print(f"  Item {i}: {text}")
            except:
                pass
        
        backup_item = page.locator('[data-testid="settings-sidebar"]').get_by_text("备份中心", exact=False).first
        if backup_item.is_visible(timeout=3000):
            backup_item.click()
            time.sleep(2)
            print("Clicked backup center")
            page.screenshot(path="/tmp/07_backup_center.png", full_page=True)
        else:
            print("Backup center item not visible after clicking data tab")
    else:
        print("Data tab not found")
    
    print("\n=== Console Errors ===")
    for err in console_errors[-20:]:
        print(err)
    
    browser.close()
