from playwright.sync_api import sync_playwright
import time

with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    page = browser.new_page(viewport={"width": 1400, "height": 900})
    
    page.goto("http://localhost:1420/settings")
    page.wait_for_load_state("networkidle")
    time.sleep(3)
    
    page.screenshot(path="/tmp/08_settings_full.png", full_page=True)
    
    tabs = page.locator('[data-testid="settings-sidebar"] .ant-tabs-tab')
    tab_count = tabs.count()
    print(f"Found {tab_count} tabs")
    
    for i in range(tab_count):
        try:
            tab = tabs.nth(i)
            tab_html = tab.inner_html()
            print(f"  Tab {i} HTML: {tab_html[:200]}")
        except Exception as e:
            print(f"  Tab {i} error: {e}")
    
    if tab_count >= 5:
        data_tab = tabs.nth(4)
        data_tab.click()
        time.sleep(1)
        print("Clicked tab index 4 (data)")
        page.screenshot(path="/tmp/09_data_tab_clicked.png", full_page=True)
        
        menu_items = page.locator('[data-testid="settings-sidebar"] .ant-menu-item')
        count = menu_items.count()
        print(f"Found {count} menu items after clicking data tab")
        for i in range(count):
            try:
                text = menu_items.nth(i).inner_text()
                print(f"  Item {i}: {text}")
            except:
                pass
        
        if count >= 3:
            backup_item = menu_items.nth(2)
            backup_item.click()
            time.sleep(2)
            print("Clicked backup center (item 2)")
            page.screenshot(path="/tmp/10_backup_center.png", full_page=True)
            
            content_area = page.locator(".min-w-0.flex-1.overflow-y-auto")
            if content_area.is_visible(timeout=3000):
                content_html = content_area.inner_html()
                with open("/tmp/backup_content2.html", "w", encoding="utf-8") as f:
                    f.write(content_html[:10000])
                print(f"Content area HTML length: {len(content_html)}")
    
    browser.close()
