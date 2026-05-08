from playwright.sync_api import sync_playwright
import time

with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    page = browser.new_page(viewport={"width": 1280, "height": 800})
    
    console_errors = []
    page.on("console", lambda msg: console_errors.append(f"[{msg.type}] {msg.text}") if msg.type in ["error", "warning"] else None)
    page.on("pageerror", lambda err: console_errors.append(f"[PAGE_ERROR] {err}"))
    
    page.goto("http://localhost:1420/")
    page.wait_for_load_state("networkidle")
    time.sleep(2)
    
    page.screenshot(path="/tmp/01_home.png", full_page=True)
    
    try:
        settings_btn = page.locator('[data-testid="settings-btn"], [aria-label*="设置"], [aria-label*="Settings"]').first
        if settings_btn.is_visible(timeout=3000):
            settings_btn.click()
            time.sleep(1)
        else:
            page.goto("http://localhost:1420/settings")
            page.wait_for_load_state("networkidle")
            time.sleep(1)
    except:
        page.goto("http://localhost:1420/settings")
        page.wait_for_load_state("networkidle")
        time.sleep(1)
    
    page.screenshot(path="/tmp/02_settings.png", full_page=True)
    
    try:
        data_tab = page.locator('[data-testid="settings-sidebar"]').locator("text=数据").first
        if data_tab.is_visible(timeout=3000):
            data_tab.click()
            time.sleep(1)
            page.screenshot(path="/tmp/03_data_tab.png", full_page=True)
    except Exception as e:
        print(f"Could not find data tab: {e}")
    
    try:
        backup_item = page.locator('[data-testid="settings-sidebar"]').locator("text=备份中心").first
        if backup_item.is_visible(timeout=3000):
            backup_item.click()
            time.sleep(2)
            page.screenshot(path="/tmp/04_backup_center.png", full_page=True)
        else:
            print("Backup center menu item not found")
    except Exception as e:
        print(f"Could not find backup center: {e}")
    
    content_area = page.locator(".min-w-0.flex-1.overflow-y-auto")
    if content_area.is_visible(timeout=3000):
        content_html = content_area.inner_html()
        with open("/tmp/backup_content.html", "w", encoding="utf-8") as f:
            f.write(content_html[:5000])
    
    print("\n=== Console Errors ===")
    for err in console_errors:
        print(err)
    
    browser.close()
