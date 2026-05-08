from playwright.sync_api import sync_playwright
import time

with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    page = browser.new_page(viewport={"width": 1400, "height": 900})
    
    page.goto("http://localhost:1420/settings")
    page.wait_for_load_state("networkidle")
    time.sleep(3)
    
    page.screenshot(path="/tmp/11_settings_initial.png", full_page=True)
    
    modals = page.locator(".ant-modal-wrap")
    modal_count = modals.count()
    print(f"Found {modal_count} modals")
    for i in range(modal_count):
        try:
            modal = modals.nth(i)
            is_visible = modal.is_visible()
            print(f"  Modal {i}: visible={is_visible}")
            if is_visible:
                modal_html = modal.inner_html()
                print(f"  Modal {i} HTML: {modal_html[:300]}")
                close_btn = modal.locator(".ant-modal-close")
                if close_btn.is_visible(timeout=1000):
                    close_btn.click()
                    time.sleep(0.5)
                    print(f"  Closed modal {i}")
        except Exception as e:
            print(f"  Modal {i} error: {e}")
    
    page.screenshot(path="/tmp/12_after_close_modal.png", full_page=True)
    
    tabs = page.locator('[data-testid="settings-sidebar"] .ant-tabs-tab')
    tab_count = tabs.count()
    
    if tab_count >= 5:
        data_tab = tabs.nth(4)
        data_tab.click()
        time.sleep(1)
        print("Clicked data tab")
        page.screenshot(path="/tmp/13_data_tab.png", full_page=True)
        
        menu_items = page.locator('[data-testid="settings-sidebar"] .ant-menu-item')
        count = menu_items.count()
        print(f"Found {count} menu items")
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
            print("Clicked backup center")
            page.screenshot(path="/tmp/14_backup_center.png", full_page=True)
            
            content_area = page.locator(".min-w-0.flex-1.overflow-y-auto")
            if content_area.is_visible(timeout=3000):
                content_html = content_area.inner_html()
                with open("/tmp/backup_content3.html", "w", encoding="utf-8") as f:
                    f.write(content_html[:10000])
                print(f"Content area HTML length: {len(content_html)}")
    
    browser.close()
